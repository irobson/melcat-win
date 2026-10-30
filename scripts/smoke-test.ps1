<#
.SYNOPSIS
    End-to-end smoke test of MelCat.exe on a real (or CI) Windows desktop.

.DESCRIPTION
    Steps (each reports PASSED, FAILED or SKIPPED with its evidence):

      exe           MelCat.exe is a 64-bit GUI executable with MelCat's
                    version info (ProductName MelCat; FileVersion,
                    ProductVersion and both version numbers are the workspace
                    version), an icon (icon group 1, RT_GROUP_ICON, with every
                    image it names), and a manifest whose assemblyIdentity
                    version is the workspace version too, asking for
                    PerMonitorV2 DPI awareness and common controls v6.
      start         With {mode Always, Gentle, idle 5 s} MelCat starts, logs
                    'MelCat <version> starting' and 'tray icon added', and
                    Shell_NotifyIconGetRect finds the tray icon of its
                    MelCat.Main window (uID 1). A second MelCat.exe exits at
                    once with code 0 and logs 'another MelCat is already
                    running; exiting'.
      stroll        The pointer is parked mid-screen with one injected move,
                    then left alone. Within 15 s it moves by itself ('stroll
                    started', then 'stroll finished'); every position sampled
                    (GetCursorPos every millisecond or so) stays within the
                    Gentle radius of the parking spot (5 pt, +1 px for
                    rounding); more strolls follow while idle, 2.5 to 5 s
                    apart (5.05 s allowed for the log time stamps), 4 in all,
                    without drifting from the spot. MelCat's own moves reset
                    the session's idle time: the script sends no input after
                    parking, yet GetLastInputInfo (read every 10 ms) shows
                    input during every stroll, and the idle time never exceeds
                    the interval plus 2.5 s.
      edges         With Playful (12 pt), parked 2 px from the top-left corner
                    and then exactly at (0,0): over several strolls no sampled
                    position comes closer than 2 px to a screen edge once it
                    has left the 4 pt edge band, the pointer only moves
                    inwards from where it was parked, and it stays within the
                    radius of the parking spot moved inside the band.
      takeover      A move injected 200 px away during a stroll: MelCat stops
                    within one frame ('stroll stopped: pointer moved'), the
                    pointer stays exactly where the move put it for at least
                    1.5 s, and the next stroll wanders around the new spot.
      input         A key press and release (F13) injected during one stroll
                    and a wheel notch during another each stop the stroll
                    ('stroll stopped: input'), and the pointer is let go.
      key-release   A lone F13 release (KEYEVENTF_KEYUP, no press before it)
                    injected during a stroll does not stop it: the stroll
                    ends with 'stroll finished at'. As on macOS, only key
                    presses, modifier changes, button presses and the wheel
                    are a person.
      off           With mode Off, no stroll starts and the pointer stays put
                    over 12 s of idleness, the log says 'status: off', and
                    MelCat is still running afterwards.
      kill-mid-stroll
                    MelCat killed (Stop-Process -Force) during a stroll leaves
                    nothing behind: the pointer stays still, an injected move
                    lands exactly where sent, no mouse button or modifier is
                    held, and one click on a WinForms test button hosted by the
                    script counts once.
      held-button   The left button pressed where the pointer is (over a
                    WinForms test button hosted by the script, without
                    moving it) and held 100 ms during a stroll is a hand on
                    the mouse, as on macOS: 'stroll stopped: pointer moved',
                    and the pointer is let go.
      first-launch  Without settings.json MelCat opens its settings window (a
                    visible top-level MelCat.Settings window of its process; a
                    screenshot is saved). A click on Preview gives it the
                    focus and plays a preview; then Enter, held 100 ms, plays
                    another, which ends with 'stroll finished at' (the
                    release of Enter is not input). WM_CLOSE closes the
                    window, settings.json then exists, and MelCat keeps
                    running until asked to quit. Then a clean quit: WM_CLOSE
                    to MelCat.Main ends MelCat within 5 s with exit code 0,
                    it logs 'MelCat stopped', and the shell no longer knows
                    its tray icon (NIM_DELETE ran).
      settings-bom  settings.json written with a byte order mark (UTF-8, as
                    Notepad and Set-Content -Encoding UTF8 write it; UTF-16 LE,
                    as Out-File and '>' write it; UTF-16 BE) holding mode Off,
                    Sunday and Saturday, Playful and 5 s: MelCat loads every
                    value (its 'settings:' line) and its first status is 'off'.
                    A settings.json that is not JSON leaves a warning in the
                    log, and MelCat runs with the defaults.
      settings-reload
                    With MelCat running in mode Always, settings.json edited
                    to mode Off applies within 3 s ('settings reloaded',
                    'status: off'), and no stroll starts and the pointer stays
                    put for 8 s after. Then MelCat's own save (posting
                    WM_APP_SETTINGS_CHANGED, 0x8002, to its MelCat.Msg
                    window) keeps mode Off in the file and is not taken for
                    an edit (no 'settings reloaded' in the 2.5 s after it).
      save-failure  With a folder in the way of settings.json.tmp, a save
                    (WM_APP_SETTINGS_CHANGED) fails and the balloon (its
                    'notice:' log line) reads as whole sentences, such as
                    'Access is denied (error 5). The change applies until
                    MelCat quits.', not 'Access is denied. (os error 5). ...'.
                    MelCat keeps running.
      settings-preview-label
                    The settings window's Preview row is the macOS one: a
                    Static 'Try the movement now' left of the Preview button
                    (control 135), on its row, with the note (control 136)
                    under the label.
      settings-repaint
                    After a larger text size (the message font 1.5 times as
                    tall, set for the session only with SPI_SETNONCLIENTMETRICS
                    and put back afterwards, then the WM_SETTINGCHANGE Windows
                    sends for it; the window must grow), every label of the
                    settings window on the screen shows exactly the pixels a
                    full repaint (RedrawWindow with RDW_ALLCHILDREN |
                    RDW_UPDATENOW) gives: none keeps pixels drawn in the old
                    font. The window is made topmost first. (WM_DPICHANGED
                    takes the same path, but Windows refuses to send it from
                    another process. A guard: on this Windows the window's own
                    resize already repaints every control.)
      settings-short-time
                    With the long time format 'h:mm:ss tt', the short one
                    'HH:mm tt' and the markers 'a.m.'/'p.m.' (SetLocaleInfoW,
                    put back afterwards), twelve Down presses on the To
                    picker's hour (posted to it after WM_NEXTDLGCTL) make the
                    schedule end at 06:00 (settings.json end_minutes 360), and
                    the hint reads exactly 'Ends the next morning at 06:00
                    a.m.': the pickers' short format, one final period.
                    (On this Windows a null format with TIME_NOSECONDS already
                    gives the short format; the time part guards it.)
                    SetLocaleInfoW changes the user's saved formats: if the
                    script is killed during this step on a machine of yours,
                    put the time formats back in Settings > Time & language.

    settings-preview-label, settings-repaint and settings-short-time each
    start MelCat without settings.json (a first launch, so the settings window
    opens), with the pointer parked away from it.

    Each step from 'stroll' on starts its own MelCat: it asks any MelCat still
    running to quit (WM_CLOSE to MelCat.Main; the step fails unless it quits
    within 5 s with exit code 0), writes
    %APPDATA%\MelCat\settings.json (idle interval 5 s, the shortest), parks
    the pointer with one injected move, starts the pointer sampler and then
    MelCat.exe. (first-launch removes settings.json instead, and
    settings-bom writes its own.) Input is injected with SendInput and
    dwExtraInfo 0, like a person's.

    MelCat is always stopped at the end, its log printed and copied, and the
    script exits non-zero if any step FAILED. Artifacts (log, desktop
    screenshots, summary.json) go to -ArtifactsDir.

    Runs on Windows PowerShell 5.1 (STA; it re-launches itself with -STA if needed).

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-test.ps1 -Exe target\release\MelCat.exe
#>
[CmdletBinding()]
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\MelCat.exe'),
    [string]$ArtifactsDir = (Join-Path (Get-Location) 'smoke-artifacts')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

if ([System.Threading.Thread]::CurrentThread.GetApartmentState() -ne [System.Threading.ApartmentState]::STA) {
    Write-Host 'Not in a single-threaded apartment; re-launching with -STA.'
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -STA -File $PSCommandPath -Exe $Exe -ArtifactsDir $ArtifactsDir
    exit $LASTEXITCODE
}

$Exe = [System.IO.Path]::GetFullPath($Exe)
$ArtifactsDir = [System.IO.Path]::GetFullPath($ArtifactsDir)
New-Item -ItemType Directory -Force -Path $ArtifactsDir | Out-Null

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# ---------------------------------------------------------------------------
# Native helpers (C# 5: compiled by Windows PowerShell 5.1's Add-Type).
# Steps that need more helpers add their own Add-Type block, each in its own
# namespace (MelCatSmoke.<Step>), so blocks never collide.

$helpers = @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace MelCatSmoke
{
    public static class Native
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Sequential)]
        public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Explicit)]
        public struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
        [StructLayout(LayoutKind.Sequential)]
        public struct INPUT { public uint type; public InputUnion u; }
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
        [StructLayout(LayoutKind.Sequential)]
        public struct POINT { public int X; public int Y; }
        [StructLayout(LayoutKind.Sequential)]
        public struct NOTIFYICONIDENTIFIER { public uint cbSize; public IntPtr hWnd; public uint uID; public Guid guidItem; }

        [DllImport("user32.dll", SetLastError = true)]
        public static extern uint SendInput(uint count, [In, MarshalAs(UnmanagedType.LPArray)] INPUT[] inputs, int size);
        [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
        [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
        [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
        public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lparam);
        [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc proc, IntPtr lparam);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hwnd, StringBuilder name, int max);
        [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("shell32.dll")] public static extern int Shell_NotifyIconGetRect(ref NOTIFYICONIDENTIFIER identifier, out RECT rect);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        public static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
        [DllImport("kernel32.dll")] public static extern bool FreeLibrary(IntPtr module);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
        [DllImport("kernel32.dll")] public static extern IntPtr LoadResource(IntPtr module, IntPtr info);
        [DllImport("kernel32.dll")] public static extern IntPtr LockResource(IntPtr data);
        [DllImport("kernel32.dll")] public static extern uint SizeofResource(IntPtr module, IntPtr info);
    }

    public static class Win
    {
        public static string ClassOf(IntPtr hwnd)
        {
            StringBuilder name = new StringBuilder(256);
            Native.GetClassName(hwnd, name, name.Capacity);
            return name.ToString();
        }

        /// Handle of the first top-level window of class `cls` (visible or not).
        public static IntPtr Find(string cls)
        {
            IntPtr result = IntPtr.Zero;
            Native.EnumWindows(delegate(IntPtr hwnd, IntPtr unused)
            {
                if (ClassOf(hwnd) == cls) { result = hwnd; return false; }
                return true;
            }, IntPtr.Zero);
            return result;
        }

        /// Where the shell shows tray icon `id` of `owner`, or why it cannot say.
        public static string TrayIcon(IntPtr owner, uint id)
        {
            Native.NOTIFYICONIDENTIFIER identifier = new Native.NOTIFYICONIDENTIFIER();
            identifier.cbSize = (uint)Marshal.SizeOf(typeof(Native.NOTIFYICONIDENTIFIER));
            identifier.hWnd = owner;
            identifier.uID = id;
            Native.RECT r;
            int hr = Native.Shell_NotifyIconGetRect(ref identifier, out r);
            if (hr != 0) return "error 0x" + hr.ToString("X8");
            return "rect=" + r.Left + "," + r.Top + "," + r.Right + "," + r.Bottom;
        }

        /// The RT_MANIFEST resource 1 of an executable, as text.
        public static string Manifest(string path)
        {
            IntPtr module = Native.LoadLibraryEx(path, IntPtr.Zero, 0x00000002 /* LOAD_LIBRARY_AS_DATAFILE */);
            if (module == IntPtr.Zero) return "";
            try
            {
                IntPtr info = Native.FindResource(module, new IntPtr(1), new IntPtr(24));
                if (info == IntPtr.Zero) return "";
                IntPtr data = Native.LockResource(Native.LoadResource(module, info));
                uint size = Native.SizeofResource(module, info);
                byte[] bytes = new byte[size];
                Marshal.Copy(data, bytes, 0, (int)size);
                return Encoding.UTF8.GetString(bytes);
            }
            finally { Native.FreeLibrary(module); }
        }

        /// Saves the whole virtual desktop to a PNG; returns an error text or "".
        public static string Shot(string path)
        {
            try
            {
                int left = Native.GetSystemMetrics(76), top = Native.GetSystemMetrics(77);
                int width = Native.GetSystemMetrics(78), height = Native.GetSystemMetrics(79);
                using (Bitmap bitmap = new Bitmap(Math.Max(1, width), Math.Max(1, height)))
                using (Graphics g = Graphics.FromImage(bitmap))
                {
                    g.CopyFromScreen(left, top, 0, 0, bitmap.Size);
                    bitmap.Save(path, ImageFormat.Png);
                }
                return "";
            }
            catch (Exception e) { return e.Message; }
        }

        /// Reads a file another process may be writing.
        public static string ReadShared(string path)
        {
            using (FileStream stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
            using (StreamReader reader = new StreamReader(stream, Encoding.UTF8))
            {
                return reader.ReadToEnd();
            }
        }
    }
}
'@
Add-Type -TypeDefinition $helpers -ReferencedAssemblies System.Windows.Forms, System.Drawing -Language CSharp
# Physical pixels, like MelCat (per-monitor DPI aware).
[void][MelCatSmoke.Native]::SetProcessDPIAware()

# ---------------------------------------------------------------------------
# Plumbing

$LogPath = Join-Path $env:LOCALAPPDATA 'MelCat\melcat.log'
$SettingsPath = Join-Path $env:APPDATA 'MelCat\settings.json'

$script:Results = New-Object System.Collections.Generic.List[object]
$script:Notes = New-Object System.Collections.Generic.List[string]
$script:Mel = $null
$script:ShotCount = 0

function Note([string]$Text) {
    $script:Notes.Add($Text)
    Write-Host "    $Text"
}

function Assert([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Skip([string]$Why) { throw "SKIP: $Why" }

function Save-Desktop([string]$Name) {
    $script:ShotCount++
    $path = Join-Path $ArtifactsDir ('{0:D2}-{1}.png' -f $script:ShotCount, $Name)
    $err = [MelCatSmoke.Win]::Shot($path)
    if ($err) { Write-Host "    (desktop screenshot failed: $err)" } else { Write-Host "    desktop screenshot: $path" }
}

function Invoke-Step([string]$Name, [scriptblock]$Body) {
    Write-Host ''
    Write-Host "=== $Name ==="
    $script:Notes = New-Object System.Collections.Generic.List[string]
    $result = 'passed'
    $why = $null
    try {
        & $Body | Out-Host
    } catch {
        $message = $_.Exception.Message
        if ($message.StartsWith('SKIP: ')) {
            $result = 'skipped'
            $why = $message.Substring(6)
        } else {
            $result = 'failed'
            $why = '{0} (script line {1})' -f $message, $_.InvocationInfo.ScriptLineNumber
        }
    }
    $evidence = @($script:Notes)
    if ($why) { $evidence = @($why) + $evidence }
    $script:Results.Add([pscustomobject]@{ Step = $Name; Result = $result; Evidence = ($evidence -join ' | ') })
    Write-Host ('>>> {0}: {1}' -f $Name, $result.ToUpper())
    if ($why) { Write-Host "    $why" }
    if ($result -eq 'failed') { Save-Desktop "failed-$Name" }
}

function Read-MelLog {
    if (Test-Path -LiteralPath $LogPath) { return [MelCatSmoke.Win]::ReadShared($LogPath) }
    return ''
}

function Get-LogMark { return (Read-MelLog).Length }

function Get-LogSince([int]$Mark) {
    $text = Read-MelLog
    if ($text.Length -le $Mark) { return '' }
    return $text.Substring($Mark)
}

# Waits for a regex in the log written after $Mark; returns the matching line or $null.
function Wait-Log([string]$Pattern, [int]$Mark = 0, [int]$TimeoutMs = 10000) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    do {
        foreach ($line in ((Get-LogSince $Mark) -split "`r?`n")) {
            if ($line -match $Pattern) { return $line.Trim() }
        }
        Start-Sleep -Milliseconds 100
    } while ($watch.ElapsedMilliseconds -lt $TimeoutMs)
    return $null
}

function Wait-Until([scriptblock]$Condition, [int]$TimeoutMs = 5000) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    do {
        if (& $Condition) { return $true }
        Start-Sleep -Milliseconds 50
    } while ($watch.ElapsedMilliseconds -lt $TimeoutMs)
    return [bool](& $Condition)
}

function Start-MelCat {
    $process = Start-Process -FilePath $Exe -PassThru
    $null = $process.Handle  # keeps ExitCode readable after the process ends
    return $process
}

function Stop-MelCat {
    Get-Process -Name MelCat -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}

$script:Invariant = [System.Globalization.CultureInfo]::InvariantCulture
$script:Version = [regex]::Match((Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '..\Cargo.toml')), '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
$script:IdleSeconds = 5
$script:Primary = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$script:Dpi = 96.0
try {
    $graphics = [System.Drawing.Graphics]::FromHwnd([IntPtr]::Zero)
    $script:Dpi = [double]$graphics.DpiX
    $graphics.Dispose()
} catch { }
# MelCat's distances are logical points; on this screen, in pixels.
$script:Scale = $script:Dpi / 96.0
$script:GentlePx = 5.0 * $script:Scale
$script:PlayfulPx = 12.0 * $script:Scale
$script:EdgeBandPx = 4.0 * $script:Scale

