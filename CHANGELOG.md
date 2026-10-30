# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-10-30

### Added
- Tray mouse jiggler for Windows 10 (1607 and later) and 11, the Windows port of MelCat for macOS: after a chosen idle time, Mel moves the pointer a few points along a slow, slightly curved, random path, and lets go when you use the mouse or the keyboard.
- Modes: On Schedule, Always On and Off. A schedule of days and a start and end time; hours can run past midnight, and equal times mean all day.
- Movement: Tiny (2 pt), Gentle (5 pt) or Playful (12 pt), and an idle time of 30 seconds, 1, 2 or 4 minutes, randomized between 75 % and 100 % of the choice, so moves never come later than chosen.
- Strolls wander around the spot where you left the pointer, stay 4 points away from every screen edge, and stop on the next frame when you move the mouse, press a key or a button, or turn the wheel.
- No moves while the session is locked, the screen saver runs, this session's display is off, or another session is active.
- Tray icon (white or black for the taskbar theme, dimmed while Mel is not playing) with the status line ("Mel is playing · until 18:00", "Mel is napping · back Mon 09:00"), the schedule, the modes, Movement, Move After Idle, Settings…, Launch at Login, Open Log File and Quit MelCat.
- Settings window with the mode, days, hours, movement, idle time, a Preview of the movement and Launch at login; it opens on first launch.
- Settings in `%APPDATA%\MelCat\settings.json`, reloaded when edited by hand; log in `%LOCALAPPDATA%\MelCat\melcat.log`.
- Core logic shared in behavior and test cases with the macOS app, unit tests for every pure helper, and an end-to-end smoke test on a Windows runner for every push to main, every pull request and every release.

[Unreleased]: https://github.com/irobson/melcat-win/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/irobson/melcat-win/releases/tag/v0.1.0
