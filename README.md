<p align="center">
  <img src="docs/media/icon.png" width="160" alt="MelCat icon: Mel the tabby cat, with a curious mouse peeking over her ear">
</p>

<h1 align="center">MelCat for Windows</h1>

<p align="center">Let Mel play with your mouse.</p>

<p align="center">
  <a href="https://github.com/irobson/melcat-win/releases/latest"><b>Download the latest MelCat.exe</b></a>
</p>

[![CI](https://github.com/irobson/melcat-win/actions/workflows/ci.yml/badge.svg)](https://github.com/irobson/melcat-win/actions/workflows/ci.yml)
![Windows 10 1607+ and 11](https://img.shields.io/badge/Windows-10%201607%2B%20%7C%2011-0078D6)
![Rust](https://img.shields.io/badge/Rust-native-orange)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A gentle, scheduled mouse jiggler for the Windows system tray, written in native Rust. The Windows sibling of [MelCat for macOS](https://github.com/irobson/melcat), with the same behavior.

On the days and hours you choose, after a stretch without any keyboard or mouse input, Mel walks the pointer a few points along a slow, slightly curved path around where you left it. Windows sees ordinary mouse movement, so the screen saver, the screen lock and sleep stay away. The moment you use the mouse or the keyboard, Mel lets go.

No installer, no account, no network access.

## Features

- **A schedule.** Pick the days and the hours, for example Monday to Friday, 09:00 to 18:00. Hours can run past midnight, and the same start and end mean all day. Or switch to **Always On**, or **Off**.
- **Light, random, slow.** Each move is a short stroll along a gentle curve, eased in and out, lasting up to three seconds, that never takes the pointer more than 2, 5 or 12 points (Tiny, Gentle, Playful) from where you left it (or, if you left it on a screen edge, from the spot 4 points inside it). No twitching, no fixed rhythm.
- **Only when you are away.** Mel waits for the idle time you choose (30 seconds to 4 minutes) without any input. While you work, nothing happens.
- **Lets go at once.** Move the mouse, press a key or a button, or turn the wheel during a stroll and it stops on the next frame.
- **Polite.** No moves while the screen is locked, the screen saver runs, the displays are off or you are switched to another session. Strolls stay 4 points away from every screen edge, so snapping, an auto-hidden taskbar and screen corners are never triggered.
- **Stays where you left it.** Strolls wander around the spot where you last put the pointer, so it never drifts away over a long afternoon.
- **Light.** One small executable, no dependencies, no background services.

## Install

1. Open the [latest release](https://github.com/irobson/melcat-win/releases/latest) and download **MelCat-x.y.z-windows-x64.exe**.
2. Put it anywhere, for example in a `MelCat` folder in your user folder, and double-click it.
3. Windows SmartScreen may say the app is unrecognized, because it is not code-signed yet. Click **More info**, then **Run anyway**.
4. Mel appears in the tray, near the clock, and the settings window opens the first time. If you do not see her, click the **^** arrow and drag her onto the taskbar.

To start MelCat with Windows, enable **Launch at Login** in its tray menu.

Requirements: Windows 10 version 1607 or later, or Windows 11, 64-bit (x64). ARM64 PCs with Windows 11 run it through x64 emulation; Windows 10 on ARM cannot run it. MelCat needs no special permission on Windows.

## Use it

Click Mel in the tray.

| Item | What it does |
|------|--------------|
| *Mel is playing · until 18:00* | What Mel is doing now (see below) |
| *Mon–Fri, 09:00–18:00* | The schedule, shown in On Schedule mode |
| On Schedule / Always On / Off | When Mel may move the pointer |
| Movement | Tiny (up to 2 pt), Gentle (up to 5 pt) or Playful (up to 12 pt) |
| Move After Idle | 30 seconds, 1, 2 or 4 minutes |
| Settings… | The settings window |
| Launch at Login | Start MelCat when you sign in |
| Open Log File | MelCat's log, useful when something does not work |
| Quit MelCat | Stops MelCat |

The first line says what Mel is doing, and the tray icon is dimmed whenever she is not playing:

| Status line | Meaning |
|-------------|---------|
| **Mel is playing · until 18:00** | Inside the schedule. Mel moves the pointer whenever you are idle. |
| **Mel is playing** | Always On, or On Schedule with every day selected all day. |
| **Mel is napping · back Mon 09:00** | Outside the schedule. Nothing moves until then. |
| **Mel is napping · no days selected** | On Schedule, but no day is selected. |
| **MelCat is off** | Off. Nothing moves. |

### Settings window

**Settings…** opens one page with everything: the mode, the days (one check box per day) and the start and end times, the movement and the idle time with a **Preview** button that takes Mel for one stroll right away (keep your hand off the mouse for a few seconds to watch it), and Launch at login. Changes apply at once.

### How Mel moves

1. Once a second, MelCat checks the mode, the schedule and how long Windows has gone without input.
2. When the idle time passes a threshold, Mel takes one stroll: a slightly curved path to a random spot within the movement radius of where you left the pointer (4 points inside the screen edge, if you left it on one), at walking pace, eased in and out, for 0.6 to 3 seconds.
3. The threshold is drawn at random between 75 % and 100 % of **Move After Idle**, less the slack of the one-second check, so moves never fall into a fixed rhythm and never come later than the time you picked. While you stay away, Mel keeps strolling about that often.
4. If the pointer is not where Mel left it, or a key, a mouse button or the wheel is used, the stroll ends on the spot.

Each step is an ordinary mouse move sent with `SendInput`, so apps and Windows' idle timers see it like any other movement. Pick an idle time shorter than your screen saver, screen-off and lock delays: Mel stays still once the screen is off or locked, and does not wake it.

### Settings file

Settings live in `%APPDATA%\MelCat\settings.json` and can be edited by hand; MelCat picks up an edit within a second.

```json
{ "mode": "Scheduled", "days": [2, 3, 4, 5, 6], "start_minutes": 540,
  "end_minutes": 1080, "intensity": "Gentle", "idle_interval_secs": 60 }
```

Modes are `Off`, `Scheduled` and `Always`; movements are `Tiny`, `Gentle` and `Playful`. Days are numbered from Sunday (1) to Saturday (7), and times are minutes since midnight. Idle times are rounded to whole seconds and kept between 5 seconds and one day.

## Safety: your mouse and keyboard always keep working

MelCat only ever *moves* the pointer. It installs no hook and never holds back, changes or swallows a click, a key or a movement of yours: it only notices keys, buttons and the wheel through Raw Input, which leaves them untouched. Whatever goes wrong in MelCat, your input keeps working.

- A stroll stops on the next frame when you move the mouse, press a key or a button, or turn the wheel.
- Nothing moves while the session is locked, the screen saver runs, the displays are off, or you are switched to another session.
- Only one MelCat can run at a time in your Windows session; a second one exits at once.
- Windows does not let a normal program move the pointer over a window of a program running as administrator. MelCat then stops the stroll and tries again later; nothing else happens.

To stop MelCat, choose **Quit MelCat** in the tray menu, or end **MelCat** in Task Manager (Ctrl+Shift+Esc). The log is `%LOCALAPPDATA%\MelCat\melcat.log`.

## Troubleshooting

**The screen still locks, or the computer still sleeps.**
Choose a **Move After Idle** shorter than the screen saver, screen-off and lock delays in Windows Settings, and check that the status line says *Mel is playing*. MelCat stays still once the screen is off or locked.

**Mel does not move while a program running as administrator is in front.**
Windows blocks mouse movement sent by a normal program to such windows. Run MelCat as administrator too, or leave the pointer over another window.

**I want to see what MelCat is doing.**
Choose **Open Log File** in the tray menu. Each stroll is logged when it starts, finishes or stops, with the reason.

## Build from source

On Windows with the Rust toolchain (MSVC):

```powershell
cargo test --workspace
cargo build --release -p melcat
```

The executable is `target\release\MelCat.exe`. You can also cross-compile from macOS or Linux with the `x86_64-pc-windows-gnu` target and mingw-w64.

Every push to main, and every pull request, runs the unit tests (the core logic shares its test cases with the macOS app) and an end-to-end smoke test that drives MelCat.exe on a Windows runner with simulated input. Pushing a tag such as `v0.1.0` that matches the version in `Cargo.toml` builds, tests and smoke-tests MelCat.exe, then publishes it on the Releases page.

How it works inside: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

MIT. See [LICENSE](LICENSE).