# Settings for the next MelCat start (the shape in crates/app/src/settings.rs).
function Write-MelSettings([string]$Mode = 'Always', [string]$Intensity = 'Gentle') {
    New-Item -ItemType Directory -Force -Path (Split-Path $SettingsPath) | Out-Null
    $json = '{{"mode":"{0}","days":[2,3,4,5,6],"start_minutes":540,"end_minutes":1080,"intensity":"{1}","idle_interval_secs":{2}}}' -f $Mode, $Intensity, $script:IdleSeconds
    # UTF-8 without a byte order mark.
    [System.IO.File]::WriteAllText($SettingsPath, $json)
}

# Every log line after $Mark that matches $Pattern, trimmed.
function Get-LogLines([string]$Pattern, [int]$Mark = 0) {
    return @(((Get-LogSince $Mark) -split "`r?`n") | Where-Object { $_ -match $Pattern } | ForEach-Object { $_.Trim() })
}

# Waits until $Count lines after $Mark match $Pattern; returns the lines found.
function Wait-LogCount([string]$Pattern, [int]$Mark, [int]$Count, [int]$TimeoutMs) {
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    do {
        $lines = @(Get-LogLines $Pattern $Mark)
        if ($lines.Count -ge $Count) { return $lines }
        Start-Sleep -Milliseconds 100
    } while ($watch.ElapsedMilliseconds -lt $TimeoutMs)
    return @(Get-LogLines $Pattern $Mark)
}

# The text of a log line without its time stamp and level.
function Get-LogText([string]$Line) {
    return ($Line -replace '^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d{3} \w+\s+', '')
}

# MelCat's stroll lines between two log marks (the end defaults to now),
# oldest first: Time, Kind (started, finished or stopped), Detail (what
# follows), and for 'started' the steps and both ends, for 'finished' the end.
function Get-StrollEvents([int]$Mark, [int]$End = -1) {
    $text = Read-MelLog
    if ($End -lt 0 -or $End -gt $text.Length) { $End = $text.Length }
    $events = New-Object System.Collections.Generic.List[object]
    if ($Mark -ge $End) { return $events.ToArray() }
    foreach ($line in ($text.Substring($Mark, $End - $Mark) -split "`r?`n")) {
        $m = [regex]::Match($line, '^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d{3}) \w+\s+stroll (started|finished|stopped)(?:: | at )?(.*)$')
        if (-not $m.Success) { continue }
        $e = [pscustomobject]@{
            Time = [datetime]::ParseExact($m.Groups[1].Value, 'yyyy-MM-dd HH:mm:ss.fff', $script:Invariant)
            Kind = $m.Groups[2].Value
            Detail = $m.Groups[3].Value.Trim()
            Steps = 0; FromX = 0; FromY = 0; ToX = 0; ToY = 0
        }
        $p = [regex]::Match($e.Detail, '^(\d+) steps from (-?\d+),(-?\d+) to (-?\d+),(-?\d+)$')
        if ($p.Success) {
            $e.Steps = [int]$p.Groups[1].Value
            $e.FromX = [int]$p.Groups[2].Value; $e.FromY = [int]$p.Groups[3].Value
            $e.ToX = [int]$p.Groups[4].Value; $e.ToY = [int]$p.Groups[5].Value
        }
        $q = [regex]::Match($e.Detail, '^(-?\d+),(-?\d+)$')
        if ($q.Success) { $e.ToX = [int]$q.Groups[1].Value; $e.ToY = [int]$q.Groups[2].Value }
        $events.Add($e)
    }
    return $events.ToArray()
}

# "started 84 steps from ...; finished 512,380; ..." for evidence.
function Format-StrollEvents([object[]]$Events) {
    if ($Events.Count -eq 0) { return '(no stroll lines)' }
    return (($Events | ForEach-Object { '{0} {1} {2}' -f $_.Time.ToString('HH:mm:ss.fff'), $_.Kind, $_.Detail }) -join '; ')
}

# "started finished started ..." of a list of stroll events.
function Get-StrollKinds([object[]]$Events) {
    return (($Events | ForEach-Object { $_.Kind }) -join ' ')
}

function Get-Distance([double]$X1, [double]$Y1, [double]$X2, [double]$Y2) {
    return [Math]::Sqrt(($X1 - $X2) * ($X1 - $X2) + ($Y1 - $Y2) * ($Y1 - $Y2))
}

# Pixels from (X, Y) to the nearest edge of the primary screen (0 on an edge pixel).
function Get-EdgeDistance([int]$X, [int]$Y) {
    $p = $script:Primary
    return [Math]::Min([Math]::Min($X - $p.Left, $p.Right - 1 - $X), [Math]::Min($Y - $p.Top, $p.Bottom - 1 - $Y))
}

function Get-ScreenCenter {
    $p = $script:Primary
    return @([int][Math]::Floor($p.Left + $p.Width / 2), [int][Math]::Floor($p.Top + $p.Height / 2))
}

# ---------------------------------------------------------------------------
# Steps

$exitCode = 0
try {
    # -----------------------------------------------------------------------
    # exe: the icon resource (MelCatSmoke.AppExe)

    $appExeHelpers = @'
using System;
using System.Runtime.InteropServices;
using System.Text;

namespace MelCatSmoke.AppExe
{
    public static class Res
    {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
        [DllImport("kernel32.dll")] static extern bool FreeLibrary(IntPtr module);
        [DllImport("kernel32.dll")] static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
        [DllImport("kernel32.dll")] static extern IntPtr LoadResource(IntPtr module, IntPtr info);
        [DllImport("kernel32.dll")] static extern IntPtr LockResource(IntPtr data);
        [DllImport("kernel32.dll")] static extern uint SizeofResource(IntPtr module, IntPtr info);

        /// The images of icon group `id` (RT_GROUP_ICON) of an executable,
        /// such as "16x16 32bpp, 256x256 32bpp", or "" when the group is
        /// missing or empty, or names an image (RT_ICON) the file lacks.
        public static string IconGroup(string path, int id)
        {
            IntPtr module = LoadLibraryEx(path, IntPtr.Zero, 0x00000002 /* LOAD_LIBRARY_AS_DATAFILE */);
            if (module == IntPtr.Zero) return "";
            try
            {
                IntPtr info = FindResource(module, new IntPtr(id), new IntPtr(14) /* RT_GROUP_ICON */);
                if (info == IntPtr.Zero) return "";
                uint size = SizeofResource(module, info);
                if (size < 6) return "";
                byte[] dir = new byte[size];
                Marshal.Copy(LockResource(LoadResource(module, info)), dir, 0, (int)size);
                int count = BitConverter.ToUInt16(dir, 4);
                if (count == 0 || size < 6 + 14 * count) return "";
                StringBuilder images = new StringBuilder();
                for (int i = 0; i < count; i++)
                {
                    // GRPICONDIRENTRY: width, height, colors, reserved, planes, bit count, bytes, id.
                    int entry = 6 + 14 * i;
                    int width = dir[entry] == 0 ? 256 : dir[entry];
                    int height = dir[entry + 1] == 0 ? 256 : dir[entry + 1];
                    int bits = BitConverter.ToUInt16(dir, entry + 6);
                    int image = BitConverter.ToUInt16(dir, entry + 12);
                    IntPtr icon = FindResource(module, new IntPtr(image), new IntPtr(3) /* RT_ICON */);
                    if (icon == IntPtr.Zero || SizeofResource(module, icon) == 0) return "";
                    if (images.Length > 0) images.Append(", ");
                    images.Append(width + "x" + height + " " + bits + "bpp");
                }
                return images.ToString();
            }
            finally { FreeLibrary(module); }
        }
    }
}
'@
    Add-Type -TypeDefinition $appExeHelpers -Language CSharp

    Invoke-Step 'exe' {
        Assert (Test-Path -LiteralPath $Exe) "MelCat.exe not found at $Exe"
        $bytes = [System.IO.File]::ReadAllBytes($Exe)
        $pe = [BitConverter]::ToInt32($bytes, 0x3C)
        $machine = [BitConverter]::ToUInt16($bytes, $pe + 4)
        $subsystem = [BitConverter]::ToUInt16($bytes, $pe + 24 + 68)
        Note ('{0:N0} bytes; machine 0x{1:X4}; subsystem {2}' -f $bytes.Length, $machine, $subsystem)
        Assert ($machine -eq 0x8664) 'not an x64 executable'
        Assert ($subsystem -eq 2) 'not a GUI (windows subsystem) executable'

        $info = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($Exe)
        Note "ProductName '$($info.ProductName)', FileVersion '$($info.FileVersion)', OriginalFilename '$($info.OriginalFilename)'"
        Assert ($info.ProductName -eq 'MelCat') 'the version info does not name MelCat'
        $cargo = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '..\Cargo.toml')
        $version = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
        Assert ($info.ProductVersion -eq $version) "ProductVersion '$($info.ProductVersion)' is not the workspace version '$version'"
        # The workspace version as four numbers, as VERSIONINFO and the manifest hold it.
        $numbers = @(($version -split '[-+]')[0].Split('.'))
        while ($numbers.Count -lt 4) { $numbers += '0' }
        $version4 = $numbers -join '.'
        $fileNumbers = '{0}.{1}.{2}.{3}' -f $info.FileMajorPart, $info.FileMinorPart, $info.FileBuildPart, $info.FilePrivatePart
        $productNumbers = '{0}.{1}.{2}.{3}' -f $info.ProductMajorPart, $info.ProductMinorPart, $info.ProductBuildPart, $info.ProductPrivatePart
        Note "ProductVersion '$($info.ProductVersion)'; FILEVERSION $fileNumbers, PRODUCTVERSION $productNumbers"
        Assert ($info.FileVersion -eq $version) "FileVersion '$($info.FileVersion)' is not the workspace version '$version'"
        Assert ($fileNumbers -eq $version4) "FILEVERSION $fileNumbers is not the workspace version $version4"
        Assert ($productNumbers -eq $version4) "PRODUCTVERSION $productNumbers is not the workspace version $version4"

        # ExtractAssociatedIcon falls back to a stock shell icon, so read the resource itself.
        $icons = [MelCatSmoke.AppExe.Res]::IconGroup($Exe, 1)
        Note "icon group 1: $(if ($icons) { $icons } else { 'missing' })"
        Assert ($icons -ne '') 'MelCat.exe has no icon: no icon group 1 (RT_GROUP_ICON) with all its images'

        $manifest = [MelCatSmoke.Win]::Manifest($Exe)
        Assert ($manifest -match 'PerMonitorV2') 'the manifest does not ask for PerMonitorV2 DPI awareness'
        Assert ($manifest -match 'Microsoft\.Windows\.Common-Controls') 'the manifest does not ask for common controls v6'
        Assert ($manifest -match 'asInvoker') 'the manifest does not run as invoker'
        $identity = [regex]::Match($manifest, '<assemblyIdentity\b[^>]*\bname="dev\.melcat\.MelCat"[^>]*>')
        Assert $identity.Success 'the manifest has no assemblyIdentity named dev.melcat.MelCat'
        $manifestVersion = [regex]::Match($identity.Value, '\bversion="([^"]*)"').Groups[1].Value
        Note "manifest: PerMonitorV2, common controls v6, asInvoker, version '$manifestVersion'"
        Assert ($manifestVersion -eq $version4) "the manifest's assemblyIdentity version '$manifestVersion' is not the workspace version $version4"
    }

    # =======================================================================
    # start

    $startHelpers = @'
using System;
using System.Runtime.InteropServices;
using System.Text;

namespace MelCatSmoke.Start
{
    public static class Win
    {
        public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lparam);
        [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc proc, IntPtr lparam);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder name, int max);
        [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("user32.dll", SetLastError = true)] static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wparam, IntPtr lparam);

        /// The first top-level window of class `cls` that belongs to process
        /// `pid`, visible or not, or zero.
        public static IntPtr FindOwned(string cls, int pid)
        {
            IntPtr result = IntPtr.Zero;
            EnumWindows(delegate(IntPtr hwnd, IntPtr unused)
            {
                StringBuilder name = new StringBuilder(256);
                GetClassName(hwnd, name, name.Capacity);
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner == (uint)pid && name.ToString() == cls) { result = hwnd; return false; }
                return true;
            }, IntPtr.Zero);
            return result;
        }

        /// Posts `msg` without parameters; "" or the error.
        public static string Post(IntPtr hwnd, uint msg)
        {
            if (PostMessage(hwnd, msg, IntPtr.Zero, IntPtr.Zero)) return "";
            return "PostMessage error " + Marshal.GetLastWin32Error();
        }
    }
}
'@
    Add-Type -TypeDefinition $startHelpers -Language CSharp

    # Asks every MelCat to quit (WM_CLOSE to its MelCat.Main window, like
    # taskkill without /f) and kills any that still runs 5 s later. Returns ''
    # when each one quit within 5 s of WM_CLOSE with exit code 0, otherwise
    # what went wrong.
    function Invoke-MelCatQuit {
        $problems = New-Object System.Collections.Generic.List[string]
        foreach ($process in @(Get-Process -Name MelCat -ErrorAction SilentlyContinue)) {
            try { $null = $process.Handle } catch { }  # keeps ExitCode readable after it ends
            $main = [MelCatSmoke.Start.Win]::FindOwned('MelCat.Main', $process.Id)
            if ($main -eq [IntPtr]::Zero) {
                $problems.Add("MelCat pid $($process.Id) has no MelCat.Main window")
            } else {
                $err = [MelCatSmoke.Start.Win]::Post($main, 0x0010)
                if ($err) { $problems.Add("WM_CLOSE to MelCat pid $($process.Id): $err") }
            }
            if (-not $process.WaitForExit(5000)) {
                Write-Host "    (MelCat pid $($process.Id) did not quit on WM_CLOSE; killing it)"
                $problems.Add("MelCat pid $($process.Id) still ran 5 s after WM_CLOSE")
                Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
                [void]$process.WaitForExit(5000)
            } else {
                $code = $null
                try { $code = $process.ExitCode } catch { }
                if (($null -ne $code) -and ($code -ne 0)) { $problems.Add(('MelCat pid {0} quit with exit code 0x{1:X8}' -f $process.Id, $code)) }
            }
        }
        return ($problems -join '; ')
    }

    # Invoke-MelCatQuit, whatever the outcome.
    function Close-MelCat { $null = Invoke-MelCatQuit }

    # Invoke-MelCatQuit, failing the step unless every MelCat quit cleanly.
    function Stop-MelCatCleanly {
        $quit = Invoke-MelCatQuit
        Assert ($quit -eq '') "MelCat did not quit cleanly on WM_CLOSE: $quit"
    }

    Write-Host ''
    Write-Host "MelCat smoke test: $Exe (version $($script:Version))"
    Write-Host ("OS {0}; PowerShell {1}, {2}; session {3}" -f [Environment]::OSVersion.VersionString, $PSVersionTable.PSVersion, [System.Threading.Thread]::CurrentThread.GetApartmentState(), [System.Diagnostics.Process]::GetCurrentProcess().SessionId)
    Write-Host ("primary screen {0} at {1} DPI; virtual screen {2}" -f $script:Primary, $script:Dpi, [System.Windows.Forms.SystemInformation]::VirtualScreen)
    Write-Host ("in pixels: Gentle radius {0}, Playful radius {1}, edge band {2}" -f $script:GentlePx, $script:PlayfulPx, $script:EdgeBandPx)

    Invoke-Step 'start' {
        Stop-MelCat
        Remove-Item -LiteralPath $LogPath -Force -ErrorAction SilentlyContinue
        Write-MelSettings 'Always' 'Gentle'
        Note "settings.json: $([System.IO.File]::ReadAllText($SettingsPath))"
        $mark = Get-LogMark
        $script:Mel = Start-MelCat
        $starting = Wait-Log 'MelCat \S+ starting$' $mark 10000
        Assert ($null -ne $starting) "MelCat logged no 'MelCat <version> starting' within 10 s"
        Note "log: '$(Get-LogText $starting)'"
        Assert ($starting -match ('MelCat {0} starting$' -f [regex]::Escape($script:Version))) "the start line does not name version $($script:Version)"
        $tray = Wait-Log 'tray icon added$' $mark 15000
        Assert ($null -ne $tray) "MelCat logged no 'tray icon added' within 15 s"
        Note "log: '$(Get-LogText $tray)'"
        Assert (-not $script:Mel.HasExited) ('MelCat exited with code {0}' -f $script:Mel.ExitCode)

        $main = [MelCatSmoke.Start.Win]::FindOwned('MelCat.Main', $script:Mel.Id)
        Assert ($main -ne [IntPtr]::Zero) "no MelCat.Main window belongs to MelCat (pid $($script:Mel.Id))"
        $icon = [MelCatSmoke.Win]::TrayIcon($main, 1)
        Note ('Shell_NotifyIconGetRect(MelCat.Main 0x{0:X}, uID 1): {1}' -f $main.ToInt64(), $icon)
        Assert ($icon -match '^rect=(-?\d+),(-?\d+),(-?\d+),(-?\d+)$') "the shell does not know MelCat's tray icon: $icon"
        Assert (([int]$Matches[3] -gt [int]$Matches[1]) -and ([int]$Matches[4] -gt [int]$Matches[2])) "the tray icon's rectangle is empty: $icon"
        Save-Desktop 'started'

        $mark = Get-LogMark
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        $second = Start-MelCat
        $exited = $second.WaitForExit(3000)
        $ms = $watch.ElapsedMilliseconds
        if (-not $exited) { Stop-Process -Id $second.Id -Force -ErrorAction SilentlyContinue }
        Assert $exited "a second MelCat.exe (pid $($second.Id)) was still running after 3 s"
        Note ('a second MelCat.exe (pid {0}) exited after {1} ms with code {2}' -f $second.Id, $ms, $second.ExitCode)
        Assert ($second.ExitCode -eq 0) ('the second MelCat.exe exited with code {0}, not 0' -f $second.ExitCode)
        $line = Wait-Log 'another MelCat is already running; exiting$' $mark 3000
        Assert ($null -ne $line) "the second MelCat.exe did not log 'another MelCat is already running; exiting'"
        Note "log: '$(Get-LogText $line)'"
        Assert (-not $script:Mel.HasExited) 'the first MelCat exited when the second one started'
    }

    # =======================================================================
    # stroll: pointer injection, the pointer sampler, and a MelCat per step.

    $strollHelpers = @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;

