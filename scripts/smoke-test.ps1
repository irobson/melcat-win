<#
.SYNOPSIS
    End-to-end smoke test of MelCat.exe on a real (or CI) Windows desktop.

.DESCRIPTION
    Steps (each reports PASSED, FAILED or SKIPPED with its evidence):

      exe           MelCat.exe is a 64-bit GUI executable with MelCat's
                    version info (ProductName MelCat, the workspace version),
                    an icon, and a manifest asking for PerMonitorV2 DPI
                    awareness and common controls v6.

    More steps are added as the app is implemented (see docs/ARCHITECTURE.md,
    "What the smoke test proves").

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

# ---------------------------------------------------------------------------
# Steps

$exitCode = 0
try {
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

        $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($Exe)
        Note "icon: $($icon.Width)x$($icon.Height)"

        $manifest = [MelCatSmoke.Win]::Manifest($Exe)
        Assert ($manifest -match 'PerMonitorV2') 'the manifest does not ask for PerMonitorV2 DPI awareness'
        Assert ($manifest -match 'Microsoft\.Windows\.Common-Controls') 'the manifest does not ask for common controls v6'
        Assert ($manifest -match 'asInvoker') 'the manifest does not run as invoker'
        Note 'manifest: PerMonitorV2, common controls v6, asInvoker'
    }
} catch {
    Write-Host "Smoke test aborted: $($_.Exception.Message) (script line $($_.InvocationInfo.ScriptLineNumber))"
    $script:Results.Add([pscustomobject]@{ Step = 'script'; Result = 'failed'; Evidence = $_.Exception.Message })
    Save-Desktop 'aborted'
} finally {
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
