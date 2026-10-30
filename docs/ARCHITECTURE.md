# Architecture

MelCat for Windows is a port of [MelCat for macOS](https://github.com/irobson/melcat). It does one thing: on the days and hours you choose, after a stretch without input, it walks the pointer a few points along a slow, curved path and lets go the moment you use the mouse or keyboard.

## Crates

| Crate | What it holds |
|-------|---------------|
| `crates/core` (`melcat-core`) | Pure logic, no Win32, builds and tests on any OS: weekdays, wall-clock times, the schedule (including daylight saving time), the move-or-wait decision and the stroll planner. A 1:1 port of the macOS app's `MelCatCore`, with the same test cases. |
| `crates/app` (`MelCat.exe`) | Everything Windows: the tray, the settings window, the pointer, raw input and the session. Builds only for Windows. |

## Threads

One: the UI thread, like the macOS app's main actor. A 1-second timer runs the decision; a 16 ms timer plays a stroll back. There is no hook and no worker thread.

## Safety

MelCat only ever moves the pointer. It installs no hook that could hold or swallow input, so whatever goes wrong, the mouse and keyboard keep working.