namespace MelCatSmoke.Stroll
{
    public static class Native
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Sequential)]
        public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Explicit)]
        public struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
        [StructLayout(LayoutKind.Sequential)]
        public struct INPUT { public uint type; public InputUnion u; }
        [StructLayout(LayoutKind.Sequential)]
        public struct POINT { public int X; public int Y; }
        [StructLayout(LayoutKind.Sequential)]
        public struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }

        [DllImport("user32.dll", SetLastError = true)]
        public static extern uint SendInput(uint count, [In, MarshalAs(UnmanagedType.LPArray)] INPUT[] inputs, int size);
        [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
        [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
        [DllImport("user32.dll")] public static extern bool GetLastInputInfo(ref LASTINPUTINFO info);
        [DllImport("kernel32.dll")] public static extern uint GetTickCount();
        [DllImport("winmm.dll")] public static extern uint timeBeginPeriod(uint period);
    }

    /// Pointer moves injected like a hand's: absolute, on the virtual
    /// desktop, dwExtraInfo 0 (MelCat marks its own moves "MCAT").
    public static class Mouse
    {
        public static string LastError = "";

        static Mouse() { Native.timeBeginPeriod(1); }

        /// The mapping MelCat uses too: the center of the pixel.
        static int Normalize(int value, int origin, int extent)
        {
            if (extent <= 0) return 0;
            long offset = Math.Max(0L, Math.Min((long)extent - 1, (long)value - origin));
            return (int)Math.Min(65535L, (offset * 65536 + 32768) / extent);
        }

        public static bool Move(int x, int y)
        {
            int left = Native.GetSystemMetrics(76), top = Native.GetSystemMetrics(77);
            int width = Native.GetSystemMetrics(78), height = Native.GetSystemMetrics(79);
            Native.INPUT input = new Native.INPUT();
            input.type = 0;
            input.u.mi.dx = Normalize(x, left, width);
            input.u.mi.dy = Normalize(y, top, height);
            input.u.mi.dwFlags = 0x0001 | 0x8000 | 0x4000;  // MOVE | ABSOLUTE | VIRTUALDESK
            input.u.mi.dwExtraInfo = IntPtr.Zero;
            uint inserted = Native.SendInput(1, new Native.INPUT[] { input }, Marshal.SizeOf(typeof(Native.INPUT)));
            if (inserted != 1)
            {
                LastError = "SendInput inserted " + inserted + " of 1 events, error " + Marshal.GetLastWin32Error();
                return false;
            }
            return true;
        }

        public static string Cursor()
        {
            Native.POINT p;
            if (!Native.GetCursorPos(out p)) return "unknown";
            return p.X + "," + p.Y;
        }

        public static bool CursorIs(int x, int y)
        {
            Native.POINT p;
            return Native.GetCursorPos(out p) && p.X == x && p.Y == y;
        }

        /// Waits until the pointer leaves where it is now. Returns the
        /// milliseconds waited, or -1 after `timeoutMs`.
        public static int WaitForMove(int timeoutMs)
        {
            Native.timeBeginPeriod(1);
            Native.POINT start;
            if (!Native.GetCursorPos(out start)) return -1;
            Stopwatch watch = Stopwatch.StartNew();
            while (watch.ElapsedMilliseconds < timeoutMs)
            {
                Native.POINT p;
                if (Native.GetCursorPos(out p) && (p.X != start.X || p.Y != start.Y)) return (int)watch.ElapsedMilliseconds;
                Thread.Sleep(1);
            }
            return -1;
        }

        /// Milliseconds since the last input in this session (GetLastInputInfo).
        public static long IdleMs()
        {
            Native.LASTINPUTINFO info = new Native.LASTINPUTINFO();
            info.cbSize = (uint)Marshal.SizeOf(typeof(Native.LASTINPUTINFO));
            if (!Native.GetLastInputInfo(ref info)) return -1;
            return (long)unchecked(Native.GetTickCount() - info.dwTime);
        }
    }

    /// A pointer position the sampler saw, `Ms` after it started.
    public class Sample
    {
        public int Ms;
        public int X;
        public int Y;
        public override string ToString() { return X + "," + Y + "@" + Ms + "ms"; }
    }

    /// Reads GetCursorPos about every millisecond on its own thread and
    /// keeps every change of position.
    public static class Sampler
    {
        static readonly object Gate = new object();
        static readonly List<Sample> changes = new List<Sample>();
        static readonly Stopwatch watch = new Stopwatch();
        static Thread thread;
        static volatile bool running;
        public static DateTime StartedAt;
        public static int Reads;
        public static int LongestGapMs;

        public static void Start()
        {
            Stop();
            Native.timeBeginPeriod(1);
            lock (Gate) changes.Clear();
            Reads = 0;
            LongestGapMs = 0;
            StartedAt = DateTime.Now;
            watch.Reset();
            watch.Start();
            running = true;
            thread = new Thread(Run);
            thread.IsBackground = true;
            thread.Priority = ThreadPriority.AboveNormal;
            thread.Start();
            // The first read: where the pointer is now.
            for (int i = 0; i < 200 && Changes().Length == 0; i++) Thread.Sleep(1);
        }

        static void Run()
        {
            int lastX = int.MinValue, lastY = int.MinValue;
            long lastRead = -1;
            while (running)
            {
                Native.POINT p;
                bool ok = Native.GetCursorPos(out p);
                long now = watch.ElapsedMilliseconds;
                if (lastRead >= 0 && now - lastRead > LongestGapMs) LongestGapMs = (int)(now - lastRead);
                lastRead = now;
                if (ok)
                {
                    Reads++;
                    if (p.X != lastX || p.Y != lastY)
                    {
                        Sample s = new Sample();
                        s.Ms = (int)now;
                        s.X = p.X;
                        s.Y = p.Y;
                        lock (Gate) changes.Add(s);
                        lastX = p.X;
                        lastY = p.Y;
                    }
                }
                Thread.Sleep(1);
            }
        }

        public static void Stop()
        {
            running = false;
            if (thread != null)
            {
                thread.Join(2000);
                thread = null;
            }
        }

        /// Milliseconds since Start.
        public static int Now() { return (int)watch.ElapsedMilliseconds; }

        /// The sampler's time of a wall-clock moment, such as a MelCat log line's.
        public static int MsAt(DateTime time) { return (int)Math.Round((time - StartedAt).TotalMilliseconds); }

        /// Every change of position, oldest first. The first is where the
        /// pointer was when the sampler started.
        public static Sample[] Changes() { lock (Gate) return changes.ToArray(); }
    }
}
'@
    Add-Type -TypeDefinition $strollHelpers -Language CSharp

    function Invoke-Input([bool]$Ok, [string]$What) {
        if (-not $Ok) { throw "SendInput failed for ${What}: $([MelCatSmoke.Stroll.Mouse]::LastError)" }
    }

    # One injected move; fails unless the pointer lands exactly there.
    function Set-Pointer([int]$X, [int]$Y) {
        Invoke-Input ([MelCatSmoke.Stroll.Mouse]::Move($X, $Y)) "the move to $X,$Y"
        $landed = Wait-Until { [MelCatSmoke.Stroll.Mouse]::CursorIs($X, $Y) } 1000
        Assert $landed "a move injected to $X,$Y left the pointer at $([MelCatSmoke.Stroll.Mouse]::Cursor())"
    }

    # Asks any MelCat to quit (failing unless it quits cleanly), writes the
    # settings, parks the pointer (one injected move), starts the sampler,
    # then MelCat, and waits for its first status line. Returns the log mark
    # taken just before the start.
    function Restart-MelCat([string]$Mode = 'Always', [string]$Intensity = 'Gentle', [int[]]$Park = $null) {
        Stop-MelCatCleanly
        Write-MelSettings $Mode $Intensity
        if ($null -ne $Park) { Set-Pointer $Park[0] $Park[1] }
        [MelCatSmoke.Stroll.Sampler]::Start()
        $mark = Get-LogMark
        $script:Mel = Start-MelCat
        $status = Wait-Log 'status: (off|napping|active)$' $mark 15000
        Assert ($null -ne $status) "MelCat (pid $($script:Mel.Id)) logged no status within 15 s"
        $where = if ($null -ne $Park) { "pointer parked at $($Park[0]),$($Park[1])" } else { 'pointer not parked' }
        Note ("MelCat pid {0} started with {{{1}, {2}, idle {3} s}}, {4}: '{5}'" -f $script:Mel.Id, $Mode, $Intensity, $script:IdleSeconds, $where, (Get-LogText $status))
        return $mark
    }

    # The sampled position farthest from (X, Y): @(distance, sample).
    function Get-Farthest([object[]]$Samples, [double]$X, [double]$Y) {
        $max = 0.0
        $at = $null
        foreach ($s in $Samples) {
            $d = Get-Distance $s.X $s.Y $X $Y
            if ($null -eq $at -or $d -gt $max) { $max = $d; $at = $s }
        }
        return @($max, $at)
    }

    # The session's idle time while MelCat strolls (used by the 'stroll' step).
    $jiggleIdleHelpers = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Threading;

namespace MelCatSmoke.JiggleIdle
{
    /// Reads GetLastInputInfo about every 10 ms on its own thread: the
    /// largest idle time seen, and when each input the session counted came.
    public static class IdleWatch
    {
        [StructLayout(LayoutKind.Sequential)]
        struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }
        [DllImport("user32.dll")] static extern bool GetLastInputInfo(ref LASTINPUTINFO info);
        [DllImport("kernel32.dll")] static extern uint GetTickCount();

        static readonly object Gate = new object();
        static readonly List<DateTime> inputs = new List<DateTime>();
        static Thread thread;
        static volatile bool running;
        public static long LargestIdleMs;
        public static int Reads;

        public static void Start()
        {
            Stop();
            lock (Gate) inputs.Clear();
            LargestIdleMs = 0;
            Reads = 0;
            running = true;
            thread = new Thread(Run);
            thread.IsBackground = true;
            thread.Priority = ThreadPriority.AboveNormal;
            thread.Start();
        }

        static void Run()
        {
            bool first = true;
            uint last = 0;
            while (running)
            {
                LASTINPUTINFO info = new LASTINPUTINFO();
                info.cbSize = (uint)Marshal.SizeOf(typeof(LASTINPUTINFO));
                if (GetLastInputInfo(ref info))
                {
                    long idle = (long)unchecked(GetTickCount() - info.dwTime);
                    DateTime now = DateTime.Now;
                    Reads++;
                    if (idle > LargestIdleMs) LargestIdleMs = idle;
                    if (first || info.dwTime != last)
                    {
                        lock (Gate) inputs.Add(now.AddMilliseconds(-idle));
                        last = info.dwTime;
                        first = false;
                    }
                }
                Thread.Sleep(10);
            }
        }

        public static void Stop()
        {
            running = false;
            if (thread != null)
            {
                thread.Join(2000);
                thread = null;
            }
        }

