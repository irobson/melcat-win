# Architecture

MelCat for Windows is a port of [MelCat for macOS](https://github.com/irobson/melcat). It does one thing: on the days and hours you choose, after a stretch without input, it walks the pointer a few points along a slow, curved path, and lets go the moment you use the mouse or keyboard. This document explains the parts that are not obvious from the code: the crates, the one thread, coordinates, how a person is noticed, the safety decisions and what the smoke test proves.

Every app module's top doc comment is its contract: the behavior it must have, which the module's unit tests and the smoke test check.

## Crates

| Crate | What it holds |
|-------|---------------|
| `crates/core` (`melcat-core`) | Pure logic, no Win32, builds and tests on any OS: weekdays and their descriptions, wall-clock times, the schedule (including daylight saving time), the move-or-wait decision, the idle threshold and the stroll planner. A 1:1 port of the macOS app's `MelCatCore`, with the same test cases, so both apps decide the same way. |
| `crates/app` (`MelCat.exe`) | Everything Windows: the tray, the settings window, the pointer, raw input, the session and launch at login. Its modules are only compiled for Windows; elsewhere `main` prints a message and exits. |

The schedule needs a time zone to turn instants into wall-clock times and back. `melcat_core::Calendar` abstracts it; `ChronoCalendar` implements it for any `chrono` zone. The app uses `chrono::Local` (Windows' own zone rules), and the tests use `chrono_tz` zones, including a minute-by-minute check of every 2026 daylight saving change in five zones.

## One thread

Everything runs on the UI thread, like the macOS app's main actor. There is no worker thread and no hook.

| Window | Kind | Receives |
|--------|------|----------|
| `MelCat.Main` | hidden top-level | tray callbacks, `TaskbarCreated`, `WM_SETTINGCHANGE`, `WM_DISPLAYCHANGE`, `WM_WTSSESSION_CHANGE`, `WM_POWERBROADCAST`, the end of the session |
| `MelCat.Msg` | message-only (`HWND_MESSAGE`) | the 1-second tick, the 16 ms stroll timer, `WM_INPUT`, the settings window's notifications |
| `MelCat.Settings` | top-level, shown on demand | the settings page |

`MelCat.Main` is top-level so it gets broadcasts, which also means other processes can post to it. It reads no pointer from a message's parameters except the `POWERBROADCAST_SETTING` of a power notification, which it checks before reading. Plain values (a lock or unlock event, a tray event, the end of the session) are taken as given, so another process can at most make MelCat stay still, open its menu or quit. Nothing MelCat posts carries a pointer.

The controller (`app.rs`) lives in a thread-local `RefCell`. Window procedures queue events; the queue is drained by whichever call is not nested inside the controller, so it is never re-entered. Calls that run a modal loop (the tray menu, `ShellExecuteW`) run outside the controller, so the ticks and strolls continue while the menu is open.

## The tick and the stroll

`jiggle::jiggler` is a port of the macOS `Jiggler.swift`.

- **Tick** (once a second, and after any settings change): the activity (`Activity::at`, cached per minute, mode and schedule, since nothing can change inside a minute) gives the status: off, napping until, or active until. When active and no stroll runs, `verdict` decides from the session state (asked only then), the mouse buttons and the idle time: the smaller of `GetLastInputInfo` and the time since MelCat's last stroll. The threshold is 75–100 % of the chosen interval, minus 1.2 s for the tick's own period and slack.
- **Stroll**: `plan_walk` runs in the logical points of the monitor under the anchor (the spot where the user left the pointer), inside that monitor inset by 4 pt. The result is converted to physical pixels.
- **Steps**: a 16 ms timer, which Windows rounds to its tick. Each step picks its point from the time elapsed since the stroll began, so a late timer skips points instead of stretching the stroll.

## Coordinates

The process is per-monitor DPI aware (v2 from the manifest on Windows 10 1703 and later, per-monitor v1 on 1607), so `GetCursorPos`, `SendInput` and monitor rectangles all speak physical pixels of the virtual desktop. Distances from the core (the 2, 5 and 12 pt radii, the 4 pt margin, the walking speeds) are logical points, converted with the monitor's effective DPI / 96 (`geom::MonitorInfo`).

A move is one `SendInput` with `MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK`, `time` 0 and `dwExtraInfo` "MCAT", at `((x - left) * 65536 + 32768) / width` over `SM_XVIRTUALSCREEN`/`SM_CXVIRTUALSCREEN` (and Y): the center of the target pixel, on any layout, negative origins included. A unit test round-trips every pixel of ten layouts.

## Noticing a person

During a stroll, before each step:

1. **The pointer moved.** If it is more than 1 px from both of the last two pixels MelCat posted (two, to absorb one frame of lag), a hand moved it: stop and forget the anchor. A held mouse button counts the same, as on macOS.
2. **A key, a button press or the wheel.** Raw input (`RegisterRawInputDevices` with `RIDEV_INPUTSINK` on `MelCat.Msg`) reports key presses, modifier changes, button presses and the wheel. MelCat never sends any of those, so every one is a person, or another program acting for one. Releases alone are not, as on macOS: releasing Enter after pressing Preview does not cancel the preview. Stop, keep the anchor.
3. **Moves that never land.** Windows' UIPI drops injected input aimed at a window of a higher integrity level, and `SendInput` still reports success. If the pointer is still on the stroll's start pixel after two other pixels were posted, the stroll stops as "the pointer could not be moved" and MelCat waits a whole threshold before trying again.

Plain movement is never classified: MelCat never has to tell its own moves apart from anyone else's.

## When MelCat stays still

`jiggle::session` reports the session as away when it is locked (`WTS_SESSION_LOCK`, initial state from `WTSSessionInfoEx`), not the active one (`WTSConnectState`), when the screen saver runs (`SPI_GETSCREENSAVERRUNNING`), or when this session's display is off (`GUID_SESSION_DISPLAY_STATUS`, not the console's display, so Remote Desktop and virtual machine sessions keep working when the physical monitor sleeps). A move then would only wake things up. Session changes, sleep, display changes and the end of the session also stop a stroll in progress.

## Safety decisions

The macOS sibling TedCat once used an event tap that held clicks; when its permission was revoked while it ran, every click died. The rule since: a broken app must degrade to "input keeps working".

- MelCat only moves the pointer. It installs no hook (`WH_MOUSE_LL`, `WH_KEYBOARD_LL`) and never holds, changes or swallows input. Raw input only observes; `WM_INPUT` still goes to `DefWindowProcW`.
- If MelCat hangs or is killed mid-stroll, nothing is held: the next real input simply wins. The smoke test kills it mid-stroll and then clicks.
- One instance only: the named mutex `Local\MelCat.SingleInstance`; a second MelCat exits quietly with code 0.
- The end of the session (`WM_QUERYENDSESSION`, `WM_ENDSESSION`) stops any stroll at once and never vetoes.
- Messages to the user (balloons) are whole sentences; `messages::sentence` is the one place that adds a final period, so none ends in "..".

## Settings and the tray

`%APPDATA%\MelCat\settings.json`, read leniently: a byte order mark (UTF-8 or UTF-16), unknown keys and a bad value are tolerated, each bad value falling back to its default alone. MelCat checks the file's time stamp and size every tick and reloads an edit made by hand; its own saves do not count as edits. Saves go through a temporary file. Launch at login is the `MelCat` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, and an entry disabled in Task Manager (`StartupApproved\Run`) reads as off.

The tray icon is a 64 px PNG rendered from the macOS menu bar glyph: white for a dark taskbar, black for a light one, dimmed while Mel is not playing. Windows does not tint tray icons, so MelCat reads `SystemUsesLightTheme` and swaps on `WM_SETTINGCHANGE`; it adds the icon again on `TaskbarCreated` and records a tooltip or icon as current only once the shell accepted it. The `.exe` icon is a multi-size ICO with PNG images from 16 to 256 px. Both come from the macOS masters through `scripts/icons/`.

## What the smoke test proves

`scripts/smoke-test.ps1` runs on GitHub's `windows-latest` runner (Windows Server 2025, an interactive session, 1024 x 768 at 96 DPI) against the built `MelCat.exe`, on every push to main and every pull request, and before every release. It drives MelCat with `SendInput` from PowerShell, which reaches apps like real input.

| Step | Proves |
|------|--------|
| exe | 64-bit GUI executable, version info and manifest version match `Cargo.toml`, icon, PerMonitorV2, common controls v6 |
| start | starts, adds its tray icon, and a second MelCat exits at once |
| stroll | moves by itself after the idle time, within the radius, without drift, and again every 2.5 to 5 s while idle (never later than the idle time); its moves reset the session's idle time |
| edges | from near and at the top-left corner: once out of the 4 pt edge band, never within 2 px of an edge; only moves inwards; stays within the radius of the spot moved inside the band |
| takeover | a moved pointer stops the stroll within 100 ms (a few frames), stays where it was put for at least 1.5 s, and the next stroll wanders around the new spot |
| input, key-release | a key press or the wheel stops a stroll; a lone key release does not |
| held-button | a held button stops a stroll as a hand on the mouse |
| off | Off mode never moves, and MelCat keeps running |
| kill-mid-stroll | killing MelCat mid-stroll leaves no button or key held; a click still works |
| first-launch | the settings window opens on first launch; Enter on Preview plays a whole preview; a clean quit removes the tray icon and exits 0 |
| settings-bom, settings-reload, save-failure | settings files with a byte order mark, edits while running, and a failed save with a readable notice |
| settings-preview-label, settings-repaint, settings-short-time | the settings page's Preview row, its repaint after a text size change, and times in the short format with one final period |

Not provable on the runner, and worth a check on a real PC: multiple monitors and scales above 100 %, a real lock, screen saver, display off, sleep and Remote Desktop, hardware raw input, an elevated window under the pointer with UAC on, Explorer restarts, the tray menu itself, and whether chat apps keep showing you as available.