        /// When the session counted input, oldest first; the first is the
        /// last input before Start.
        public static DateTime[] Inputs() { lock (Gate) return inputs.ToArray(); }
    }
}
'@
    Add-Type -TypeDefinition $jiggleIdleHelpers -Language CSharp

    Invoke-Step 'stroll' {
        $park = Get-ScreenCenter
        $limit = $script:GentlePx + 1
        $mark = Restart-MelCat -Intensity 'Gentle' -Park $park
        # (idle) From here on the script sends no input.
        [MelCatSmoke.JiggleIdle.IdleWatch]::Start()
        $moved = Wait-Until { [MelCatSmoke.Stroll.Sampler]::Changes().Length -gt 1 } 15000
        $changes = [MelCatSmoke.Stroll.Sampler]::Changes()
        Assert $moved 'the pointer did not move by itself within 15 s of being parked'
        Note ('the pointer first moved by itself {0} ms after the sampler started, to {1}' -f $changes[1].Ms, $changes[1])
        $started = Wait-Log 'stroll started: ' $mark 3000
        Assert ($null -ne $started) "the pointer moved but MelCat logged no 'stroll started'"

        $finished = @(Wait-LogCount 'stroll finished at ' $mark 4 45000)
        [MelCatSmoke.Stroll.Sampler]::Stop()
        [MelCatSmoke.JiggleIdle.IdleWatch]::Stop()
        $changes = [MelCatSmoke.Stroll.Sampler]::Changes()
        $events = @(Get-StrollEvents $mark)
        Note "log: $(Format-StrollEvents $events)"
        Assert ($finished.Count -ge 4) "only $($finished.Count) of 4 strolls finished within 45 s"
        $kinds = Get-StrollKinds $events
        Assert ($kinds -match '^started finished started finished started finished started finished( started)?$') "the strolls did not simply start and finish in turn: $kinds"

        $first = $changes[0]
        Assert (($first.X -eq $park[0]) -and ($first.Y -eq $park[1])) "the first sampled position, $first, is not the parking spot"
        $starts = @($events | Where-Object { $_.Kind -eq 'started' })
        Assert (($starts[0].FromX -eq $park[0]) -and ($starts[0].FromY -eq $park[1])) "the first stroll did not start from the parking spot: $($starts[0].Detail)"
        $far = Get-Farthest $changes $park[0] $park[1]
        Note ('{0} positions in {1} reads (longest gap {2} ms); farthest {3:N2} px from the parking spot, at {4}; allowed {5:N0} px' -f $changes.Length, [MelCatSmoke.Stroll.Sampler]::Reads, [MelCatSmoke.Stroll.Sampler]::LongestGapMs, $far[0], $far[1], $limit)
        Assert ($far[0] -le $limit) ('the pointer wandered {0:N2} px from the parking spot (at {1}), beyond the Gentle radius {2} px + 1' -f $far[0], $far[1], $script:GentlePx)
        foreach ($s in $starts) {
            $d = Get-Distance $s.ToX $s.ToY $park[0] $park[1]
            Assert ($d -le $limit) ('a stroll aimed {0:N2} px from the parking spot: {1}' -f $d, $s.Detail)
        }

        # Idle again after each stroll: the next one comes after a fresh
        # threshold (75 to 100 % of 5 s, minus 1.2 s of tick slack: 2.55 to
        # 3.8 s), at the first tick after it (up to 1 s, plus the 0.2 s the
        # slack allows for timer jitter), so never later than the 5 s
        # interval. 0.05 s more covers the log time stamps at both ends.
        $gaps = @()
        $ends = @($events | Where-Object { $_.Kind -eq 'finished' })
        for ($i = 0; $i -lt 3; $i++) { $gaps += ($starts[$i + 1].Time - $ends[$i].Time).TotalSeconds }
        Note ('pauses between strolls: {0} s; stroll lengths: {1} steps' -f (($gaps | ForEach-Object { '{0:N2}' -f $_ }) -join ', '), (($starts | ForEach-Object { $_.Steps }) -join ', '))
        foreach ($gap in $gaps) {
            Assert (($gap -ge 2.5) -and ($gap -le 5.05)) ('a pause between two strolls lasted {0:N2} s, outside 2.5 to 5 s (5.05 s with the log time stamps)' -f $gap)
        }

        # (idle) MelCat's own moves count as input for the session, which is
        # what keeps the computer awake: the script sent nothing after
        # parking, yet input shows up during every stroll, and the idle time
        # never grows past the interval plus the slow first and last steps.
        $inputs = @([MelCatSmoke.JiggleIdle.IdleWatch]::Inputs())
        $largestIdle = [MelCatSmoke.JiggleIdle.IdleWatch]::LargestIdleMs
        $idleLimit = ($script:IdleSeconds + 2.5) * 1000
        $strollEnds = @($events | Where-Object { $_.Kind -eq 'finished' })
        $inputsPerStroll = @()
        for ($i = 0; $i -lt 4; $i++) {
            $from = $starts[$i].Time.AddMilliseconds(-50)
            $to = $strollEnds[$i].Time.AddMilliseconds(50)
            $inputsPerStroll += @($inputs | Where-Object { ($_ -ge $from) -and ($_ -le $to) }).Count
        }
        Note ('session idle time (GetLastInputInfo, {0} reads): largest {1:N2} s, allowed {2:N1} s; inputs the session counted during each stroll: {3}' -f [MelCatSmoke.JiggleIdle.IdleWatch]::Reads, ($largestIdle / 1000.0), ($idleLimit / 1000.0), ($inputsPerStroll -join ', '))
        for ($i = 0; $i -lt 4; $i++) {
            Assert ($inputsPerStroll[$i] -ge 1) "the session counted no input during stroll $($i + 1) ($($starts[$i].Detail)): MelCat's moves did not reset its idle time"
        }
        Assert ($largestIdle -le $idleLimit) ('the session was idle for {0:N2} s while MelCat strolled every few seconds: its moves did not keep the idle time down' -f ($largestIdle / 1000.0))
    }

    # =======================================================================
    # edges

    # Edge checks over the positions sampled from one parking spot near the
    # top-left corner: once the pointer has left the edge band it never comes
    # within 2 px of an edge again; it never goes farther out than the spot
    # (left of or above it); and it stays within the radius of the spot
    # moved inside the band, where strolls are planned around.
    function Test-EdgeRun([string]$Name, [object[]]$Run, [int[]]$Park, [double]$Radius) {
        Assert ($Run.Count -gt 1) "${Name}: the pointer never moved"
        Assert (($Run[0].X -eq $Park[0]) -and ($Run[0].Y -eq $Park[1])) "${Name}: the first sampled position, $($Run[0]), is not the parking spot"
        $band = $script:EdgeBandPx
        $anchorX = [Math]::Max([double]$Park[0], $script:Primary.Left + $band)
        $anchorY = [Math]::Max([double]$Park[1], $script:Primary.Top + $band)
        $leftAt = $null
        $closest = [double]::MaxValue
        $closestAt = $null
        $outward = $null
        foreach ($s in $Run) {
            $d = Get-EdgeDistance $s.X $s.Y
            if (($null -eq $leftAt) -and ($d -ge $band)) { $leftAt = $s }
            if (($null -ne $leftAt) -and ($d -lt $closest)) { $closest = $d; $closestAt = $s }
            if (($null -eq $outward) -and (($s.X -lt $Park[0]) -or ($s.Y -lt $Park[1]))) { $outward = $s }
        }
        $far = Get-Farthest $Run $anchorX $anchorY
        $last = $Run[$Run.Count - 1]
        Note ('{0}: {1} positions; left the band at {2}; then at least {3} px from every edge (at {4}); farthest {5:N2} px from {6},{7}; last {8}' -f $Name, $Run.Count, $leftAt, $(if ($null -ne $closestAt) { $closest } else { '-' }), $closestAt, $far[0], $anchorX, $anchorY, $last)
        Assert ($null -ne $leftAt) "${Name}: the pointer never left the $band px edge band"
        Assert ($closest -ge 2) "${Name}: once out of the edge band the pointer came within $closest px of an edge, at $closestAt"
        Assert ($null -eq $outward) "${Name}: the pointer moved outwards from $($Park[0]),$($Park[1]), to $outward"
        Assert ($far[0] -le $Radius + 1) ('{0}: the pointer wandered {1:N2} px from {2},{3} (at {4}), beyond the radius {5} px + 1' -f $Name, $far[0], $anchorX, $anchorY, $far[1], $Radius)
    }

    Invoke-Step 'edges' {
        $p = $script:Primary
        $near = @(($p.Left + 2), ($p.Top + 2))
        $corner = @($p.Left, $p.Top)
        $mark = Restart-MelCat -Intensity 'Playful' -Park $near
        $first = @(Wait-LogCount 'stroll finished at ' $mark 3 45000)
        Assert ($first.Count -ge 3) "only $($first.Count) of 3 strolls from $($near -join ',') finished within 45 s"
        # Right after a stroll: the next one is at least 2.5 s away.
        $mark2 = Get-LogMark
        $t0 = [MelCatSmoke.Stroll.Sampler]::Now()
        Set-Pointer $corner[0] $corner[1]
        $second = @(Wait-LogCount 'stroll finished at ' $mark2 2 30000)
        [MelCatSmoke.Stroll.Sampler]::Stop()
        $changes = [MelCatSmoke.Stroll.Sampler]::Changes()
        $eventsA = @(Get-StrollEvents $mark $mark2)
        $eventsB = @(Get-StrollEvents $mark2)
        Note "log from $($near -join ','): $(Format-StrollEvents $eventsA)"
        Note "log from $($corner -join ','): $(Format-StrollEvents $eventsB)"
        Assert ($second.Count -ge 2) "only $($second.Count) of 2 strolls from $($corner -join ',') finished within 30 s"
        foreach ($pair in @(@($eventsA, $near), @($eventsB, $corner))) {
            $events = $pair[0]
            $spot = $pair[1]
            $kinds = Get-StrollKinds $events
            Assert ($events.Count -gt 0) "no stroll from $($spot -join ',')"
            Assert ($kinds -notmatch 'stopped') "a stroll stopped without any input: $kinds"
            Assert (($events[0].Kind -eq 'started') -and ($events[0].FromX -eq $spot[0]) -and ($events[0].FromY -eq $spot[1])) "the first stroll did not start from $($spot -join ','): $($events[0].Detail)"
        }
        $runA = @($changes | Where-Object { $_.Ms -lt $t0 })
        $runB = @($changes | Where-Object { $_.Ms -ge $t0 })
        Test-EdgeRun "from $($near -join ',')" $runA $near $script:PlayfulPx
        Test-EdgeRun "from $($corner -join ',')" $runB $corner $script:PlayfulPx
    }

    # =======================================================================
    # takeover

    Invoke-Step 'takeover' {
        $park = Get-ScreenCenter
        $limit = $script:GentlePx + 1
        $target = @(($park[0] + 200), $park[1])
        $mark = Restart-MelCat -Intensity 'Gentle' -Park $park
        $waited = [MelCatSmoke.Stroll.Mouse]::WaitForMove(15000)
        Assert ($waited -ge 0) 'no stroll started within 15 s of parking the pointer'
        Start-Sleep -Milliseconds 60
        $from = [MelCatSmoke.Stroll.Mouse]::Cursor()
        $sentAt = [DateTime]::Now
        $sentMs = [MelCatSmoke.Stroll.Sampler]::Now()
        Invoke-Input ([MelCatSmoke.Stroll.Mouse]::Move($target[0], $target[1])) 'the takeover move'
        $stop = Wait-Log 'stroll stopped: ' $mark 3000
        # Hands off until the next stroll has finished.
        $next = Wait-Log 'stroll finished at ' $mark 15000
        [MelCatSmoke.Stroll.Sampler]::Stop()
        $changes = [MelCatSmoke.Stroll.Sampler]::Changes()
        $events = @(Get-StrollEvents $mark)
        Note "log: $(Format-StrollEvents $events)"
        Assert ($null -ne $stop) "no 'stroll stopped' within 3 s of the takeover move"
        Assert ($stop -match 'stroll stopped: pointer moved$') "the stroll stopped for another reason: '$(Get-LogText $stop)'"
        $kinds = Get-StrollKinds $events
        Assert ($kinds -match '^started stopped started finished') "expected a stroll stopped by the takeover, then a whole stroll: $kinds"

        $delay = ($events[1].Time - $sentAt).TotalMilliseconds
        Note ('takeover move from {0} to {1},{2} sent 60 ms after the stroll''s first visible move ({3} ms after parking); MelCat logged the stop {4:N0} ms later' -f $from, $target[0], $target[1], $waited, $delay)
        Assert ($delay -le 100) ('MelCat logged the stop {0:N0} ms after the takeover move, not within a frame or two' -f $delay)

        $after = @($changes | Where-Object { $_.Ms -ge $sentMs })
        $land = -1
        for ($i = 0; $i -lt $after.Count; $i++) {
            if (($after[$i].X -eq $target[0]) -and ($after[$i].Y -eq $target[1])) { $land = $i; break }
        }
        Assert ($land -ge 0) "the pointer never showed up at the takeover spot: $($after -join ' ')"
        $landed = $after[$land]
        $early = if ($land -gt 0) { ($after[0..($land - 1)] -join ' ') } else { 'none' }
        $rest = @($after | Select-Object -Skip ($land + 1))
        $stillMs = if ($rest.Count -gt 0) { $rest[0].Ms - $landed.Ms } else { [MelCatSmoke.Stroll.Sampler]::Now() - $landed.Ms }
        $nextMove = if ($rest.Count -gt 0) { $rest[0] } else { 'none' }
        Note ('landed at {0} ({1} ms after sending; MelCat moves in between: {2}); stayed exactly there {3} ms, next move {4}' -f $landed, ($landed.Ms - $sentMs), $early, $stillMs, $nextMove)
        Assert ($stillMs -ge 1500) "the pointer left the takeover spot after $stillMs ms, for $nextMove"

        Assert (($events[2].FromX -eq $target[0]) -and ($events[2].FromY -eq $target[1])) "the next stroll did not start from the takeover spot: $($events[2].Detail)"
        $far = Get-Farthest @($after | Select-Object -Skip $land) $target[0] $target[1]
        Note ('the next stroll: farthest {0:N2} px from the takeover spot, at {1}; allowed {2:N0} px' -f $far[0], $far[1], $limit)
        Assert ($far[0] -le $limit) ('the next stroll wandered {0:N2} px from the takeover spot (at {1}), beyond the Gentle radius + 1' -f $far[0], $far[1])
        Assert ((Get-Distance $events[2].ToX $events[2].ToY $target[0] $target[1]) -le $limit) "the next stroll aimed away from the takeover spot: $($events[2].Detail)"
    }

    # =======================================================================
    # input: a key and the wheel

    $inputHelpers = @'
using System;
using System.Runtime.InteropServices;
using System.Threading;

namespace MelCatSmoke.Input
{
    public static class Native
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Sequential)]
        public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Explicit)]
        public struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
        [StructLayout(LayoutKind.Sequential)]
        public struct INPUT { public uint type; public InputUnion u; }

        [DllImport("user32.dll", SetLastError = true)]
        public static extern uint SendInput(uint count, [In, MarshalAs(UnmanagedType.LPArray)] INPUT[] inputs, int size);
    }

    /// A key and the wheel, injected like a person's (dwExtraInfo 0).
    public static class Send
    {
        public static string LastError = "";

        static bool One(Native.INPUT input)
        {
            uint inserted = Native.SendInput(1, new Native.INPUT[] { input }, Marshal.SizeOf(typeof(Native.INPUT)));
            if (inserted != 1)
            {
                LastError = "SendInput inserted " + inserted + " of 1 events, error " + Marshal.GetLastWin32Error();
                return false;
            }
            return true;
        }

        public static bool Key(ushort vk, bool up)
        {
            Native.INPUT input = new Native.INPUT();
            input.type = 1;
            input.u.ki.wVk = vk;
            input.u.ki.dwFlags = up ? 0x0002u : 0u;  // KEYEVENTF_KEYUP
            input.u.ki.dwExtraInfo = IntPtr.Zero;
            return One(input);
        }

        /// Press, hold `holdMs`, release.
        public static bool Tap(ushort vk, int holdMs)
        {
            if (!Key(vk, false)) return false;
            Thread.Sleep(holdMs);
            return Key(vk, true);
        }

        /// `delta` wheel units (120 is one notch) where the pointer is.
        public static bool Wheel(int delta)
        {
            Native.INPUT input = new Native.INPUT();
            input.type = 0;
            input.u.mi.mouseData = unchecked((uint)delta);
            input.u.mi.dwFlags = 0x0800;  // MOUSEEVENTF_WHEEL
            input.u.mi.dwExtraInfo = IntPtr.Zero;
            return One(input);
        }
    }
}
'@
    Add-Type -TypeDefinition $inputHelpers -Language CSharp

    Invoke-Step 'input' {
        $park = Get-ScreenCenter
        $mark = Restart-MelCat -Intensity 'Gentle' -Park $park
        foreach ($kind in @('key', 'wheel')) {
            $m = Get-LogMark
            $waited = [MelCatSmoke.Stroll.Mouse]::WaitForMove(15000)
            Assert ($waited -ge 0) "no stroll started within 15 s (before the $kind)"
            Start-Sleep -Milliseconds 60
            $at = [MelCatSmoke.Stroll.Mouse]::Cursor()
            $sentAt = [DateTime]::Now
            $sentMs = [MelCatSmoke.Stroll.Sampler]::Now()
            if ($kind -eq 'key') {
                $ok = [MelCatSmoke.Input.Send]::Tap(0x7C, 30)  # F13: down, 30 ms, up
                $what = 'F13 pressed and released'
            } else {
                $ok = [MelCatSmoke.Input.Send]::Wheel(120)  # one notch away from the user
                $what = 'one wheel notch'
            }
            if (-not $ok) { throw "SendInput failed for the ${kind}: $([MelCatSmoke.Input.Send]::LastError)" }
            $stop = Wait-Log 'stroll (stopped: |finished at )' $m 3000
            # Let go: no move from 200 ms after the input to 1.5 s after it
            # (one step may already be on its way when the input arrives).
            $rest = 1500 - ([MelCatSmoke.Stroll.Sampler]::Now() - $sentMs)
            if ($rest -gt 0) { Start-Sleep -Milliseconds $rest }
            $moves = @([MelCatSmoke.Stroll.Sampler]::Changes() | Where-Object { ($_.Ms -ge $sentMs + 200) -and ($_.Ms -le $sentMs + 1500) })
            $events = @(Get-StrollEvents $m)
            Note "${kind}: log: $(Format-StrollEvents $events)"
            Assert ($null -ne $stop) "the stroll neither stopped nor finished within 3 s of the $kind"
            $delay = if ($events.Count -ge 2) { ($events[1].Time - $sentAt).TotalMilliseconds } else { [double]::NaN }
            Note ('{0}: {1} at {2}, 60 ms after the stroll''s first visible move; logged {3:N0} ms later: ''{4}''; moves 0.2 to 1.5 s later: {5}' -f $kind, $what, $at, $delay, (Get-LogText $stop), $moves.Count)
            Assert ($stop -match 'stroll stopped: input$') "the $kind did not stop the stroll: '$(Get-LogText $stop)'"
            Assert ((Get-StrollKinds $events) -eq 'started stopped') "expected one stroll, stopped by the ${kind}: $(Get-StrollKinds $events)"
            Assert ($moves.Count -eq 0) "MelCat moved the pointer after the $kind stopped the stroll: $($moves -join ' ')"
        }
        [MelCatSmoke.Stroll.Sampler]::Stop()
    }

    # =======================================================================
    # key-release: a key's release alone is not a person (macOS counts key
    # presses and modifier changes only), so the stroll plays out.

    Invoke-Step 'key-release' {
        $park = Get-ScreenCenter
        $mark = Restart-MelCat -Intensity 'Gentle' -Park $park
        $waited = [MelCatSmoke.Stroll.Mouse]::WaitForMove(15000)
        Assert ($waited -ge 0) 'no stroll started within 15 s of parking the pointer'
        Start-Sleep -Milliseconds 60
        $at = [MelCatSmoke.Stroll.Mouse]::Cursor()
        $sentAt = [DateTime]::Now
        $sentMs = [MelCatSmoke.Stroll.Sampler]::Now()
        if (-not [MelCatSmoke.Input.Send]::Key(0x7C, $true)) { throw "SendInput failed for the F13 release: $([MelCatSmoke.Input.Send]::LastError)" }
        $end = Wait-Log 'stroll (stopped: |finished at )' $mark 5000
        [MelCatSmoke.Stroll.Sampler]::Stop()
        $events = @(Get-StrollEvents $mark)
        $later = @([MelCatSmoke.Stroll.Sampler]::Changes() | Where-Object { $_.Ms -gt $sentMs + 50 })
        Note "log: $(Format-StrollEvents $events)"
        $delay = if ($events.Count -ge 2) { ($events[1].Time - $sentAt).TotalMilliseconds } else { [double]::NaN }
        Note ('a lone F13 release (no press before it) at {0}, 60 ms after the stroll''s first visible move; the stroll ended {1:N0} ms later: ''{2}''; moves after the release: {3}' -f $at, $delay, $(if ($end) { Get-LogText $end } else { 'no end within 5 s' }), $later.Count)
        Assert ($null -ne $end) 'the stroll neither stopped nor finished within 5 s of the F13 release'
        Assert ($end -match 'stroll finished at ') "a lone key release stopped the stroll: '$(Get-LogText $end)'"
        Assert ((Get-StrollKinds $events) -eq 'started finished') "expected one stroll that plays out: $(Get-StrollKinds $events)"
    }

    # =======================================================================
    # off

    Invoke-Step 'off' {
        $park = Get-ScreenCenter
        $mark = Restart-MelCat -Mode 'Off' -Intensity 'Gentle' -Park $park
        $status = @(Get-LogLines 'status: ' $mark)
        Start-Sleep -Seconds 12
        $idle = [MelCatSmoke.Stroll.Mouse]::IdleMs()
        [MelCatSmoke.Stroll.Sampler]::Stop()
        $changes = [MelCatSmoke.Stroll.Sampler]::Changes()
        $strolls = @(Get-LogLines 'stroll started' $mark)
        Note ('status lines: {0}; after 12 s: session idle {1:N1} s, {2} stroll(s) started, positions sampled: {3} ({4} reads)' -f (($status | ForEach-Object { "'$(Get-LogText $_)'" }) -join ', '), ($idle / 1000.0), $strolls.Count, ($changes -join ' '), [MelCatSmoke.Stroll.Sampler]::Reads)
        Assert (($status.Count -ge 1) -and ($status[0] -match 'status: off$')) "MelCat did not log 'status: off'"
        Assert ($idle -ge 12000) "the session was not idle for the whole 12 s ($idle ms)"
        Assert ($strolls.Count -eq 0) "a stroll started in mode Off: $(($strolls | ForEach-Object { Get-LogText $_ }) -join '; ')"
        Assert ($changes.Length -eq 1) "the pointer moved in mode Off: $($changes -join ' ')"
        # MelCat must still be there: a crash right after 'status: off' passes everything above.
        $code = if ($script:Mel.HasExited) { ', with code 0x{0:X8}' -f $script:Mel.ExitCode } else { '' }
        Assert (-not $script:Mel.HasExited) "MelCat exited during the 12 s in mode Off$code"
    }

    # =======================================================================
    # kill-mid-stroll: a WinForms button to click afterwards.

    $killHelpers = @'
using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;

namespace MelCatSmoke.Kill
{
    public static class Native
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Sequential)]
        public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Explicit)]
        public struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
        [StructLayout(LayoutKind.Sequential)]
        public struct INPUT { public uint type; public InputUnion u; }

        [DllImport("user32.dll", SetLastError = true)]
        public static extern uint SendInput(uint count, [In, MarshalAs(UnmanagedType.LPArray)] INPUT[] inputs, int size);
        [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
        [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int key);
        [StructLayout(LayoutKind.Sequential)]
        public struct POINT { public int X; public int Y; }
        [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hwnd, System.Text.StringBuilder name, int max);
        [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    }

    public static class Clicker
    {
        public static string LastError = "";

        static int Normalize(int value, int origin, int extent)
        {
            if (extent <= 0) return 0;
            long offset = Math.Max(0L, Math.Min((long)extent - 1, (long)value - origin));
            return (int)Math.Min(65535L, (offset * 65536 + 32768) / extent);
        }

        static bool Mouse(int x, int y, uint buttons)
        {
            int left = Native.GetSystemMetrics(76), top = Native.GetSystemMetrics(77);
            int width = Native.GetSystemMetrics(78), height = Native.GetSystemMetrics(79);
            Native.INPUT input = new Native.INPUT();
            input.type = 0;
            input.u.mi.dx = Normalize(x, left, width);
            input.u.mi.dy = Normalize(y, top, height);
            input.u.mi.dwFlags = buttons | 0x0001 | 0x8000 | 0x4000;  // + MOVE | ABSOLUTE | VIRTUALDESK
            input.u.mi.dwExtraInfo = IntPtr.Zero;
            uint inserted = Native.SendInput(1, new Native.INPUT[] { input }, Marshal.SizeOf(typeof(Native.INPUT)));
            if (inserted != 1)
            {
                LastError = "SendInput inserted " + inserted + " of 1 events, error " + Marshal.GetLastWin32Error();
                return false;
            }
            return true;
        }

        /// Move there, press the left button, hold `holdMs`, release.
        public static bool Click(int x, int y, int holdMs)
        {
            if (!Mouse(x, y, 0)) return false;
            Thread.Sleep(40);
            if (!Mouse(x, y, 0x0002)) return false;
            Thread.Sleep(holdMs);
            return Mouse(x, y, 0x0004);
        }

        /// The mouse buttons and modifier keys GetAsyncKeyState says are held, or "".
        public static string Held()
        {
            int[] keys = { 0x01, 0x02, 0x04, 0x05, 0x06, 0x10, 0x11, 0x12, 0x5B, 0x5C };
            string[] names = { "left", "right", "middle", "x1", "x2", "shift", "ctrl", "alt", "lwin", "rwin" };
            string held = "";
            for (int i = 0; i < keys.Length; i++)
            {
                if (Native.GetAsyncKeyState(keys[i]) < 0) held += (held.Length > 0 ? " " : "") + names[i];
            }
            return held;
        }
    }

    public class TestForm : Form
    {
        public Button Counter;

        public TestForm(Rectangle bounds)
        {
            Text = "MelCat smoke test";
            StartPosition = FormStartPosition.Manual;
            FormBorderStyle = FormBorderStyle.FixedSingle;
            MaximizeBox = false;
            MinimizeBox = false;
            TopMost = true;
            Bounds = bounds;
            Counter = new Button();
            Counter.Text = "Clicks: 0";
            Counter.SetBounds(10, 10, ClientSize.Width - 20, ClientSize.Height - 20);
            Counter.Click += delegate
            {
                int n = Interlocked.Increment(ref Ui.Clicks);
                Counter.Text = "Clicks: " + n;
            };
            Controls.Add(Counter);
        }
    }

    /// The test window, on its own STA thread with its own message loop, so
    /// the script can sleep and inject input while the window stays responsive.
    public static class Ui
    {
        public static int Clicks;
        public static TestForm Form;
        static string startError;

        public static string Start(int x, int y, int width, int height)
        {
            ManualResetEvent ready = new ManualResetEvent(false);
            Rectangle bounds = new Rectangle(x, y, width, height);
            Thread thread = new Thread(delegate()
            {
                try
                {
                    Application.EnableVisualStyles();
                    Form = new TestForm(bounds);
                    Form.Shown += delegate { ready.Set(); };
                    Application.Run(Form);
                }
                catch (Exception e)
                {
                    startError = e.ToString();
                    ready.Set();
                }
            });
            thread.SetApartmentState(ApartmentState.STA);
            thread.IsBackground = true;
            thread.Start();
            if (!ready.WaitOne(20000)) return "the test window did not appear within 20 s";
            Thread.Sleep(300);
            return startError;
        }

        /// Screen rectangle (l, t, r, b) of the button.
        public static int[] ButtonRect()
        {
            return (int[])Form.Invoke(new Func<int[]>(delegate
            {
                Rectangle r = Form.Counter.RectangleToScreen(Form.Counter.ClientRectangle);
                return new int[] { r.Left, r.Top, r.Right, r.Bottom };
            }));
        }

        /// True when the window at (x, y) is the test button.
        public static bool ButtonAt(int x, int y)
        {
            Native.POINT p = new Native.POINT();
            p.X = x;
            p.Y = y;
            IntPtr at = Native.WindowFromPoint(p);
            IntPtr button = (IntPtr)Form.Invoke(new Func<IntPtr>(delegate { return Form.Counter.Handle; }));
            return at == button;
        }

        /// The window at (x, y) and the test window's state, for evidence.
        public static string Describe(int x, int y)
        {
            Native.POINT p = new Native.POINT();
            p.X = x;
            p.Y = y;
            IntPtr at = Native.WindowFromPoint(p);
            System.Text.StringBuilder name = new System.Text.StringBuilder(256);
            Native.GetClassName(at, name, name.Capacity);
            uint pid;
            Native.GetWindowThreadProcessId(at, out pid);
            string form = (string)Form.Invoke(new Func<string>(delegate
            {
                return "form 0x" + Form.Handle.ToString("X") + " visible=" + Form.Visible + " topmost=" + Form.TopMost
                    + " bounds=" + Form.Bounds + " button 0x" + Form.Counter.Handle.ToString("X");
            }));
            return "window at " + x + "," + y + ": 0x" + at.ToString("X") + " " + name + " (pid " + pid + "); " + form;
        }

        /// Puts the test window on top again.
        public static void Raise()
        {
            Form.Invoke(new Action(delegate
            {
                Form.TopMost = false;
                Form.TopMost = true;
                Form.BringToFront();
                Form.Activate();
            }));
        }

        public static void Close()
        {
            try { if (Form != null) Form.Invoke(new Action(delegate { Form.Close(); })); } catch (Exception) { }
        }
    }
}
'@
    Add-Type -TypeDefinition $killHelpers -ReferencedAssemblies System.Windows.Forms, System.Drawing -Language CSharp

    Invoke-Step 'kill-mid-stroll' {
        $p = $script:Primary
        $startError = [MelCatSmoke.Kill.Ui]::Start($p.Right - 280, $p.Top + 40, 240, 120)
        if ($startError) { throw "the test window failed: $startError" }
        $button = [MelCatSmoke.Kill.Ui]::ButtonRect()
        $park = Get-ScreenCenter
        $mark = Restart-MelCat -Intensity 'Gentle' -Park $park
        $waited = [MelCatSmoke.Stroll.Mouse]::WaitForMove(15000)
        Assert ($waited -ge 0) 'no stroll started within 15 s of parking the pointer'
        Start-Sleep -Milliseconds 60
        $at = [MelCatSmoke.Stroll.Mouse]::Cursor()
        $melPid = $script:Mel.Id
        Stop-Process -Id $melPid -Force
        Assert ($script:Mel.WaitForExit(5000)) "MelCat (pid $melPid) still ran 5 s after Stop-Process -Force"
        $events = @(Get-StrollEvents $mark)
        Note "log: $(Format-StrollEvents $events)"
        Note ('killed MelCat (pid {0}, exit code 0x{1:X8}) with the pointer at {2}, 60 ms after the stroll''s first visible move' -f $melPid, $script:Mel.ExitCode, $at)
        Assert (($events.Count -ge 1) -and ($events[$events.Count - 1].Kind -eq 'started')) "MelCat was not in the middle of a stroll when it was killed: $(Get-StrollKinds $events)"

        # Nothing moves the pointer any more.
        Start-Sleep -Milliseconds 200
        $quietFrom = [MelCatSmoke.Stroll.Sampler]::Now()
        Start-Sleep -Milliseconds 800
        $moves = @([MelCatSmoke.Stroll.Sampler]::Changes() | Where-Object { $_.Ms -ge $quietFrom })
        Assert ($moves.Count -eq 0) "the pointer still moved after MelCat was killed: $($moves -join ' ')"

        # An injected move lands exactly where sent, and stays there.
        $target = @(($park[0] - 150), ($park[1] + 100))
        Invoke-Input ([MelCatSmoke.Stroll.Mouse]::Move($target[0], $target[1])) 'the move after the kill'
        $landed = Wait-Until { [MelCatSmoke.Stroll.Mouse]::CursorIs($target[0], $target[1]) } 500
        Start-Sleep -Milliseconds 500
        $stayed = [MelCatSmoke.Stroll.Mouse]::CursorIs($target[0], $target[1])
        [MelCatSmoke.Stroll.Sampler]::Stop()
        $held = [MelCatSmoke.Kill.Clicker]::Held()
        Note ('a move injected to {0},{1} left the pointer at {2} (landed {3}, still there 0.5 s later {4}); held after the kill: {5}' -f $target[0], $target[1], [MelCatSmoke.Stroll.Mouse]::Cursor(), $landed, $stayed, $(if ($held) { $held } else { 'nothing' }))
        Assert ($landed -and $stayed) "the move to $($target -join ',') did not land and stay exactly there"
        Assert ($held -eq '') "held after MelCat was killed: $held"

        # One click on the test button counts once.
        $cx = [int][Math]::Floor(($button[0] + $button[2]) / 2)
        $cy = [int][Math]::Floor(($button[1] + $button[3]) / 2)
        # The runner's terminal can sit above even a topmost window shown later.
        for ($try = 1; ($try -le 3) -and (-not [MelCatSmoke.Kill.Ui]::ButtonAt($cx, $cy)); $try++) {
            Note "the test button is covered: $([MelCatSmoke.Kill.Ui]::Describe($cx, $cy)); raising it"
            [MelCatSmoke.Kill.Ui]::Raise()
            Start-Sleep -Milliseconds 300
        }
        Assert ([MelCatSmoke.Kill.Ui]::ButtonAt($cx, $cy)) "the test button is not under $cx,${cy}: $([MelCatSmoke.Kill.Ui]::Describe($cx, $cy))"
        $before = [MelCatSmoke.Kill.Ui]::Clicks
        if (-not [MelCatSmoke.Kill.Clicker]::Click($cx, $cy, 40)) { throw "SendInput failed for the click: $([MelCatSmoke.Kill.Clicker]::LastError)" }
        [void](Wait-Until { [MelCatSmoke.Kill.Ui]::Clicks -ge $before + 1 } 3000)
        Start-Sleep -Milliseconds 700  # time for a duplicate click to show up
        $after = [MelCatSmoke.Kill.Ui]::Clicks
        $held = [MelCatSmoke.Kill.Clicker]::Held()
        Note ('one click on the test button at {0},{1}: clicks {2} -> {3}; held afterwards: {4}' -f $cx, $cy, $before, $after, $(if ($held) { $held } else { 'nothing' }))
        Assert ($after -eq $before + 1) "one click on the test button counted $($after - $before) times"
        Assert ($held -eq '') "held after the click: $held"
        [MelCatSmoke.Kill.Ui]::Close()
    }

    # =======================================================================
    # held-button: a button held during a stroll is a hand on the mouse.

    $jiggleHeldButtonHelpers = @'
using System;
using System.Runtime.InteropServices;

namespace MelCatSmoke.JiggleHeldButton
{
    public static class Native
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Sequential)]
        public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
        [StructLayout(LayoutKind.Explicit)]
        public struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
        [StructLayout(LayoutKind.Sequential)]
        public struct INPUT { public uint type; public InputUnion u; }

        [DllImport("user32.dll", SetLastError = true)]
        public static extern uint SendInput(uint count, [In, MarshalAs(UnmanagedType.LPArray)] INPUT[] inputs, int size);
    }

    public static class Button
    {
        public static string LastError = "";

        /// The left button pressed (or released) where the pointer is, like
        /// a person's (dwExtraInfo 0); no MOUSEEVENTF_MOVE, so the pointer stays put.
        public static bool Left(bool down)
        {
            Native.INPUT input = new Native.INPUT();
            input.type = 0;
            input.u.mi.dwFlags = down ? 0x0002u : 0x0004u;  // LEFTDOWN or LEFTUP
            input.u.mi.dwExtraInfo = IntPtr.Zero;
            uint inserted = Native.SendInput(1, new Native.INPUT[] { input }, Marshal.SizeOf(typeof(Native.INPUT)));
            if (inserted != 1)
            {
                LastError = "SendInput inserted " + inserted + " of 1 events, error " + Marshal.GetLastWin32Error();
                return false;
            }
            return true;
        }
    }
}
'@
    Add-Type -TypeDefinition $jiggleHeldButtonHelpers -Language CSharp

    Invoke-Step 'held-button' {
        # The press lands on the script's own test button, never on the
        # runner's terminal.
        [MelCatSmoke.Kill.Ui]::Close()
        $p = $script:Primary
        $startError = [MelCatSmoke.Kill.Ui]::Start($p.Right - 280, $p.Top + 40, 240, 120)
        if ($startError) { throw "the test window failed: $startError" }
        $button = [MelCatSmoke.Kill.Ui]::ButtonRect()
        $cx = [int][Math]::Floor(($button[0] + $button[2]) / 2)
        $cy = [int][Math]::Floor(($button[1] + $button[3]) / 2)
        for ($try = 1; ($try -le 3) -and (-not [MelCatSmoke.Kill.Ui]::ButtonAt($cx, $cy)); $try++) {
            Note "the test button is covered: $([MelCatSmoke.Kill.Ui]::Describe($cx, $cy)); raising it"
            [MelCatSmoke.Kill.Ui]::Raise()
            Start-Sleep -Milliseconds 300
        }
        Assert ([MelCatSmoke.Kill.Ui]::ButtonAt($cx, $cy)) "the test button is not under $cx,${cy}: $([MelCatSmoke.Kill.Ui]::Describe($cx, $cy))"
        $mark = Restart-MelCat -Intensity 'Gentle' -Park @($cx, $cy)
        $waited = [MelCatSmoke.Stroll.Mouse]::WaitForMove(15000)
        Assert ($waited -ge 0) 'no stroll started within 15 s of parking the pointer'
        Start-Sleep -Milliseconds 60
        $at = [MelCatSmoke.Stroll.Mouse]::Cursor()
        $xy = @($at -split ',' | ForEach-Object { [int]$_ })
        Assert ([MelCatSmoke.Kill.Ui]::ButtonAt($xy[0], $xy[1])) "the pointer at $at is not over the test button: $([MelCatSmoke.Kill.Ui]::Describe($xy[0], $xy[1]))"
        $sentAt = [DateTime]::Now
        $sentMs = [MelCatSmoke.Stroll.Sampler]::Now()
        if (-not [MelCatSmoke.JiggleHeldButton.Button]::Left($true)) { throw "SendInput failed for the press: $([MelCatSmoke.JiggleHeldButton.Button]::LastError)" }
        Start-Sleep -Milliseconds 100
        if (-not [MelCatSmoke.JiggleHeldButton.Button]::Left($false)) { throw "SendInput failed for the release: $([MelCatSmoke.JiggleHeldButton.Button]::LastError)" }
        $stop = Wait-Log 'stroll (stopped: |finished at )' $mark 3000
        $rest = 1500 - ([MelCatSmoke.Stroll.Sampler]::Now() - $sentMs)
        if ($rest -gt 0) { Start-Sleep -Milliseconds $rest }
        [MelCatSmoke.Stroll.Sampler]::Stop()
        $moves = @([MelCatSmoke.Stroll.Sampler]::Changes() | Where-Object { ($_.Ms -ge $sentMs + 200) -and ($_.Ms -le $sentMs + 1500) })
        $events = @(Get-StrollEvents $mark)
        $held = [MelCatSmoke.Kill.Clicker]::Held()
        [MelCatSmoke.Kill.Ui]::Close()
        Note "log: $(Format-StrollEvents $events)"
        $delay = if ($events.Count -ge 2) { ($events[1].Time - $sentAt).TotalMilliseconds } else { [double]::NaN }
        Note ('left button held 100 ms at {0} (over the test button), 60 ms after the stroll''s first visible move; logged {1:N0} ms after the press: ''{2}''; moves 0.2 to 1.5 s later: {3}; held afterwards: {4}' -f $at, $delay, $(if ($stop) { Get-LogText $stop } else { 'nothing' }), $moves.Count, $(if ($held) { $held } else { 'nothing' }))
        Assert ($null -ne $stop) 'the stroll neither stopped nor finished within 3 s of the press'
        Assert ($stop -match 'stroll stopped: pointer moved$') "a held button did not stop the stroll as a hand on the mouse: '$(Get-LogText $stop)'"
        Assert ((Get-StrollKinds $events) -eq 'started stopped') "expected one stroll, stopped by the held button: $(Get-StrollKinds $events)"
        Assert ($moves.Count -eq 0) "MelCat moved the pointer after the held button stopped the stroll: $($moves -join ' ')"
        Assert ($held -eq '') "held after the release: $held"
    }

    # =======================================================================
    # first-launch

    $firstLaunchHelpers = @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;

namespace MelCatSmoke.FirstLaunch
{
    public static class Win
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
        public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lparam);
        [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc proc, IntPtr lparam);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder name, int max);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
        [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool IsWindow(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
        [DllImport("user32.dll", SetLastError = true)] static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wparam, IntPtr lparam);

        static string ClassOf(IntPtr hwnd)
        {
            StringBuilder name = new StringBuilder(256);
            GetClassName(hwnd, name, name.Capacity);
            return name.ToString();
        }

        /// The first visible top-level window of class `cls` that belongs to process `pid`, or zero.
        public static IntPtr FindVisibleOwned(string cls, int pid)
        {
            IntPtr result = IntPtr.Zero;
            EnumWindows(delegate(IntPtr hwnd, IntPtr unused)
            {
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner == (uint)pid && IsWindowVisible(hwnd) && ClassOf(hwnd) == cls) { result = hwnd; return false; }
                return true;
            }, IntPtr.Zero);
            return result;
        }

        /// "class visible=.. rect=.." for every top-level window of process `pid`.
        public static string Describe(int pid)
        {
            StringBuilder all = new StringBuilder();
            EnumWindows(delegate(IntPtr hwnd, IntPtr unused)
            {
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner == (uint)pid)
                {
                    RECT r;
                    GetWindowRect(hwnd, out r);
                    if (all.Length > 0) all.Append("; ");
                    all.Append(ClassOf(hwnd) + " visible=" + IsWindowVisible(hwnd) + " rect=" + r.Left + "," + r.Top + "," + r.Right + "," + r.Bottom);
                }
                return true;
            }, IntPtr.Zero);
            return all.Length > 0 ? all.ToString() : "none";
        }

        public static bool Exists(IntPtr hwnd) { return IsWindow(hwnd); }

        public static int[] Rect(IntPtr hwnd)
        {
            RECT r;
            GetWindowRect(hwnd, out r);
            return new int[] { r.Left, r.Top, r.Right, r.Bottom };
        }

        public static string Title(IntPtr hwnd)
        {
            StringBuilder text = new StringBuilder(256);
            GetWindowText(hwnd, text, text.Capacity);
            return text.ToString();
        }

        /// Posts WM_CLOSE; "" or the error.
        public static string Close(IntPtr hwnd)
        {
            if (PostMessage(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero)) return "";
            return "PostMessage error " + Marshal.GetLastWin32Error();
        }

        /// Saves the screen rectangle (l, t, r, b) to a PNG; "" or the error.
        public static string Shot(string path, int[] rect)
        {
            try
            {
                int width = Math.Max(1, rect[2] - rect[0]), height = Math.Max(1, rect[3] - rect[1]);
                using (Bitmap bitmap = new Bitmap(width, height))
                using (Graphics g = Graphics.FromImage(bitmap))
                {
                    g.CopyFromScreen(rect[0], rect[1], 0, 0, bitmap.Size);
                    bitmap.Save(path, ImageFormat.Png);
                }
                return "";
            }
            catch (Exception e) { return e.Message; }
        }
    }
}
'@
    Add-Type -TypeDefinition $firstLaunchHelpers -ReferencedAssemblies System.Drawing -Language CSharp

    # Enter on the settings window's Preview button (used by 'first-launch').
    $jigglePreviewEnterHelpers = @'
using System;
using System.Runtime.InteropServices;

namespace MelCatSmoke.JigglePreviewEnter
{
    public static class Dialog
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
        [StructLayout(LayoutKind.Sequential)]
        public struct POINT { public int X; public int Y; }
        [StructLayout(LayoutKind.Sequential)]
        public struct GUITHREADINFO
        {
            public uint cbSize; public uint flags;
            public IntPtr hwndActive; public IntPtr hwndFocus; public IntPtr hwndCapture;
            public IntPtr hwndMenuOwner; public IntPtr hwndMoveSize; public IntPtr hwndCaret;
            public RECT rcCaret;
        }
        [DllImport("user32.dll")] static extern IntPtr GetDlgItem(IntPtr window, int id);
        [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
        [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(POINT point);
        [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("user32.dll")] static extern bool GetGUIThreadInfo(uint thread, ref GUITHREADINFO info);
        [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();

        /// The child window of `window` with control id `id`, or zero.
        public static IntPtr Item(IntPtr window, int id) { return GetDlgItem(window, id); }

        /// The screen point in the middle of `hwnd`.
        public static int[] Center(IntPtr hwnd)
        {
            RECT r;
            GetWindowRect(hwnd, out r);
            return new int[] { (r.Left + r.Right) / 2, (r.Top + r.Bottom) / 2 };
        }

        /// True when `hwnd` is the window at (x, y).
        public static bool At(IntPtr hwnd, int x, int y)
        {
            POINT p = new POINT();
            p.X = x;
            p.Y = y;
            return WindowFromPoint(p) == hwnd;
        }

        /// The window with the keyboard focus on the thread of `window`, or zero.
        public static IntPtr Focus(IntPtr window)
        {
            uint pid;
            uint thread = GetWindowThreadProcessId(window, out pid);
            GUITHREADINFO info = new GUITHREADINFO();
            info.cbSize = (uint)Marshal.SizeOf(typeof(GUITHREADINFO));
            if (!GetGUIThreadInfo(thread, ref info)) return IntPtr.Zero;
            return info.hwndFocus;
        }

        public static IntPtr Foreground() { return GetForegroundWindow(); }
    }
}
'@
    Add-Type -TypeDefinition $jigglePreviewEnterHelpers -Language CSharp

    Invoke-Step 'first-launch' {
        Stop-MelCatCleanly
        foreach ($path in @($SettingsPath, "$SettingsPath.tmp")) { Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue }
        Assert (-not (Test-Path -LiteralPath $SettingsPath)) "could not remove $SettingsPath"
        $mark = Get-LogMark
        $script:Mel = Start-MelCat
        $hwnd = [IntPtr]::Zero
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        do {
            $hwnd = [MelCatSmoke.FirstLaunch.Win]::FindVisibleOwned('MelCat.Settings', $script:Mel.Id)
            if ($hwnd -ne [IntPtr]::Zero) { break }
            Start-Sleep -Milliseconds 100
        } while ($watch.ElapsedMilliseconds -lt 10000)
        Assert ($hwnd -ne [IntPtr]::Zero) "no visible MelCat.Settings window of MelCat (pid $($script:Mel.Id)) within 10 s; its windows: $([MelCatSmoke.FirstLaunch.Win]::Describe($script:Mel.Id))"
        $rect = [MelCatSmoke.FirstLaunch.Win]::Rect($hwnd)
        Note ("visible MelCat.Settings window 0x{0:X} of MelCat (pid {1}) after {2} ms: '{3}' at {4}" -f $hwnd.ToInt64(), $script:Mel.Id, $watch.ElapsedMilliseconds, [MelCatSmoke.FirstLaunch.Win]::Title($hwnd), ($rect -join ','))
        Start-Sleep -Milliseconds 700  # let it paint
        Save-Desktop 'settings-window'
        $shot = Join-Path $ArtifactsDir 'settings-window.png'
        $err = [MelCatSmoke.FirstLaunch.Win]::Shot($shot, $rect)
        if ($err) { Note "settings window screenshot failed: $err" } else { Note "settings window screenshot: settings-window.png" }

        # --- Enter on a focused Preview plays a whole preview ----------------
        # A click on Preview gives it the focus and plays one preview. Enter
        # then presses Preview on its key-down; the release of the same key,
        # about 100 ms into the new preview, must not stop it.
        $preview = [MelCatSmoke.JigglePreviewEnter.Dialog]::Item($hwnd, 135)
        Assert ($preview -ne [IntPtr]::Zero) 'the settings window has no Preview button (control id 135)'
        $center = [MelCatSmoke.JigglePreviewEnter.Dialog]::Center($preview)
        Assert ([MelCatSmoke.JigglePreviewEnter.Dialog]::At($preview, $center[0], $center[1])) "the Preview button is covered at $($center -join ',')"
        $clickMark = Get-LogMark
        if (-not [MelCatSmoke.Kill.Clicker]::Click($center[0], $center[1], 40)) { throw "SendInput failed for the click on Preview: $([MelCatSmoke.Kill.Clicker]::LastError)" }
        $clicked = Wait-Log 'stroll (stopped: |finished at )' $clickMark 6000
        Note "a click on Preview at $($center -join ','): log: $(Format-StrollEvents @(Get-StrollEvents $clickMark))"
        Assert ($null -ne $clicked) 'a click on Preview played no preview that ended within 6 s'
        $focused = Wait-Until { ([MelCatSmoke.JigglePreviewEnter.Dialog]::Focus($hwnd) -eq $preview) -and ([MelCatSmoke.JigglePreviewEnter.Dialog]::Foreground() -eq $hwnd) } 2000
        Assert $focused ('Preview does not have the focus after the click: focus 0x{0:X}, foreground 0x{1:X}, Preview 0x{2:X}' -f [MelCatSmoke.JigglePreviewEnter.Dialog]::Focus($hwnd).ToInt64(), [MelCatSmoke.JigglePreviewEnter.Dialog]::Foreground().ToInt64(), $preview.ToInt64())
        $enterMark = Get-LogMark
        if (-not [MelCatSmoke.Input.Send]::Tap(0x0D, 100)) { throw "SendInput failed for Enter: $([MelCatSmoke.Input.Send]::LastError)" }
        $entered = Wait-Log 'stroll (stopped: |finished at )' $enterMark 6000
        $enterEvents = @(Get-StrollEvents $enterMark)
        Note "Enter held 100 ms on the focused Preview: log: $(Format-StrollEvents $enterEvents)"
        Assert ([MelCatSmoke.FirstLaunch.Win]::Exists($hwnd)) 'Enter closed the settings window instead of pressing Preview'
        Assert ($null -ne $entered) 'Enter on Preview played no preview that ended within 6 s'
        Assert ($entered -match 'stroll finished at ') "the release of Enter stopped the preview it started: '$(Get-LogText $entered)'"
        Assert ((Get-StrollKinds $enterEvents) -eq 'started finished') "expected one preview that plays out: $(Get-StrollKinds $enterEvents)"
        # ---------------------------------------------------------------------

        $err = [MelCatSmoke.FirstLaunch.Win]::Close($hwnd)
        Assert ($err -eq '') "could not post WM_CLOSE to the settings window: $err"
        $closed = Wait-Until { -not [MelCatSmoke.FirstLaunch.Win]::Exists($hwnd) } 5000
        $lines = @(Get-LogLines 'settings window (opened|closed)$' $mark)
        Note "log: $(($lines | ForEach-Object { "'$(Get-LogText $_)'" }) -join ', ')"
        Assert $closed 'the settings window was still there 5 s after WM_CLOSE'
        Assert (Test-Path -LiteralPath $SettingsPath) "$SettingsPath does not exist after the settings window closed"
        $json = [System.IO.File]::ReadAllText($SettingsPath)
        Note "settings.json: $($json -replace '\s+', ' ')"
        $parsed = $json | ConvertFrom-Json
        Assert ($parsed.mode -eq 'Scheduled') "settings.json does not hold the default mode (Scheduled): $($parsed.mode)"
        Assert (-not $script:Mel.HasExited) 'MelCat quit when its settings window closed'

        # A clean quit: WM_CLOSE to MelCat.Main ends MelCat within 5 s with
        # exit code 0, it logs 'MelCat stopped', and its tray icon is gone.
        $mel = $script:Mel
        $main = [MelCatSmoke.Start.Win]::FindOwned('MelCat.Main', $mel.Id)
        Assert ($main -ne [IntPtr]::Zero) "no MelCat.Main window belongs to MelCat (pid $($mel.Id))"
        $iconBefore = [MelCatSmoke.Win]::TrayIcon($main, 1)
        $quit = Invoke-MelCatQuit
        $stopped = Wait-Log 'MelCat stopped$' $mark 3000
        $iconAfter = [MelCatSmoke.Win]::TrayIcon($main, 1)
        $code = if ($mel.HasExited) { '0x{0:X8}' -f $mel.ExitCode } else { 'none (still running)' }
        Note ('asked MelCat to quit (WM_CLOSE to MelCat.Main): {0}; exit code {1}; log: {2}; tray icon before: {3}, after: {4}' -f $(if ($quit) { $quit } else { 'quit within 5 s' }), $code, $(if ($stopped) { "'$(Get-LogText $stopped)'" } else { 'no ''MelCat stopped''' }), $iconBefore, $iconAfter)
        Assert ($quit -eq '') "MelCat did not quit cleanly on WM_CLOSE: $quit"
        Assert ($mel.HasExited -and ($mel.ExitCode -eq 0)) "MelCat's exit code is $code, not 0"
        Assert ($null -ne $stopped) "MelCat logged no 'MelCat stopped' on its way out"
        Assert ($iconBefore -match '^rect=') "the shell did not know MelCat's tray icon before it quit: $iconBefore"
        Assert ($iconAfter -match '^error ') "MelCat's tray icon outlived it (no NIM_DELETE): Shell_NotifyIconGetRect says $iconAfter"
    }

    # =======================================================================
    # settings-bom, settings-reload, save-failure: settings.json as other
    # programs write it, edits while MelCat runs, and a save that fails
    # (MelCatSmoke.AppSettings).

    $appSettingsHelpers = @'
using System;
using System.Runtime.InteropServices;

namespace MelCatSmoke.AppSettings
{
    public static class Win
    {
        static readonly IntPtr HWND_MESSAGE = new IntPtr(-3);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
        [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("user32.dll", SetLastError = true)] static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wparam, IntPtr lparam);

        /// The message-only MelCat.Msg window of process `pid`, or zero.
        public static IntPtr FindMsg(int pid)
        {
            IntPtr hwnd = IntPtr.Zero;
            while ((hwnd = FindWindowEx(HWND_MESSAGE, hwnd, "MelCat.Msg", null)) != IntPtr.Zero)
            {
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner == (uint)pid) return hwnd;
            }
            return IntPtr.Zero;
        }

        /// Posts `msg` without parameters; "" or the error.
        public static string Post(IntPtr hwnd, uint msg)
        {
            if (PostMessage(hwnd, msg, IntPtr.Zero, IntPtr.Zero)) return "";
            return "PostMessage error " + Marshal.GetLastWin32Error();
        }
    }
}
'@
    Add-Type -TypeDefinition $appSettingsHelpers -Language CSharp

    # WM_APP_SETTINGS_CHANGED (WM_APP + 2) to MelCat's MelCat.Msg window: the
    # controller saves the settings it holds, as after a settings window change.
    function Send-MelSave {
        $msg = [MelCatSmoke.AppSettings.Win]::FindMsg($script:Mel.Id)
        Assert ($msg -ne [IntPtr]::Zero) "no MelCat.Msg window belongs to MelCat (pid $($script:Mel.Id))"
        $err = [MelCatSmoke.AppSettings.Win]::Post($msg, 0x8002)
        Assert ($err -eq '') "could not post WM_APP_SETTINGS_CHANGED to MelCat.Msg: $err"
    }

    # A log line's time stamp.
    function Get-LogTime([string]$Line) {
        return [datetime]::ParseExact($Line.Substring(0, 23), 'yyyy-MM-dd HH:mm:ss.fff', $script:Invariant)
    }

    # Starts MelCat with settings.json as it is and waits for its first status
    # line. Returns the log mark taken just before the start.
    function Start-MelCatAsIs {
        $mark = Get-LogMark
        $script:Mel = Start-MelCat
        $status = Wait-Log 'status: (off|napping|active)$' $mark 15000
        Assert ($null -ne $status) "MelCat (pid $($script:Mel.Id)) logged no status within 15 s"
        return $mark
    }

    Invoke-Step 'settings-bom' {
        $json = '{"mode":"Off","days":[1,7],"start_minutes":540,"end_minutes":1080,"intensity":"Playful","idle_interval_secs":5}'
        $cases = @(
            @('UTF-8 with a byte order mark', (New-Object System.Text.UTF8Encoding $true), 'EF-BB-BF'),
            @('UTF-16 LE with a byte order mark', [System.Text.Encoding]::Unicode, 'FF-FE'),
            @('UTF-16 BE with a byte order mark', [System.Text.Encoding]::BigEndianUnicode, 'FE-FF'))
        foreach ($case in $cases) {
            $what = $case[0]
            Stop-MelCatCleanly
            New-Item -ItemType Directory -Force -Path (Split-Path $SettingsPath) | Out-Null
            [System.IO.File]::WriteAllText($SettingsPath, $json, $case[1])
            $bytes = [System.IO.File]::ReadAllBytes($SettingsPath)
            $head = [BitConverter]::ToString($bytes, 0, [Math]::Min(4, $bytes.Length))
            Assert ($head.StartsWith($case[2])) "settings.json written as $what starts with $head, not $($case[2])"
            $mark = Start-MelCatAsIs
            $status = @(Get-LogLines 'status: ' $mark)[0]
            $loaded = @(Get-LogLines 'settings: Settings ' $mark)
            $shown = if ($loaded.Count -gt 0) { Get-LogText $loaded[0] } else { 'no settings line' }
            Note ("{0} ({1} ...): '{2}'; first status '{3}'" -f $what, $head, $shown, (Get-LogText $status))
            Assert ($loaded.Count -ge 1) "MelCat logged no 'settings:' line"
            Assert (($loaded[0] -match 'mode: Off,') -and ($loaded[0] -match 'days: Days\(65\)') -and ($loaded[0] -match 'intensity: Playful,') -and ($loaded[0] -match 'idle_interval: 5\.0 ')) "MelCat did not load settings.json written as ${what}: '$shown'"
            Assert ($status -match 'status: off$') "with mode Off in settings.json written as $what, MelCat's first status is '$(Get-LogText $status)'"
        }

        # A file that is not JSON: the defaults, and a warning in the log.
        Stop-MelCatCleanly
        [System.IO.File]::WriteAllText($SettingsPath, 'not json')
        $mark = Start-MelCatAsIs
        $warning = @(Get-LogLines 'settings\.json is not a JSON object; using the defaults$' $mark)
        $loaded = @(Get-LogLines 'settings: Settings ' $mark)
        Note ("'not json': warning {0}; '{1}'" -f $(if ($warning.Count -gt 0) { "'$(Get-LogText $warning[0])'" } else { 'none' }), $(if ($loaded.Count -gt 0) { Get-LogText $loaded[0] } else { 'no settings line' }))
        Assert ($warning.Count -ge 1) 'a settings.json that is not JSON left no warning in the log'
        Assert (($loaded.Count -ge 1) -and ($loaded[0] -match 'mode: Scheduled,')) 'with a settings.json that is not JSON, MelCat did not use the defaults'
    }

    Invoke-Step 'settings-reload' {
        $mark = Restart-MelCat -Mode 'Always' -Intensity 'Gentle' -Park (Get-ScreenCenter)
        $first = @(Get-LogLines 'status: ' $mark)[0]
        Assert ($first -match 'status: active$') "with mode Always MelCat's first status is '$(Get-LogText $first)'"

        # Edited while MelCat runs: it applies within a second or so.
        $edit = Get-LogMark
        Write-MelSettings 'Off' 'Gentle'
        $editedAt = [DateTime]::Now
        $off = Wait-Log 'status: off$' $edit 3000
        $reloads = @(Get-LogLines 'settings reloaded: ' $edit)
        $after = if ($off) { '{0:N0} ms' -f ((Get-LogTime $off) - $editedAt).TotalMilliseconds } else { '-' }
        Note ("settings.json edited to mode Off: {0}; then '{1}', {2} after the edit" -f $(if ($reloads.Count -gt 0) { "'$(Get-LogText $reloads[0])'" } else { "no 'settings reloaded'" }), $(if ($off) { Get-LogText $off } else { "no 'status: off' within 3 s" }), $after)
        Assert ($null -ne $off) "MelCat did not switch to 'status: off' within 3 s of settings.json being edited to mode Off"
        Assert (($reloads.Count -ge 1) -and ($reloads[0] -match 'mode: Off,')) "MelCat logged no 'settings reloaded: ... mode: Off'"

        # Nothing moves from then on.
        $afterOff = $edit + (Get-LogSince $edit).IndexOf($off)
        $offMs = [MelCatSmoke.Stroll.Sampler]::MsAt((Get-LogTime $off))
        Start-Sleep -Seconds 8
        [MelCatSmoke.Stroll.Sampler]::Stop()
        $strolls = @(Get-LogLines 'stroll started' $afterOff)
        $moves = @([MelCatSmoke.Stroll.Sampler]::Changes() | Where-Object { $_.Ms -gt $offMs + 100 })
        Note ("in the 8 s after 'status: off': {0} stroll(s) started; pointer moves: {1}" -f $strolls.Count, $(if ($moves.Count -gt 0) { $moves -join ' ' } else { 'none' }))
        Assert ($strolls.Count -eq 0) "a stroll started after settings.json was edited to mode Off: $(($strolls | ForEach-Object { Get-LogText $_ }) -join '; ')"
        Assert ($moves.Count -eq 0) "the pointer moved after settings.json was edited to mode Off: $($moves -join ' ')"
        Assert (-not $script:Mel.HasExited) 'MelCat exited after reloading its settings'

        # MelCat's own save keeps the edit and is not taken for another edit.
        $save = Get-LogMark
        $before = [MelCatSmoke.Win]::ReadShared($SettingsPath)
        Send-MelSave
        $saved = Wait-Until { [MelCatSmoke.Win]::ReadShared($SettingsPath) -ne $before } 3000
        Start-Sleep -Milliseconds 2500  # two ticks or more
        $json = [MelCatSmoke.Win]::ReadShared($SettingsPath)
        $again = @(Get-LogLines 'settings reloaded' $save)
        Note ("after WM_APP_SETTINGS_CHANGED: settings.json {0}: {1}; 'settings reloaded' lines since: {2}" -f $(if ($saved) { 'rewritten' } else { 'unchanged' }), ($json -replace '\s+', ' '), $again.Count)
        Assert $saved 'MelCat did not save settings.json within 3 s of WM_APP_SETTINGS_CHANGED'
        $mode = ($json | ConvertFrom-Json).mode
        Assert ($mode -eq 'Off') "MelCat saved mode '$mode' over the edited mode Off"
        Assert ($again.Count -eq 0) "MelCat took its own save for an edit: $(($again | ForEach-Object { Get-LogText $_ }) -join '; ')"
    }

    Invoke-Step 'save-failure' {
        $tmp = "$SettingsPath.tmp"
        $mark = Restart-MelCat -Mode 'Off' -Intensity 'Gentle'
        [MelCatSmoke.Stroll.Sampler]::Stop()
        # MelCat writes settings.json.tmp, then renames it; a folder of that name makes the write fail.
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        try {
            Send-MelSave
            $line = Wait-Log 'notice: Could not save the settings: ' $mark 5000
            $logged = @(Get-LogLines 'could not save the settings: ' $mark)
            Assert ($null -ne $line) "no 'notice: Could not save the settings: ...' within 5 s of a save with $tmp in the way"
            $text = (Get-LogText $line) -replace '^notice: Could not save the settings: ', ''
            Note ("log: '{0}'; balloon: '{1}'" -f $(if ($logged.Count -gt 0) { Get-LogText $logged[0] } else { '-' }), $text)
            Assert ($text -notmatch '\. \(') "the balloon has a period in the middle of a sentence: '$text'"
            Assert ($text -notmatch '\.\.') "the balloon has a double period: '$text'"
            Assert ($text -cmatch '^\p{Lu}') "the balloon does not start with a capital letter: '$text'"
            Assert ($text -cmatch ' \(error \d+\)\. The change applies until MelCat quits\.$') "the balloon does not end with ' (error <code>). The change applies until MelCat quits.': '$text'"
            Assert (-not $script:Mel.HasExited) 'MelCat exited after failing to save'
        } finally {
            Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    # =======================================================================
    # settings-preview-label, settings-repaint, settings-short-time: the
    # settings window's page, each on a first launch of its own.

    $uiSettingsHelpers = @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;

namespace MelCatSmoke.UiSettings
{
    /// A direct child window: its control ID, class, text and screen rectangle.
    public class Child
    {
        public IntPtr Hwnd;
        public int Id;
        public string Class;
        public string Text;
        public int Left, Top, Right, Bottom;
        public bool Visible;
        public override string ToString()
        {
            return Id + " " + Class + " '" + Text + "' " + Left + "," + Top + "," + Right + "," + Bottom;
        }
    }

    public static class Win
    {
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
        [StructLayout(LayoutKind.Sequential)]
        public struct POINT { public int X; public int Y; }
        [StructLayout(LayoutKind.Sequential)]
        public struct GUITHREADINFO
        {
            public uint cbSize; public uint flags; public IntPtr hwndActive; public IntPtr hwndFocus; public IntPtr hwndCapture;
            public IntPtr hwndMenuOwner; public IntPtr hwndMoveSize; public IntPtr hwndCaret; public RECT rcCaret;
        }
        public delegate bool EnumProc(IntPtr hwnd, IntPtr lparam);
        [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr lparam);
        [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent, EnumProc proc, IntPtr lparam);
        [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder name, int max);
        [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
        [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool IsWindow(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
        [DllImport("user32.dll")] static extern IntPtr GetDlgItem(IntPtr parent, int id);
        [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr hwnd);
        [DllImport("user32.dll", EntryPoint = "SendMessageTimeoutW", CharSet = CharSet.Unicode)]
        static extern IntPtr SendText(IntPtr hwnd, uint msg, IntPtr wparam, StringBuilder lparam, uint flags, uint timeout, out IntPtr result);
        [DllImport("user32.dll", EntryPoint = "SendMessageTimeoutW")]
        static extern IntPtr SendPlain(IntPtr hwnd, uint msg, IntPtr wparam, IntPtr lparam, uint flags, uint timeout, out IntPtr result);
        [DllImport("user32.dll", SetLastError = true)] static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr wparam, IntPtr lparam);
        [DllImport("user32.dll")] static extern bool GetGUIThreadInfo(uint thread, ref GUITHREADINFO info);
        [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(POINT point);
        [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
        [DllImport("user32.dll")] static extern bool RedrawWindow(IntPtr hwnd, IntPtr rect, IntPtr region, uint flags);
        [DllImport("kernel32.dll", EntryPoint = "GetLocaleInfoW", CharSet = CharSet.Unicode)]
        static extern int GetLocaleInfo(uint locale, uint type, StringBuilder data, int size);
        [DllImport("kernel32.dll", EntryPoint = "SetLocaleInfoW", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern bool SetLocaleInfo(uint locale, uint type, string data);

        const uint SMTO_ABORTIFHUNG = 0x0002;
        const uint LOCALE_USER_DEFAULT = 0x0400;

        static string ClassOf(IntPtr hwnd)
        {
            StringBuilder name = new StringBuilder(256);
            GetClassName(hwnd, name, name.Capacity);
            return name.ToString();
        }

        /// The first visible top-level window of class `cls` that belongs to process `pid`, or zero.
        public static IntPtr FindVisibleOwned(string cls, int pid)
        {
            IntPtr result = IntPtr.Zero;
            EnumWindows(delegate(IntPtr hwnd, IntPtr unused)
            {
                uint owner;
                GetWindowThreadProcessId(hwnd, out owner);
                if (owner == (uint)pid && IsWindowVisible(hwnd) && ClassOf(hwnd) == cls) { result = hwnd; return false; }
                return true;
            }, IntPtr.Zero);
            return result;
        }

        public static bool Exists(IntPtr hwnd) { return IsWindow(hwnd); }

        /// Posts WM_CLOSE; "" or the error.
        public static string Close(IntPtr hwnd)
        {
            if (PostMessage(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero)) return "";
            return "PostMessage error " + Marshal.GetLastWin32Error();
        }

        /// The window's text (WM_GETTEXT, which the system carries across processes).
        public static string Text(IntPtr hwnd)
        {
            StringBuilder text = new StringBuilder(1024);
            IntPtr result;
            if (SendText(hwnd, 0x000D, (IntPtr)text.Capacity, text, SMTO_ABORTIFHUNG, 3000, out result) == IntPtr.Zero) return "(no answer)";
            return text.ToString();
        }

        public static IntPtr Item(IntPtr parent, int id) { return GetDlgItem(parent, id); }

        public static int[] Rect(IntPtr hwnd)
        {
            RECT r;
            GetWindowRect(hwnd, out r);
            return new int[] { r.Left, r.Top, r.Right, r.Bottom };
        }

        /// The direct children of `parent`, in z-order (the order they were created).
        public static Child[] Children(IntPtr parent)
        {
            List<Child> all = new List<Child>();
            EnumChildWindows(parent, delegate(IntPtr hwnd, IntPtr unused)
            {
                if (GetAncestor(hwnd, 1 /* GA_PARENT */) != parent) return true;
                RECT r;
                GetWindowRect(hwnd, out r);
                Child c = new Child();
                c.Hwnd = hwnd; c.Id = GetDlgCtrlID(hwnd); c.Class = ClassOf(hwnd); c.Text = Text(hwnd);
                c.Left = r.Left; c.Top = r.Top; c.Right = r.Right; c.Bottom = r.Bottom; c.Visible = IsWindowVisible(hwnd);
                all.Add(c);
                return true;
            }, IntPtr.Zero);
            return all.ToArray();
        }

        /// The window with the keyboard focus on `hwnd`'s thread.
        public static IntPtr FocusOf(IntPtr hwnd)
        {
            uint pid;
            uint thread = GetWindowThreadProcessId(hwnd, out pid);
            GUITHREADINFO info = new GUITHREADINFO();
            info.cbSize = (uint)Marshal.SizeOf(typeof(GUITHREADINFO));
            if (!GetGUIThreadInfo(thread, ref info)) return IntPtr.Zero;
            return info.hwndFocus;
        }

        public static IntPtr Foreground() { return GetForegroundWindow(); }

        /// WM_NEXTDLGCTL: the dialog moves the focus to `control`.
        public static bool FocusControl(IntPtr dialog, IntPtr control)
        {
            IntPtr result;
            return SendPlain(dialog, 0x0028, control, (IntPtr)1, SMTO_ABORTIFHUNG, 3000, out result) != IntPtr.Zero;
        }

        /// Posts `count` presses (WM_KEYDOWN, WM_KEYUP) of an extended key to `control`.
        public static string PostKeys(IntPtr control, int vk, int scan, int count)
        {
            long down = 1L | ((long)scan << 16) | (1L << 24);
            long up = down | (1L << 30) | (1L << 31);
            for (int i = 0; i < count; i++)
            {
                if (!PostMessage(control, 0x0100, (IntPtr)vk, new IntPtr(down))) return "PostMessage error " + Marshal.GetLastWin32Error();
                if (!PostMessage(control, 0x0101, (IntPtr)vk, new IntPtr(up))) return "PostMessage error " + Marshal.GetLastWin32Error();
            }
            return "";
        }

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        public struct LOGFONT
        {
            public int lfHeight; public int lfWidth; public int lfEscapement; public int lfOrientation; public int lfWeight;
            public byte lfItalic; public byte lfUnderline; public byte lfStrikeOut; public byte lfCharSet;
            public byte lfOutPrecision; public byte lfClipPrecision; public byte lfQuality; public byte lfPitchAndFamily;
            [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string lfFaceName;
        }
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        public struct NONCLIENTMETRICS
        {
            public int cbSize; public int iBorderWidth; public int iScrollWidth; public int iScrollHeight; public int iCaptionWidth; public int iCaptionHeight;
            public LOGFONT lfCaptionFont; public int iSmCaptionWidth; public int iSmCaptionHeight; public LOGFONT lfSmCaptionFont;
            public int iMenuWidth; public int iMenuHeight; public LOGFONT lfMenuFont; public LOGFONT lfStatusFont; public LOGFONT lfMessageFont;
            public int iPaddedBorderWidth;
        }
        [DllImport("user32.dll", EntryPoint = "SystemParametersInfoW", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern bool SystemParametersInfoMetrics(uint action, uint param, ref NONCLIENTMETRICS metrics, uint winIni);

        static NONCLIENTMETRICS savedMetrics;
        static bool haveSaved;

        static string Metrics(out NONCLIENTMETRICS metrics)
        {
            metrics = new NONCLIENTMETRICS();
            metrics.cbSize = Marshal.SizeOf(typeof(NONCLIENTMETRICS));
            if (SystemParametersInfoMetrics(0x0029 /* SPI_GETNONCLIENTMETRICS */, (uint)metrics.cbSize, ref metrics, 0)) return "";
            return "SPI_GETNONCLIENTMETRICS error " + Marshal.GetLastWin32Error();
        }

        static string SetMetrics(NONCLIENTMETRICS metrics)
        {
            // Not saved to the profile and not broadcast: only this session's value changes.
            if (SystemParametersInfoMetrics(0x002A /* SPI_SETNONCLIENTMETRICS */, (uint)metrics.cbSize, ref metrics, 0)) return "";
            return "SPI_SETNONCLIENTMETRICS error " + Marshal.GetLastWin32Error();
        }

        /// The message font: "face height".
        public static string MessageFont()
        {
            NONCLIENTMETRICS m;
            string err = Metrics(out m);
            return err != "" ? err : m.lfMessageFont.lfFaceName + " " + m.lfMessageFont.lfHeight;
        }

        /// Makes the message font `factor` times as tall (the first call saves the original); "" or the error.
        public static string ScaleMessageFont(double factor)
        {
            NONCLIENTMETRICS m;
            string err = Metrics(out m);
            if (err != "") return err;
            if (!haveSaved) { savedMetrics = m; haveSaved = true; }
            m.lfMessageFont.lfHeight = (int)Math.Round(m.lfMessageFont.lfHeight * factor);
            return SetMetrics(m);
        }

        /// Puts back the message font ScaleMessageFont saved; "" or the error.
        public static string RestoreMessageFont()
        {
            if (!haveSaved) return "";
            string err = SetMetrics(savedMetrics);
            if (err == "") haveSaved = false;
            return err;
        }

        /// Sends WM_SETTINGCHANGE for SPI_SETNONCLIENTMETRICS, as Windows does
        /// when the text size changes; "" or the error.
        public static string SendMetricsChanged(IntPtr hwnd)
        {
            IntPtr result;
            if (SendPlain(hwnd, 0x001A, (IntPtr)0x002A, IntPtr.Zero, SMTO_ABORTIFHUNG, 5000, out result) != IntPtr.Zero) return "";
            return "SendMessageTimeout error " + Marshal.GetLastWin32Error();
        }

        /// Repaints the window and every control in it, now.
        public static bool RepaintAll(IntPtr hwnd)
        {
            // RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW
            return RedrawWindow(hwnd, IntPtr.Zero, IntPtr.Zero, 0x0001 | 0x0004 | 0x0080 | 0x0100);
        }

        /// Puts the window on top of every other one, topmost, without activating it.
        public static bool Raise(IntPtr hwnd)
        {
            // HWND_TOPMOST; SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE
            return SetWindowPos(hwnd, new IntPtr(-1), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010);
        }

        /// The process that owns the window at screen point (x, y).
        public static int OwnerAt(int x, int y)
        {
            POINT p = new POINT();
            p.X = x;
            p.Y = y;
            uint pid;
            GetWindowThreadProcessId(WindowFromPoint(p), out pid);
            return (int)pid;
        }

        public static string GetLocale(uint type)
        {
            StringBuilder data = new StringBuilder(100);
            if (GetLocaleInfo(LOCALE_USER_DEFAULT, type, data, data.Capacity) <= 0) return null;
            return data.ToString();
        }

        /// Sets one of the user's locale values; "" or the error.
        public static string SetLocale(uint type, string value)
        {
            if (SetLocaleInfo(LOCALE_USER_DEFAULT, type, value)) return "";
            return "SetLocaleInfo(0x" + type.ToString("X") + ", '" + value + "') error " + Marshal.GetLastWin32Error();
        }

        /// The screen rectangle (l, t, r, b), as a bitmap.
        public static Bitmap Capture(int[] rect)
        {
            int width = Math.Max(1, rect[2] - rect[0]), height = Math.Max(1, rect[3] - rect[1]);
            Bitmap bitmap = new Bitmap(width, height, PixelFormat.Format32bppArgb);
            using (Graphics g = Graphics.FromImage(bitmap)) g.CopyFromScreen(rect[0], rect[1], 0, 0, bitmap.Size);
            return bitmap;
        }

        static int[] Pixels(Bitmap bitmap)
        {
            Rectangle all = new Rectangle(0, 0, bitmap.Width, bitmap.Height);
            BitmapData data = bitmap.LockBits(all, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            int[] pixels = new int[bitmap.Width * bitmap.Height];
            for (int y = 0; y < bitmap.Height; y++) Marshal.Copy(IntPtr.Add(data.Scan0, y * data.Stride), pixels, y * bitmap.Width, bitmap.Width);
            bitmap.UnlockBits(data);
            return pixels;
        }

        public static int Compared;
        public static int DifferentPixels;
        public static string Detail = "";

        /// Compares two captures of screen rectangle `area` inside the rectangles
        /// of `children` (screen coordinates). Sets Compared (children compared),
        /// DifferentPixels and Detail (each child that differs, with its count).
        public static void Compare(Bitmap before, Bitmap after, int[] area, Child[] children)
        {
            int[] a = Pixels(before), b = Pixels(after);
            int width = before.Width;
            Compared = 0;
            DifferentPixels = 0;
            StringBuilder detail = new StringBuilder();
            foreach (Child c in children)
            {
                int left = Math.Max(c.Left, area[0]), top = Math.Max(c.Top, area[1]);
                int right = Math.Min(c.Right, area[2]), bottom = Math.Min(c.Bottom, area[3]);
                if (right <= left || bottom <= top) continue;
                Compared++;
                int differ = 0;
                for (int y = top; y < bottom; y++)
                {
                    for (int x = left; x < right; x++)
                    {
                        int i = (y - area[1]) * width + (x - area[0]);
                        if (a[i] != b[i]) differ++;
                    }
                }
                if (differ > 0)
                {
                    DifferentPixels += differ;
                    if (detail.Length > 0) detail.Append("; ");
                    detail.Append(c.Id + " '" + c.Text + "': " + differ + " px");
                }
            }
            Detail = detail.ToString();
        }
    }
}
'@
    Add-Type -TypeDefinition $uiSettingsHelpers -ReferencedAssemblies System.Drawing -Language CSharp

    # Starts MelCat without settings.json, so it opens its settings window
    # (with the default settings: On Schedule, Mon-Fri 9:00-18:00), and
    # returns that window once it has painted. The pointer is parked away
    # from the window first, so nothing in it is hovered.
    function Open-SettingsWindow {
        Close-MelCat
        Set-Pointer ($script:Primary.Left + 150) ($script:Primary.Top + 40)
        foreach ($path in @($SettingsPath, "$SettingsPath.tmp")) { Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue }
        Assert (-not (Test-Path -LiteralPath $SettingsPath)) "could not remove $SettingsPath"
        $script:Mel = Start-MelCat
        $hwnd = [IntPtr]::Zero
        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        do {
            $hwnd = [MelCatSmoke.UiSettings.Win]::FindVisibleOwned('MelCat.Settings', $script:Mel.Id)
            if ($hwnd -ne [IntPtr]::Zero) { break }
            Start-Sleep -Milliseconds 100
        } while ($watch.ElapsedMilliseconds -lt 10000)
        Assert ($hwnd -ne [IntPtr]::Zero) "no visible MelCat.Settings window of MelCat (pid $($script:Mel.Id)) within 10 s"
        Start-Sleep -Milliseconds 700  # let it paint
        Note ("settings window 0x{0:X} of MelCat (pid {1}) at {2}" -f $hwnd.ToInt64(), $script:Mel.Id, ([MelCatSmoke.UiSettings.Win]::Rect($hwnd) -join ','))
        return $hwnd
    }

    # Closes the settings window (WM_CLOSE) and waits until it is gone.
    function Close-SettingsWindow([IntPtr]$Hwnd) {
        $err = [MelCatSmoke.UiSettings.Win]::Close($Hwnd)
        Assert ($err -eq '') "could not post WM_CLOSE to the settings window: $err"
        Assert (Wait-Until { -not [MelCatSmoke.UiSettings.Win]::Exists($Hwnd) } 5000) 'the settings window was still there 5 s after WM_CLOSE'
    }

    Invoke-Step 'settings-preview-label' {
        $hwnd = Open-SettingsWindow
        $children = @([MelCatSmoke.UiSettings.Win]::Children($hwnd))
        $label = @($children | Where-Object { $_.Class -eq 'Static' -and $_.Text -eq 'Try the movement now' })
        $preview = @($children | Where-Object { $_.Id -eq 135 })
        $note = @($children | Where-Object { $_.Id -eq 136 })
        Assert ($label.Count -eq 1) "no Static reads 'Try the movement now' (as on macOS); the controls: $(($children | ForEach-Object { $_.ToString() }) -join '; ')"
        Assert (($preview.Count -eq 1) -and ($preview[0].Text -eq 'Preview')) "control 135 is not the Preview button: $($preview -join '; ')"
        Assert ($note.Count -eq 1) 'control 136 (the note) is missing'
        $l = $label[0]; $b = $preview[0]; $n = $note[0]
        Note "label: $l; button: $b; note: $n"
        Assert ($l.Visible -and ($l.Right -gt $l.Left)) "the label is not shown: $l"
        Assert (($l.Right -le $b.Left) -and ($l.Top -eq $b.Top) -and ($l.Bottom -eq $b.Bottom)) "the label is not on the Preview button's row, left of it"
        Assert (($n.Top -ge $l.Bottom) -and ($n.Left -eq $l.Left)) "the note '$($n.Text)' is not under the label"
        $shot = [MelCatSmoke.UiSettings.Win]::Capture([MelCatSmoke.UiSettings.Win]::Rect($hwnd))
        $shot.Save((Join-Path $ArtifactsDir 'settings-preview-label.png'), [System.Drawing.Imaging.ImageFormat]::Png)
        $shot.Dispose()
        Note 'screenshot: settings-preview-label.png'
        Close-SettingsWindow $hwnd
    }

    Invoke-Step 'settings-repaint' {
        $hwnd = Open-SettingsWindow
        try {
            Assert ([MelCatSmoke.UiSettings.Win]::Raise($hwnd)) 'could not put the settings window on top'
            Start-Sleep -Milliseconds 300
            $before = [MelCatSmoke.UiSettings.Win]::Rect($hwnd)
            # A larger text size: the message font half as tall again, and the
            # WM_SETTINGCHANGE Windows sends for it.
            $font = [MelCatSmoke.UiSettings.Win]::MessageFont()
            $err = [MelCatSmoke.UiSettings.Win]::ScaleMessageFont(1.5)
            Assert ($err -eq '') $err
            Note "message font: $font, now $([MelCatSmoke.UiSettings.Win]::MessageFont())"
            $err = [MelCatSmoke.UiSettings.Win]::SendMetricsChanged($hwnd)
            Assert ($err -eq '') "WM_SETTINGCHANGE (SPI_SETNONCLIENTMETRICS) was not answered: $err"
            Start-Sleep -Milliseconds 700
            $after = [MelCatSmoke.UiSettings.Win]::Rect($hwnd)
            $grew = ($after[2] - $after[0]) / [double]($before[2] - $before[0])
            Note ('the window went from {0} to {1} ({2:N2} times as wide)' -f ($before -join ','), ($after -join ','), $grew)
            Assert ($grew -gt 1.1) 'the window was not laid out again for the larger font'

            # The part of the window on the screen, and the labels in it.
            $p = $script:Primary
            $area = [int[]]@([Math]::Max($after[0], $p.Left), [Math]::Max($after[1], $p.Top), [Math]::Min($after[2], $p.Right), [Math]::Min($after[3], $p.Bottom))
            $statics = @([MelCatSmoke.UiSettings.Win]::Children($hwnd) | Where-Object { $_.Class -eq 'Static' -and $_.Visible -and $_.Text -ne '' })
            $covered = @($statics | Where-Object {
                $x = [int][Math]::Floor(($_.Left + $_.Right) / 2); $y = [int][Math]::Floor(($_.Top + $_.Bottom) / 2)
                ($x -ge $area[0]) -and ($x -lt $area[2]) -and ($y -ge $area[1]) -and ($y -lt $area[3]) -and ([MelCatSmoke.UiSettings.Win]::OwnerAt($x, $y) -ne $script:Mel.Id)
            })
            Assert ($covered.Count -eq 0) "another window covers labels of the settings window: $($covered -join '; ')"

            # What the window shows now, against what it shows once every control repainted.
            $shown = [MelCatSmoke.UiSettings.Win]::Capture($area)
            Assert ([MelCatSmoke.UiSettings.Win]::RepaintAll($hwnd)) 'RedrawWindow failed'
            Start-Sleep -Milliseconds 500
            $repainted = [MelCatSmoke.UiSettings.Win]::Capture($area)
            $shown.Save((Join-Path $ArtifactsDir 'settings-repaint-1-after-font-change.png'), [System.Drawing.Imaging.ImageFormat]::Png)
            $repainted.Save((Join-Path $ArtifactsDir 'settings-repaint-2-repainted.png'), [System.Drawing.Imaging.ImageFormat]::Png)
            [MelCatSmoke.UiSettings.Win]::Compare($shown, $repainted, $area, $statics)
            $compared = [MelCatSmoke.UiSettings.Win]::Compared
            $differ = [MelCatSmoke.UiSettings.Win]::DifferentPixels
            Note ('{0} labels on screen compared (screenshots settings-repaint-*.png): {1} pixels differ after a full repaint{2}' -f $compared, $differ, $(if ($differ -gt 0) { ': ' + [MelCatSmoke.UiSettings.Win]::Detail } else { '' }))
            $shown.Dispose(); $repainted.Dispose()
            Assert ($compared -ge 5) "only $compared labels of the settings window are on the screen"
            Assert ($differ -eq 0) "labels still showed pixels drawn in the old font: $([MelCatSmoke.UiSettings.Win]::Detail)"
            Close-SettingsWindow $hwnd
        } finally {
            $err = [MelCatSmoke.UiSettings.Win]::RestoreMessageFont()
            if ($err) { Write-Host "    (could not restore the message font: $err)" }
        }
    }

    Invoke-Step 'settings-short-time' {
        # The long time format (LOCALE_STIMEFORMAT), the short one
        # (LOCALE_SSHORTTIME) and the AM and PM markers (LOCALE_S1159,
        # LOCALE_S2359): set for this step, put back afterwards.
        $types = @(0x1003, 0x79, 0x28, 0x29)
        $values = @('h:mm:ss tt', 'HH:mm tt', 'a.m.', 'p.m.')
        $saved = New-Object 'string[]' 4
        for ($i = 0; $i -lt 4; $i++) { $saved[$i] = [MelCatSmoke.UiSettings.Win]::GetLocale([uint32]$types[$i]) }
        Note ("the user's time formats: long '{0}', short '{1}', AM '{2}', PM '{3}'" -f $saved[0], $saved[1], $saved[2], $saved[3])
        try {
            for ($i = 0; $i -lt 4; $i++) {
                $err = [MelCatSmoke.UiSettings.Win]::SetLocale([uint32]$types[$i], $values[$i])
                Assert ($err -eq '') $err
            }
            $now = New-Object 'string[]' 4
            for ($i = 0; $i -lt 4; $i++) { $now[$i] = [MelCatSmoke.UiSettings.Win]::GetLocale([uint32]$types[$i]) }
            Note ("set for this step: long '{0}', short '{1}', AM '{2}', PM '{3}'" -f $now[0], $now[1], $now[2], $now[3])
            Assert ($now[1] -eq 'HH:mm tt') "the short time format reads back as '$($now[1])'"
            Start-Sleep -Milliseconds 300

            $hwnd = Open-SettingsWindow
            $to = [MelCatSmoke.UiSettings.Win]::Item($hwnd, 123)
            $hint = [MelCatSmoke.UiSettings.Win]::Item($hwnd, 124)
            Assert (($to -ne [IntPtr]::Zero) -and ($hint -ne [IntPtr]::Zero)) 'the To picker (123) or the hint (124) is missing'
            Note ("To picker reads '{0}', hint '{1}'" -f [MelCatSmoke.UiSettings.Win]::Text($to), [MelCatSmoke.UiSettings.Win]::Text($hint))

            # 18:00 to 06:00: twelve presses of Down on the picker's hour.
            Assert ([MelCatSmoke.UiSettings.Win]::FocusControl($hwnd, $to)) 'WM_NEXTDLGCTL was not answered'
            Note ('focus 0x{0:X} (the To picker is 0x{1:X}); foreground 0x{2:X}' -f [MelCatSmoke.UiSettings.Win]::FocusOf($hwnd).ToInt64(), $to.ToInt64(), [MelCatSmoke.UiSettings.Win]::Foreground().ToInt64())
            $err = [MelCatSmoke.UiSettings.Win]::PostKeys($to, 0x28, 0x50, 12)
            Assert ($err -eq '') "could not post the key presses: $err"
            $script:EndMinutes = $null
            $ended = Wait-Until {
                try { $script:EndMinutes = ([System.IO.File]::ReadAllText($SettingsPath) | ConvertFrom-Json).end_minutes } catch { $script:EndMinutes = $null }
                $script:EndMinutes -eq 360
            } 5000
            $text = [MelCatSmoke.UiSettings.Win]::Text($hint)
            Note ("after twelve Down presses: To picker reads '{0}', settings.json end_minutes {1}, hint '{2}'" -f [MelCatSmoke.UiSettings.Win]::Text($to), $script:EndMinutes, $text)
            Assert $ended 'the schedule did not end at 06:00 after twelve Down presses on the To picker'
            Assert (-not $text.EndsWith('..')) "the hint ends in two periods: '$text'"
            $expected = 'Ends the next morning at 06:00 a.m.'
            Assert ($text -eq $expected) "the hint reads '$text', not '$expected' (the short time format HH:mm tt, as the pickers show it)"
            Close-SettingsWindow $hwnd
        } finally {
            Close-MelCat
            for ($i = 0; $i -lt 4; $i++) {
                if ($null -ne $saved[$i]) {
                    $err = [MelCatSmoke.UiSettings.Win]::SetLocale([uint32]$types[$i], $saved[$i])
                    if ($err) { Write-Host "    (could not restore a time format: $err)" }
                }
            }
        }
    }
} catch {
    Write-Host "Smoke test aborted: $($_.Exception.Message) (script line $($_.InvocationInfo.ScriptLineNumber))"
    $script:Results.Add([pscustomobject]@{ Step = 'script'; Result = 'failed'; Evidence = $_.Exception.Message })
    Save-Desktop 'aborted'
} finally {
    try { [MelCatSmoke.Stroll.Sampler]::Stop() } catch { }
    try { [MelCatSmoke.JiggleIdle.IdleWatch]::Stop() } catch { }
    try { [MelCatSmoke.Kill.Ui]::Close() } catch { }
    Stop-MelCat
    Save-Desktop 'final'

    Write-Host ''
    Write-Host '=== MelCat log ==='
    Write-Host (Read-MelLog)
    if (Test-Path -LiteralPath $LogPath) { Copy-Item -LiteralPath $LogPath -Destination $ArtifactsDir -Force }

    Write-Host ''
    Write-Host '=== Summary ==='
    $script:Results | Format-Table -AutoSize -Wrap | Out-String -Width 220 | Write-Host
    $script:Results | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $ArtifactsDir 'summary.json') -Encoding UTF8
    if ($env:GITHUB_STEP_SUMMARY) {
        $md = New-Object System.Text.StringBuilder
        [void]$md.AppendLine('### MelCat smoke test')
        [void]$md.AppendLine('')
        [void]$md.AppendLine('| Step | Result | Evidence |')
        [void]$md.AppendLine('|------|--------|----------|')
        foreach ($r in $script:Results) {
            [void]$md.AppendLine(('| {0} | {1} | {2} |' -f $r.Step, $r.Result.ToUpper(), ($r.Evidence -replace '\|', '/' -replace "`r?`n", ' ')))
        }
        Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Value $md.ToString() -Encoding UTF8
    }
    $failed = @($script:Results | Where-Object { $_.Result -eq 'failed' })
    if ($failed.Count -gt 0) {
        Write-Host ("FAILED: {0}" -f (($failed | ForEach-Object { $_.Step }) -join ', '))
        $exitCode = 1
    } else {
        Write-Host 'No step failed.'
    }
}
exit $exitCode
