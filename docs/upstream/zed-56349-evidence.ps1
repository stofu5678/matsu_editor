<#
.SYNOPSIS
Collects, in one run, the evidence for
https://github.com/zed-industries/zed/issues/56349#issuecomment-6078561427
(GPUI's glyph atlas uploads and the process memory on Windows).

.DESCRIPTION
Run it on the Windows machine to check, from a clone of https://github.com/stofu5678/matsu_editor
that has this script (branch docs/zed-56349-repro). It needs Git, Rust (rustup) and the Visual
Studio C++ build tools, about 8 GB of disk and about 20 minutes. Windows open and close one at a
time while it runs; leave the machine alone until it ends. docs/upstream/zed-56349.md (Japanese)
has the same steps by hand.

The script was written with Claude Code (an AI) and reviewed with Codex (an AI of another
company). It does this:

1. Records the machine (Windows build, model, CPU, GPU and driver, display scale, ClearType,
   Rust) and reads crates/gpui_windows/src/directx_atlas.rs on Zed's main from GitHub.
2. Takes the prototype at commit 97908ca (poc/gpui-textview, gpui-pre 0.3.8) out of the clone
   with git archive and builds it three ways:
     gpui  as published (cargo build --release --locked);
     fix   with the workaround patch that the comment links to
           (poc/gpui-textview/patches/gpui-pre-windows-atlas-upload.diff at 97908ca);
     diag  with poc/gpui-textview/patches/gpui-pre-windows-atlas-diag.diff (at HEAD), which logs
           the private bytes right before and after each atlas texture creation and upload.
   The patches go on gpui-pre-windows 0.3.8 from the .crate file in Cargo's cache, after its
   SHA-256 is checked against the prototype's Cargo.lock.
3. Starts each window, reads its working set and private bytes 6 seconds later (Get-Process;
   Task Manager shows them as "Working set (memory)" and "Commit size") and closes it. Some runs
   also write a memory breakdown (memstat-<pid>.txt) or the atlas log (atlas-<pid>.txt). The
   first run of each window in the gpui and fix builds is captured as a picture (PrintWindow).
4. Writes summary.md (the comment's numbers next to the measured ones), measurements.csv, the
   logs and the pictures to <WorkDir>\evidence-<time>, replaces the user folder, user name and
   computer name in the text files, and zips the folder.

.PARAMETER WorkDir
Folder for the builds (about 6 GB) and the evidence. It must be outside the clone.

.PARAMETER Repeat
How many times each window is measured. The summary shows the median and the range.

.EXAMPLE
powershell -NoProfile -ExecutionPolicy Bypass -File C:\work\matsu_editor\docs\upstream\zed-56349-evidence.ps1
#>
#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$WorkDir = 'C:\work\zed-56349',
    [ValidateRange(1, 10)][int]$Repeat = 3
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$PocCommit = '97908ca0d9f48571cced7f7c82c6cdbfca7353f5'
$FixPatchInRepo = 'poc/gpui-textview/patches/gpui-pre-windows-atlas-upload.diff'
$DiagPatchInRepo = 'poc/gpui-textview/patches/gpui-pre-windows-atlas-diag.diff'
$Poc = 'gpui-textview-poc'
$Inv = [Globalization.CultureInfo]::InvariantCulture
$Utf8 = New-Object System.Text.UTF8Encoding($false)
$Started = Get-Date
$LogPath = $null
$Out = $null
$Repo = $null
$RustHost = $null
$LockPath = $null
$LockPristine = $null
$Live = $null
$RunCount = 0
$RunTotal = 0
$Plan = New-Object System.Collections.Generic.List[object]
$Samples = New-Object System.Collections.Generic.List[object]
$Runs = New-Object System.Collections.Generic.List[object]

# Window pictures and their comparison. PrintWindow with PW_RENDERFULLCONTENT also gets what
# DirectX draws; the thread is made per-monitor DPI aware so that the sizes are real pixels.
$NativeSource = @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace Zed56349
{
    public static class Native
    {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CreateFileW(string name, uint access, uint share, IntPtr security, uint creation, uint flags, IntPtr template);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint GetFinalPathNameByHandleW(IntPtr file, StringBuilder path, uint size, uint flags);

        [DllImport("kernel32.dll")]
        private static extern bool CloseHandle(IntPtr handle);

        // The path with junctions, symbolic links and short (8.3) names resolved for the deepest
        // part of it that exists, file or folder; the part that does not exist yet is appended
        // as it is.
        public static string RealPath(string path)
        {
            string full = Path.GetFullPath(path);
            if (full.Length > 3) full = full.TrimEnd('\\');
            string existing = full;
            string rest = "";
            while (!Directory.Exists(existing) && !File.Exists(existing))
            {
                string parent = Path.GetDirectoryName(existing);
                if (parent == null) return full;
                rest = rest.Length == 0 ? Path.GetFileName(existing) : Path.Combine(Path.GetFileName(existing), rest);
                existing = parent;
            }
            // No access asked (only the name is read); FILE_SHARE_READ | FILE_SHARE_WRITE |
            // FILE_SHARE_DELETE; OPEN_EXISTING; FILE_FLAG_BACKUP_SEMANTICS, which folders need.
            IntPtr handle = CreateFileW(existing, 0, 7, IntPtr.Zero, 3, 0x02000000, IntPtr.Zero);
            if (handle == new IntPtr(-1)) throw new Win32Exception(Marshal.GetLastWin32Error(), "cannot open " + existing);
            try
            {
                StringBuilder buffer = new StringBuilder(32768);
                uint length = GetFinalPathNameByHandleW(handle, buffer, (uint)buffer.Capacity, 0);
                if (length == 0 || length >= buffer.Capacity) throw new Win32Exception(Marshal.GetLastWin32Error(), "cannot resolve " + existing);
                string real = buffer.ToString();
                if (real.StartsWith(@"\\?\UNC\")) real = @"\\" + real.Substring(8);
                else if (real.StartsWith(@"\\?\")) real = real.Substring(4);
                if (real.Length > 3) real = real.TrimEnd('\\');
                return rest.Length == 0 ? real : Path.Combine(real, rest);
            }
            finally
            {
                CloseHandle(handle);
            }
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct Rect
        {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;
        }

        [DllImport("user32.dll")]
        private static extern bool GetWindowRect(IntPtr window, out Rect rect);

        [DllImport("user32.dll")]
        private static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);

        [DllImport("user32.dll")]
        private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);

        private const uint RenderFullContent = 2;
        private static readonly IntPtr PerMonitorAwareV2 = new IntPtr(-4);

        public static string Capture(IntPtr window, string path)
        {
            if (window == IntPtr.Zero) return "no window";
            IntPtr previous = SetThreadDpiAwarenessContext(PerMonitorAwareV2);
            try
            {
                Rect rect;
                if (!GetWindowRect(window, out rect)) return "GetWindowRect failed";
                int width = rect.Right - rect.Left;
                int height = rect.Bottom - rect.Top;
                if (width <= 0 || height <= 0) return "empty window";
                using (Bitmap bitmap = new Bitmap(width, height, PixelFormat.Format32bppRgb))
                {
                    bool printed;
                    using (Graphics graphics = Graphics.FromImage(bitmap))
                    {
                        IntPtr dc = graphics.GetHdc();
                        try { printed = PrintWindow(window, dc, RenderFullContent); }
                        finally { graphics.ReleaseHdc(dc); }
                    }
                    if (!printed) return "PrintWindow failed";
                    bitmap.Save(path, ImageFormat.Png);
                    int colors = CountColors(Pixels(bitmap), 1000);
                    return string.Format("{0}x{1} pixels, {2}{3} colors", width, height, colors, colors >= 1000 ? "+" : "");
                }
            }
            finally
            {
                if (previous != IntPtr.Zero) SetThreadDpiAwarenessContext(previous);
            }
        }

        public static string Compare(string first, string second)
        {
            using (Bitmap a = new Bitmap(first))
            using (Bitmap b = new Bitmap(second))
            {
                if (a.Width != b.Width || a.Height != b.Height)
                    return string.Format("the sizes differ ({0}x{1} and {2}x{3})", a.Width, a.Height, b.Width, b.Height);
                int[] p = Pixels(a);
                int[] q = Pixels(b);
                long differ = 0;
                int left = a.Width, top = a.Height, right = -1, bottom = -1;
                for (int i = 0; i < p.Length; i++)
                {
                    if ((p[i] & 0xFFFFFF) == (q[i] & 0xFFFFFF)) continue;
                    differ++;
                    int x = i % a.Width, y = i / a.Width;
                    if (x < left) left = x;
                    if (x > right) right = x;
                    if (y < top) top = y;
                    if (y > bottom) bottom = y;
                }
                if (differ == 0) return string.Format("0 of {0} pixels differ", p.Length);
                return string.Format("{0} of {1} pixels differ, all within x {2}-{3}, y {4}-{5}", differ, p.Length, left, right, top, bottom);
            }
        }

        private static int[] Pixels(Bitmap bitmap)
        {
            Rectangle all = new Rectangle(0, 0, bitmap.Width, bitmap.Height);
            BitmapData data = bitmap.LockBits(all, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            try
            {
                int[] pixels = new int[bitmap.Width * bitmap.Height];
                for (int y = 0; y < bitmap.Height; y++)
                    Marshal.Copy(IntPtr.Add(data.Scan0, y * data.Stride), pixels, y * bitmap.Width, bitmap.Width);
                return pixels;
            }
            finally
            {
                bitmap.UnlockBits(data);
            }
        }

        private static int CountColors(int[] pixels, int cap)
        {
            HashSet<int> seen = new HashSet<int>();
            foreach (int pixel in pixels)
            {
                seen.Add(pixel & 0xFFFFFF);
                if (seen.Count >= cap) break;
            }
            return seen.Count;
        }
    }
}
'@

# string.Format with the invariant culture, so that numbers look the same on every machine.
function Fmt([string]$Format) {
    [string]::Format($Inv, $Format, [object[]]$args)
}

function Write-Log([string]$Text = '') {
    Write-Host $Text
    if ($LogPath) { [IO.File]::AppendAllText($LogPath, $Text + "`r`n", $Utf8) }
}

# Runs a program and returns its output lines; stops the script when the program fails.
# Windows PowerShell turns the program's stderr lines into errors, so 'Stop' is not used here.
function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)][string]$Exe,
        [string[]]$Arguments = @(),
        [string]$Log = '',
        [switch]$Quiet
    )
    $ErrorActionPreference = 'Continue'
    $lines = New-Object System.Collections.Generic.List[string]
    & $Exe @Arguments 2>&1 | ForEach-Object {
        $line = [string]$_
        $lines.Add($line)
        if ($Log) { [IO.File]::AppendAllText($Log, $line + "`r`n", $Utf8) }
        if (-not $Quiet) { Write-Host "    $line" }
    }
    if ($LASTEXITCODE -ne 0) {
        throw (Fmt '{0} {1} failed with exit code {2}' $Exe ($Arguments -join ' ') $LASTEXITCODE)
    }
    , $lines.ToArray()
}

# Whether two folders are the same, or one holds the other.
function Test-Overlap([string]$First, [string]$Second) {
    $a = $First.TrimEnd('\') + '\'
    $b = $Second.TrimEnd('\') + '\'
    $a.StartsWith($b, [StringComparison]::OrdinalIgnoreCase) -or $b.StartsWith($a, [StringComparison]::OrdinalIgnoreCase)
}

# Removes a file or a folder, but only inside the work folder, and never the clone or a folder
# that holds it. Both are compared as real paths (junctions and links resolved).
function Remove-WorkItem([string]$Path) {
    $full = [Zed56349.Native]::RealPath($Path)
    $root = $WorkDir.TrimEnd('\') + '\'
    if ($full.Length -le $root.Length -or -not $full.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing to remove $Path, which is not inside $root"
    }
    if (-not $Repo -or (Test-Overlap $full $Repo)) { throw "refusing to remove $Path, which may hold the clone" }
    if (Test-Path -LiteralPath $full) { Remove-Item -LiteralPath $full -Recurse -Force }
}

# Where a build puts a program. The target is always given to cargo, so a build.target
# setting cannot move the programs elsewhere.
function Get-Exe([string]$Build, [string]$Bin) {
    Join-Path $Targets[$Build] ($RustHost + '\release\' + $Bin + '.exe')
}

function Get-Prop($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    $property = $Object.PSObject.Properties[$Name]
    if ($property) { return $property.Value }
    $null
}

function Get-Sha256([string]$Path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

# The machine type in the PE header: whether a program runs natively on ARM64.
function Get-PeMachine([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    try {
        $reader = New-Object IO.BinaryReader($stream)
        $stream.Position = 0x3C
        $stream.Position = $reader.ReadInt32() + 4
        $machine = $reader.ReadUInt16()
    } finally {
        $stream.Dispose()
    }
    switch ($machine) {
        0xAA64 { 'ARM64' }
        0x8664 { 'x64' }
        0x014C { 'x86' }
        default { Fmt 'machine 0x{0:X4}' $machine }
    }
}

function Get-MachineInfo {
    $info = [ordered]@{}
    $os = Get-CimInstance Win32_OperatingSystem
    $version = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
    $info['Windows'] = Fmt '{0} {1}, build {2}.{3}, {4}' $os.Caption (Get-Prop $version 'DisplayVersion') $os.BuildNumber (Get-Prop $version 'UBR') ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture)
    $computer = Get-CimInstance Win32_ComputerSystem
    $info['Model'] = Fmt '{0} {1}' $computer.Manufacturer $computer.Model
    $info['CPU'] = (@(Get-CimInstance Win32_Processor) | ForEach-Object { ([string]$_.Name).Trim() }) -join '; '
    $info['Memory'] = Fmt '{0:F1} GiB' ($computer.TotalPhysicalMemory / 1GB)
    $gpus = @(Get-CimInstance Win32_VideoController)
    $info['GPU'] = ($gpus | ForEach-Object {
            $date = '?'
            if ($_.DriverDate) { $date = $_.DriverDate.ToString('yyyy-MM-dd', $Inv) }
            Fmt '{0}, driver {1} ({2})' $_.Name $_.DriverVersion $date
        }) -join '; '
    $info['Display'] = (@($gpus | Where-Object { $_.CurrentHorizontalResolution }) | ForEach-Object {
            Fmt '{0}x{1}' $_.CurrentHorizontalResolution $_.CurrentVerticalResolution
        }) -join '; '
    $dpi = Get-Prop (Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' -ErrorAction SilentlyContinue) 'AppliedDPI'
    if ($dpi) { $info['Display scale'] = Fmt 'AppliedDPI {0} ({1} %)' $dpi ([int]$dpi * 100 / 96) } else { $info['Display scale'] = 'unknown' }
    $desktop = Get-ItemProperty 'HKCU:\Control Panel\Desktop'
    $info['Font smoothing'] = Fmt 'FontSmoothing {0}, FontSmoothingType {1} (2 is ClearType; GPUI then draws glyphs as Subpixel)' (Get-Prop $desktop 'FontSmoothing') (Get-Prop $desktop 'FontSmoothingType')
    if (Test-Path -LiteralPath (Join-Path $env:SystemRoot 'Fonts\BIZ-UDGothicR.ttc')) { $info['BIZ UDGothic (used by minimal)'] = 'installed' } else { $info['BIZ UDGothic (used by minimal)'] = 'not found' }
    $info['PowerShell'] = [string]$PSVersionTable.PSVersion
    $info
}

# Comment: "crates/gpui_windows/src/directx_atlas.rs on current main still uploads with UpdateSubresource".
function Get-ZedMain {
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
        $headers = @{ 'User-Agent' = 'zed-56349-evidence' }
        $commit = Invoke-RestMethod -Uri 'https://api.github.com/repos/zed-industries/zed/commits/main' -Headers $headers -TimeoutSec 30
        $sha = [string]$commit.sha
        $date = [string]$commit.commit.committer.date
        $file = Invoke-WebRequest -UseBasicParsing -Uri "https://raw.githubusercontent.com/zed-industries/zed/$sha/crates/gpui_windows/src/directx_atlas.rs" -Headers $headers -TimeoutSec 30
        $lines = ([string]$file.Content) -split "`n"
        $calls = @(for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -match '\bUpdateSubresource\(') { $i + 1 } })
        $staging = @($lines | Where-Object { $_ -match 'D3D11_USAGE_STAGING' }).Count
        $where = 'no UpdateSubresource( call'
        if ($calls.Count -gt 0) { $where = 'UpdateSubresource( on line ' + ($calls -join ', ') }
        Fmt 'zed-industries/zed@{0} ({1}), crates/gpui_windows/src/directx_atlas.rs: {2}; D3D11_USAGE_STAGING appears {3} times' $sha.Substring(0, 12) $date $where $staging
    } catch {
        'not checked: ' + $_.Exception.Message
    }
}

# git archive gives the committed bytes (core.autocrlf=false) and leaves the clone as it is.
function Expand-FromGit([string]$Commit, [string[]]$Paths, [string]$Destination) {
    $zip = $Destination + '.zip'
    Remove-WorkItem $zip
    Remove-WorkItem $Destination
    $null = Invoke-Native git (@('-c', 'core.autocrlf=false', '-C', $Repo, 'archive', '--format=zip', '-o', $zip, $Commit) + $Paths) -Quiet
    Expand-Archive -LiteralPath $zip -DestinationPath $Destination
    Remove-WorkItem $zip
}

# A copy of the published gpui-pre-windows 0.3.8 with a patch on it.
function New-PatchedCrate([string]$Name, [string]$Patch, [string]$Marker) {
    $parent = Join-Path $WorkDir "crate-$Name"
    Remove-WorkItem $parent
    New-Item -ItemType Directory -Path $parent | Out-Null
    $null = Invoke-Native $Tar @('-xzf', $Crate, '-C', $parent) -Quiet
    $dir = Join-Path $parent 'gpui-pre-windows-0.3.8'
    # Its own repository, so that git apply counts the patch's paths from the crate's folder.
    $null = Invoke-Native git @('-C', $dir, 'init', '-q') -Quiet
    $null = Invoke-Native git @('-c', 'core.autocrlf=false', '-C', $dir, 'apply', '--check', $Patch) -Quiet
    $null = Invoke-Native git @('-c', 'core.autocrlf=false', '-C', $dir, 'apply', '--verbose', $Patch) -Log (Join-Path $Out "patch-$Name.log")
    Remove-WorkItem (Join-Path $dir '.git')
    if (-not (Select-String -LiteralPath (Join-Path $dir 'src\directx_atlas.rs') -SimpleMatch $Marker -Quiet)) {
        throw "the $Name patch is not in $dir ($Marker not found)"
    }
    $dir
}

function Invoke-Build([string]$Name, [string]$CrateDir) {
    [IO.File]::WriteAllBytes($LockPath, $LockPristine)
    # Programs from an earlier run go first, so that only what this build makes gets measured.
    foreach ($bin in 'minimal', $Poc) { Remove-WorkItem (Get-Exe $Name $bin) }
    $arguments = @('build', '--release', '--target', $RustHost, '--target-dir', $Targets[$Name])
    if ($CrateDir) {
        # In a file, so that the path needs no quoting on the command line.
        $config = Join-Path $WorkDir "patch-$Name.toml"
        $toml = '[patch.crates-io]' + "`n" + 'gpui-pre-windows = { path = "' + $CrateDir.Replace('\', '/') + '" }' + "`n"
        [IO.File]::WriteAllText($config, $toml, $Utf8)
        $arguments += @('--config', $config)
        Write-Log ('  ' + $config + ': ' + $toml.Replace("`n", ' ').Trim())
    } else {
        $arguments += '--locked'
    }
    Write-Log (Fmt 'Building "{0}": cargo {1}' $Name ($arguments -join ' '))
    Push-Location $PocDir
    try {
        $null = Invoke-Native cargo $arguments -Log (Join-Path $Out "build-$Name.log")
    } finally {
        Pop-Location
    }
    if ($CrateDir) {
        # A [patch] that was not used leaves the registry source in Cargo.lock.
        $lock = [IO.File]::ReadAllText($LockPath)
        if ($lock -match '(?m)^name = "gpui-pre-windows"\r?\nversion = "0\.3\.8"\r?\nsource = ') {
            throw "the $Name build did not use $CrateDir"
        }
        [IO.File]::WriteAllBytes($LockPath, $LockPristine)
    }
    foreach ($bin in 'minimal', $Poc) {
        $exe = Get-Exe $Name $bin
        if (-not (Test-Path -LiteralPath $exe)) { throw "$exe was not built" }
    }
}

function Add-Plan {
    param([string]$Build, [string]$Bin, [string]$Config, [hashtable]$Vars = @{}, [int[]]$At = @(6), [int]$Rep = 1, [switch]$Capture)
    $Plan.Add([pscustomobject]@{ Build = $Build; Bin = $Bin; Config = $Config; Vars = $Vars; At = $At; Rep = $Rep; Capture = [bool]$Capture })
}

# Starts one window with only the given POC_* variables, reads its memory at the given seconds
# after start, and closes it (GPUI quits when its last window closes).
function Invoke-Run($Item) {
    $script:RunCount++
    $exe = Get-Exe $Item.Build $Item.Bin
    $name = Fmt '{0}-{1}-{2}' $Item.Build $Item.Config $Item.Rep
    $dir = Join-Path $RunsDir $name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Get-ChildItem -Path Env:POC_* | Remove-Item
    foreach ($key in $Item.Vars.Keys) { Set-Item -Path "Env:$key" -Value ([string]$Item.Vars[$key]) }
    try {
        $process = Start-Process -FilePath $exe -WorkingDirectory $dir -NoNewWindow -PassThru `
            -RedirectStandardOutput (Join-Path $dir 'stdout.txt') -RedirectStandardError (Join-Path $dir 'stderr.txt')
    } finally {
        Get-ChildItem -Path Env:POC_* | Remove-Item
    }
    $null = $process.Handle  # keeps the exit code readable after the process ends
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $script:Live = $process
    $ended = 'closed'
    try {
        foreach ($seconds in $Item.At) {
            $wait = [int]($seconds * 1000 - $clock.ElapsedMilliseconds)
            if ($wait -gt 0) { Start-Sleep -Milliseconds $wait }
            $process.Refresh()
            if ($process.HasExited) { throw (Fmt '{0} ended (exit code {1}) before {2} s' $name $process.ExitCode $seconds) }
            $sample = [pscustomobject]@{
                run = $name; build = $Item.Build; config = $Item.Config; rep = $Item.Rep; process_id = $process.Id
                seconds = $seconds; elapsed_ms = $clock.ElapsedMilliseconds
                working_set_bytes = $process.WorkingSet64; private_bytes = $process.PrivateMemorySize64
            }
            $Samples.Add($sample)
            Write-Log (Fmt '[{0,2}/{1}] {2,-28} {3,2} s  working set {4,6:F1} MiB  private {5,6:F1} MiB' $script:RunCount $RunTotal $name $seconds ($sample.working_set_bytes / 1MB) ($sample.private_bytes / 1MB))
        }
        if ($Item.Capture) {
            $process.Refresh()
            try {
                $result = [Zed56349.Native]::Capture($process.MainWindowHandle, (Join-Path $CapturesDir "$name.png"))
            } catch {
                $result = 'failed: ' + $_.Exception.Message
            }
            Write-Log ('        picture: ' + $result)
        }
    } finally {
        if (-not $process.HasExited) {
            [void]$process.CloseMainWindow()
            if (-not $process.WaitForExit(5000)) {
                Stop-Process -Id $process.Id -Force
                $ended = 'killed (still running 5 s after the close)'
            }
        }
        $script:Live = $null
    }
    $Runs.Add([pscustomobject]@{ run = $name; build = $Item.Build; config = $Item.Config; rep = $Item.Rep; process_id = $process.Id; ended = $ended })
    Start-Sleep -Milliseconds 1500
}

# Median and range of a value over the runs of one window, in MiB.
function Get-Stat([string]$Build, [string]$Config, [int]$Seconds = 6, [string]$Field = 'working_set_bytes') {
    $values = @($Samples | Where-Object { $_.build -eq $Build -and $_.config -eq $Config -and $_.seconds -eq $Seconds } |
            ForEach-Object { $_.$Field / 1MB } | Sort-Object)
    $n = $values.Count
    if ($n -eq 0) { return '-' }
    if ($n % 2 -eq 1) { $median = $values[($n - 1) / 2] } else { $median = ($values[$n / 2 - 1] + $values[$n / 2]) / 2 }
    if ($n -eq 1) { return (Fmt '{0:F1}' $median) }
    Fmt '{0:F1} ({1:F1}-{2:F1})' $median $values[0] $values[$n - 1]
}

function Find-RunFile([string]$Run, [string]$Prefix) {
    $files = @(Get-ChildItem -LiteralPath (Join-Path $RunsDir $Run) -Filter "$Prefix-*.txt" -File -ErrorAction SilentlyContinue)
    if ($files.Count -eq 1) { return $files[0].FullName }
    $null
}

# The lines of memstat-<pid>.txt (the prototype's src/memstat.rs) that the comment talks about.
function Read-Memstat([string]$Path) {
    $result = [ordered]@{ RustHeap = '-'; Win32Heaps = '-'; Size4 = 'not listed'; Size5 = 'not listed' }
    if (-not $Path) { return [pscustomobject]$result }
    $text = [IO.File]::ReadAllText($Path)
    if ($text -match 'rust heap live\s+([\d.]+) MB') { $result.RustHeap = $Matches[1] }
    if ($text -match 'win32 heaps\s+([\d.]+) MB\s+committed') { $result.Win32Heaps = $Matches[1] }
    foreach ($m in [regex]::Matches($text, '(?m)^\s+(\d+) MB x (\d+)\s+ws\s+([\d.]+) MB')) {
        $cell = Fmt '{0} (working set {1} MiB)' $m.Groups[2].Value $m.Groups[3].Value
        if ($m.Groups[1].Value -eq '4') { $result.Size4 = $cell }
        if ($m.Groups[1].Value -eq '5') { $result.Size5 = $cell }
    }
    [pscustomobject]$result
}

# "uploads 1-64, 70" from upload numbers.
function Format-Indexes($Items) {
    $n = @($Items | ForEach-Object { $_.Index } | Sort-Object)
    if ($n.Count -eq 0) { return '' }
    if ($n.Count -eq 1) { return 'upload ' + $n[0] }
    $parts = New-Object System.Collections.Generic.List[string]
    $start = $n[0]
    $previous = $n[0]
    for ($i = 1; $i -le $n.Count; $i++) {
        if ($i -lt $n.Count -and $n[$i] -eq $previous + 1) {
            $previous = $n[$i]
            continue
        }
        if ($start -eq $previous) { $parts.Add([string]$start) } else { $parts.Add((Fmt '{0}-{1}' $start $previous)) }
        if ($i -lt $n.Count) {
            $start = $n[$i]
            $previous = $n[$i]
        }
    }
    if ($parts.Count -gt 6) { return 'uploads ' + (($parts | Select-Object -First 6) -join ', ') + (Fmt ' and {0} more ranges' ($parts.Count - 6)) }
    'uploads ' + ($parts -join ', ')
}

function Format-KiB($Items) {
    $values = @($Items | ForEach-Object { $_.Delta } | Sort-Object)
    if ($values.Count -eq 0) { return '' }
    if ($values[0] -eq $values[$values.Count - 1]) { return (Fmt '{0:+0;-0;0} KiB' $values[0]) }
    Fmt '{0:+0;-0;0} to {1:+0;-0;0} KiB' $values[0] $values[$values.Count - 1]
}

# atlas-<pid>.txt from the diagnostic patch: one line per texture and per upload, with the
# private bytes right before and after the call.
function Read-AtlasLog([string]$Path) {
    $sizes = @{}
    $textures = New-Object System.Collections.Generic.List[string]
    $uploads = New-Object System.Collections.Generic.List[object]
    $noContext1 = $false
    foreach ($line in [IO.File]::ReadAllLines($Path)) {
        if ($line -match 'texture (\w+)\[(\d+)\] created: (\d+)x(\d+) (\S+), (\d+) KiB; private \S+ -> \S+ MB \(([+-]?\d+|\?) KiB\)') {
            $key = $Matches[1] + '[' + $Matches[2] + ']'
            $sizes[$key] = [int]$Matches[6]
            $textures.Add((Fmt '{0} {1}x{2} {3}, {4} KiB (private {5} KiB)' $key $Matches[3] $Matches[4] $Matches[5] $Matches[6] $Matches[7]))
        } elseif ($line -match 'no ID3D11DeviceContext1') {
            $noContext1 = $true
        } elseif ($line -match 'upload (\d+) to (\w+)\[(\d+)\] with ([^:]+): \d+x\d+ at \(-?\d+, -?\d+\); private \S+ -> \S+ MB \(([+-]?\d+|\?) KiB, sum') {
            $delta = $null
            if ($Matches[5] -ne '?') { $delta = [int]$Matches[5] }
            $uploads.Add([pscustomobject]@{ Index = [int]$Matches[1]; Texture = ($Matches[2] + '[' + $Matches[3] + ']'); Api = $Matches[4]; Delta = $delta })
        }
    }
    $near = New-Object System.Collections.Generic.List[object]
    $other = New-Object System.Collections.Generic.List[object]
    $zero = 0
    $shrank = 0
    $unknown = 0
    $sum = 0
    foreach ($upload in $uploads) {
        if ($null -eq $upload.Delta) { $unknown++; continue }
        $sum += $upload.Delta
        $size = 0
        if ($sizes.ContainsKey($upload.Texture)) { $size = $sizes[$upload.Texture] }
        if ($size -gt 0 -and [math]::Abs($upload.Delta - $size) -le $size * 0.1) { $near.Add($upload) }
        elseif ($upload.Delta -gt 0) { $other.Add($upload) }
        elseif ($upload.Delta -eq 0) { $zero++ }
        else { $shrank++ }
    }
    $calls = (@($uploads | ForEach-Object { $_.Api } | Sort-Object -Unique)) -join ', '
    if ($noContext1) { $calls += ' (log says: no ID3D11DeviceContext1)' }
    $nearCell = '0'
    if ($near.Count -gt 0) { $nearCell = Fmt '{0} ({1}; {2})' $near.Count (Format-KiB $near) (Format-Indexes $near) }
    $otherCell = '0'
    if ($other.Count -gt 0) { $otherCell = Fmt '{0} ({1}; {2})' $other.Count (Format-KiB $other) (Format-Indexes $other) }
    [pscustomobject]@{
        Calls = $calls
        Textures = $textures -join '; '
        Uploads = $uploads.Count
        Near = $nearCell
        Other = $otherCell
        Zero = $zero
        ShrankUnknown = Fmt '{0} / {1}' $shrank $unknown
        Sum = Fmt '{0:F1} MiB' ($sum / 1024)
    }
}

function Write-Summary {
    $md = New-Object System.Collections.Generic.List[string]
    $md.Add('# zed-industries/zed#56349: evidence from one run')
    $md.Add('')
    $scriptNote = ''
    if (-not $ScriptAtHead) { $scriptNote = ' (the script file differs from that commit; its SHA-256 is in `versions.txt`)' }
    $runsNote = 'Each window was run once.'
    if ($Repeat -gt 1) { $runsNote = Fmt 'Most windows were run {0} times; a cell with several runs shows the median and the range (min-max).' $Repeat }
    $md.Add((Fmt 'Collected {0} by `docs/upstream/zed-56349-evidence.ps1` from stofu5678/matsu_editor@{1}{2}. The script was written with Claude Code (an AI) and reviewed with Codex (an AI); the numbers come from running it on the machine below. Memory is in MiB (1024 x 1024 bytes). {3} Raw data: `measurements.csv`, `runs/`, `captures/`, `log.txt`; the script and the patches: `inputs/`.' $Started.ToString('yyyy-MM-dd HH:mm zzz', $Inv) $HeadCommit.Substring(0, 12) $scriptNote $runsNote))
    $md.Add('')
    $md.Add('## Machine')
    $md.Add('')
    foreach ($key in $Machine.Keys) { $md.Add((Fmt '- {0}: {1}' $key $Machine[$key])) }
    $md.Add('')
    $md.Add('Comment: "Surface-class Windows 11 ARM64 (26200), Qualcomm Adreno X1-85, driver 31.0.160.0"; "`crates/gpui_windows/src/directx_atlas.rs` on current `main` still uploads with `UpdateSubresource`" (posted 2026-10-09).')
    $md.Add('')
    $md.Add('## Builds')
    $md.Add('')
    $md.Add((Fmt '- Prototype: stofu5678/matsu_editor@{0}, `poc/gpui-textview`, taken out with `git archive`. `minimal` is a GPUI window with one line of text; `gpui-textview-poc` is the prototype editor (a menu bar and a text view).' $PocCommit.Substring(0, 7)))
    $md.Add((Fmt '- gpui: as published, `cargo build --release --locked` (gpui-pre {0}, built from zed {1}).' $GpuiPre $ZedRev))
    $md.Add((Fmt '- fix: gpui-pre-windows 0.3.8 from the `.crate` in Cargo''s cache (SHA-256 {0}, the checksum in Cargo.lock) with `gpui-pre-windows-atlas-upload.diff` at {1} (SHA-256 {2}), the patch the comment links to.' $CrateSha $PocCommit.Substring(0, 7) $FixSha))
    $md.Add((Fmt '- diag: the same crate with `gpui-pre-windows-atlas-diag.diff` (SHA-256 {0}), which logs the private bytes right before and after each atlas texture creation and upload (see section 3).' $DiagSha))
    $md.Add((Fmt '- All six programs are {0} (PE header); Rust {1}, host {2}.' (($Machines | Sort-Object -Unique) -join ', ') $RustRelease $RustHost))
    $md.Add('')

    $md.Add('## 1. Memory 6 s after start')
    $md.Add('')
    $md.Add('Working set and private bytes of the process (Get-Process; Task Manager calls them "Working set (memory)" and "Commit size"), read 6 s after start. Then the window was closed.')
    $md.Add('')
    $md.Add('| Window | gpui: working set | gpui: private | fix: working set | fix: private | Comment: working set, current -> staging |')
    $md.Add('|---|---|---|---|---|---|')
    $rows = @(
        @{ Label = 'Minimal GPUI window, one line of text'; Config = 'minimal'; Comment = '84 MB -> 48 MB' },
        @{ Label = 'Same, plus a line of Japanese in the system UI font'; Config = 'minimal-jp'; Comment = '147 MB -> 48 MB' },
        @{ Label = 'Prototype editor at startup'; Config = 'poc'; Comment = '307 MB -> 54 MB' }
    )
    foreach ($row in $rows) {
        $md.Add((Fmt '| {0} | {1} | {2} | {3} | {4} | {5} |' $row.Label (Get-Stat gpui $row.Config) (Get-Stat gpui $row.Config 6 private_bytes) (Get-Stat fix $row.Config) (Get-Stat fix $row.Config 6 private_bytes) $row.Comment))
    }
    $md.Add((Fmt '| Second row, `TextRenderingMode::Grayscale` | {0} | {1} | not run | not run | no number ("smaller but still present") |' (Get-Stat gpui minimal-jp-gray) (Get-Stat gpui minimal-jp-gray 6 private_bytes)))
    $md.Add((Fmt '| Prototype, `TextRenderingMode::Grayscale` | {0} | {1} | not run | not run | no number |' (Get-Stat gpui poc-gray) (Get-Stat gpui poc-gray 6 private_bytes)))
    $md.Add('')
    $md.Add((Fmt 'Prototype (gpui), one more run read at 6 s and 40 s: working set {0} -> {1}, private {2} -> {3}. Comment: "it is never released (still there after 40 s)".' (Get-Stat gpui poc-40s 6) (Get-Stat gpui poc-40s 40) (Get-Stat gpui poc-40s 6 private_bytes) (Get-Stat gpui poc-40s 40 private_bytes)))
    $md.Add('')

    $md.Add('## 2. Memory breakdown 6 s after start (memstat)')
    $md.Add('')
    $md.Add('From `memstat-<pid>.txt`, which the prototype writes with `POC_MEMSTAT=6` (`src/memstat.rs`, also written with an AI). Rust heap: a counting global allocator. Win32 heaps: `HeapSummary`. Private allocations: `VirtualQuery` regions grouped by allocation, counted by committed size rounded up to whole MB, so an allocation a little over 4 MiB is counted as 5 MB; only the 12 sizes with the largest working set are listed.')
    $md.Add('')
    $gpuiMem = Read-Memstat (Find-RunFile 'gpui-poc-memstat-1' 'memstat')
    $fixMem = Read-Memstat (Find-RunFile 'fix-poc-memstat-1' 'memstat')
    $diagMem = Read-Memstat (Find-RunFile 'diag-poc-memstat-log-1' 'memstat')
    $md.Add('| Prototype | gpui | fix | diag (with the atlas log; its row is in section 3) | Comment |')
    $md.Add('|---|---|---|---|---|')
    $md.Add((Fmt '| Rust heap live, MB | {0} | {1} | {2} | ~3 MB |' $gpuiMem.RustHeap $fixMem.RustHeap $diagMem.RustHeap))
    $md.Add((Fmt '| Win32 heaps committed, MB | {0} | {1} | {2} | ~8 MB |' $gpuiMem.Win32Heaps $fixMem.Win32Heaps $diagMem.Win32Heaps))
    $md.Add((Fmt '| Private allocations counted as 5 MB | {0} | {1} | {2} | "~4 MB private allocations: 61 of them (~244 MB)" |' $gpuiMem.Size5 $fixMem.Size5 $diagMem.Size5))
    $md.Add((Fmt '| Private allocations counted as 4 MB | {0} | {1} | {2} | |' $gpuiMem.Size4 $fixMem.Size4 $diagMem.Size4))
    $md.Add('')

    $md.Add('## 3. Atlas texture creations and uploads (diag build)')
    $md.Add('')
    $md.Add('Each line of `atlas-<pid>.txt` has the private bytes (`K32GetProcessMemoryInfo`, `PrivateUsage`) right before and right after one call. "About the texture size" means within 10 % of the size of the texture that the upload wrote to. The sum adds up the changes during the calls only. The logging itself puts time between uploads; section 4 compares the same calls with and without it, the other settings being the same.')
    $md.Add('')
    $md.Add('| Run | Call | Textures created | Uploads | Grew by about the texture size | Grew by other amounts | Did not grow | Shrank / unknown | Sum |')
    $md.Add('|---|---|---|---|---|---|---|---|---|')
    foreach ($run in @($Runs | Where-Object { $_.build -eq 'diag' -and $_.config -like '*-log' })) {
        $file = Find-RunFile $run.run 'atlas'
        if (-not $file) {
            $md.Add((Fmt '| {0} | no atlas log | | | | | | | |' $run.run))
            continue
        }
        $atlas = Read-AtlasLog $file
        $md.Add((Fmt '| {0} | {1} | {2} | {3} | {4} | {5} | {6} | {7} | {8} |' $run.run $atlas.Calls $atlas.Textures $atlas.Uploads $atlas.Near $atlas.Other $atlas.Zero $atlas.ShrankUnknown $atlas.Sum))
    }
    $md.Add('')
    $md.Add('Comment: "Only one atlas texture is created (Subpixel, 1024x1024 RGBA = 4 MB). Logging `PrivateUsage` around each call showed that every `UpdateSubresource` into that texture adds exactly +4.0 MB"; "With `TextRenderingMode::Grayscale` the atlas is R8 (1 MB per page) and the growth is smaller but still present"; "CJK text makes it much worse: more distinct glyphs, so more uploads".')
    $md.Add('')

    $md.Add('## 4. The upload calls with and without the logging (diag build, prototype, 6 s after start)')
    $md.Add('')
    $md.Add('| Call | Logged: working set | Not logged (`POC_ATLAS_LOG=0`): working set | Not logged: private |')
    $md.Add('|---|---|---|---|')
    foreach ($row in @(
            @{ Label = '`UpdateSubresource` (as GPUI)'; Logged = 'poc-log'; Plain = 'poc-nolog' },
            @{ Label = '`UpdateSubresource1` with `D3D11_COPY_NO_OVERWRITE`'; Logged = 'poc-update1-log'; Plain = 'poc-update1-nolog' },
            @{ Label = '`UpdateSubresource`, then `Flush()`'; Logged = 'poc-flush-log'; Plain = 'poc-flush-nolog' })) {
        $md.Add((Fmt '| {0} | {1} | {2} | {3} |' $row.Label (Get-Stat diag $row.Logged) (Get-Stat diag $row.Plain) (Get-Stat diag $row.Plain 6 private_bytes)))
    }
    $md.Add('')
    $md.Add((Fmt '`UpdateSubresource`, then `Flush()`, not logged, one more run read at 6 s and 40 s: private {0} -> {1}.' (Get-Stat diag poc-flush-nolog-40s 6 private_bytes) (Get-Stat diag poc-flush-nolog-40s 40 private_bytes)))
    $leaked = @($Runs | Where-Object { $_.config -like '*-nolog*' } | Where-Object { Find-RunFile $_.run 'atlas' })
    $md.Add((Fmt 'Runs with `POC_ATLAS_LOG=0` that still wrote an atlas log: {0}.' $leaked.Count))
    $md.Add('')
    $md.Add('Comment: "What did not help: `ID3D11DeviceContext1::UpdateSubresource1` with `D3D11_COPY_NO_OVERWRITE`; `Flush()` after each upload".')
    $md.Add('')

    $md.Add('## 5. Rendering')
    $md.Add('')
    $md.Add('Pictures of the first run of each window (PrintWindow with `PW_RENDERFULLCONTENT`, after the 6 s reading), gpui against fix:')
    $md.Add('')
    foreach ($row in @(@{ Label = 'Minimal GPUI window'; Config = 'minimal' }, @{ Label = 'Same, plus a line of Japanese'; Config = 'minimal-jp' }, @{ Label = 'Prototype editor'; Config = 'poc' })) {
        $first = Join-Path $CapturesDir "gpui-$($row.Config)-1.png"
        $second = Join-Path $CapturesDir "fix-$($row.Config)-1.png"
        $result = 'no picture'
        if ((Test-Path -LiteralPath $first) -and (Test-Path -LiteralPath $second)) {
            try { $result = [Zed56349.Native]::Compare($first, $second) } catch { $result = 'compare failed: ' + $_.Exception.Message }
        }
        $md.Add((Fmt '- {0}: {1} (`captures/gpui-{2}-1.png`, `captures/fix-{2}-1.png`)' $row.Label $result $row.Config))
    }
    $md.Add('')
    $md.Add('The prototype''s status bar shows the time to its first frame in milliseconds, which differs from run to run. The window title of the prototype at 97908ca still has the project''s old name.')
    $md.Add('')
    $md.Add('Comment: "rendering looks the same".')
    $md.Add('')

    $md.Add('## 6. Zed main')
    $md.Add('')
    $md.Add('- ' + $Machine['Zed main'])
    $md.Add('')

    $md.Add('## Runs')
    $md.Add('')
    $killed = @($Runs | Where-Object { $_.ended -ne 'closed' })
    if ($killed.Count -eq 0) {
        $md.Add((Fmt '{0} windows were started one at a time, and each ended within 5 s of being closed. Total time: {1:F0} minutes.' $Runs.Count ((Get-Date) - $Started).TotalMinutes))
    } else {
        $md.Add((Fmt '{0} windows were started one at a time; {1} did not end within 5 s of being closed and were killed ({2}). Total time: {3:F0} minutes.' $Runs.Count $killed.Count (($killed | ForEach-Object { $_.run }) -join ', ') ((Get-Date) - $Started).TotalMinutes))
    }
    $md.Add('')
    $md.Add('## Not shown by this')
    $md.Add('')
    $md.Add('- What the driver or the D3D11 runtime does inside a call: the logs show only the process''s private bytes around each call.')
    $md.Add('- Other GPUs.')
    [IO.File]::WriteAllLines((Join-Path $Out 'summary.md'), $md.ToArray(), $Utf8)
}

# Replaces the user folder, the user name in paths and the computer name in the text files.
# Returns the files where the user name is still found, to be looked at before posting.
function Invoke-Redaction {
    $rules = New-Object System.Collections.Generic.List[object]
    if ($env:USERPROFILE) {
        $rules.Add(@(('(?i)' + [regex]::Escape($env:USERPROFILE)), '%USERPROFILE%'))
        $rules.Add(@(('(?i)' + [regex]::Escape($env:USERPROFILE.Replace('\', '/'))), '%USERPROFILE%'))
    }
    if ($env:USERNAME) { $rules.Add(@(('(?i)(?<=[\\/]Users[\\/])' + [regex]::Escape($env:USERNAME) + '(?=[\\/])'), '<user>')) }
    if ($env:COMPUTERNAME) { $rules.Add(@(('(?i)(?<![A-Za-z0-9_-])' + [regex]::Escape($env:COMPUTERNAME) + '(?![A-Za-z0-9_-])'), '<computer>')) }
    $files = @(Get-ChildItem -LiteralPath $Out -Recurse -File | Where-Object { $_.Extension -in '.txt', '.log', '.md', '.csv' })
    foreach ($file in $files) {
        $text = [IO.File]::ReadAllText($file.FullName)
        $new = $text
        foreach ($rule in $rules) { $new = [regex]::Replace($new, $rule[0], $rule[1]) }
        if ($new -ne $text) { [IO.File]::WriteAllText($file.FullName, $new, $Utf8) }
    }
    $left = @()
    if ($env:USERNAME) {
        $pattern = '(?i)' + [regex]::Escape($env:USERNAME)
        $left = @($files | Where-Object { [IO.File]::ReadAllText($_.FullName) -match $pattern } | ForEach-Object { $_.FullName.Substring($Out.Length + 1) })
    }
    , $left
}

$WorkDir = [IO.Path]::GetFullPath($WorkDir)
$Targets = $null
$OldEncoding = $null
try {
    # Git and Cargo write UTF-8.
    $OldEncoding = [Console]::OutputEncoding
    [Console]::OutputEncoding = $Utf8
} catch {
    $OldEncoding = $null
}
$env:CARGO_TERM_COLOR = 'never'
$Failed = $false

try {
    # Checks first, before anything is written.
    foreach ($tool in 'git', 'cargo', 'rustc') {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "$tool is not on PATH" }
    }
    $Tar = Join-Path $env:SystemRoot 'System32\tar.exe'
    if (-not (Test-Path -LiteralPath $Tar)) { throw "$Tar is missing" }
    if (-not ('Zed56349.Native' -as [type])) { Add-Type -TypeDefinition $NativeSource -ReferencedAssemblies System.Drawing }
    # Real paths, so that a junction cannot hide that the work folder and the clone overlap.
    $WorkDir = [Zed56349.Native]::RealPath($WorkDir)
    # PowerShell reads [ ] * ? in many paths as wildcards.
    if ($WorkDir.IndexOfAny([char[]]'[]*?') -ge 0) { throw "-WorkDir must not have [ ] * or ? in it: $WorkDir" }
    if ([IO.Path]::GetPathRoot($WorkDir).TrimEnd('\') -eq $WorkDir.TrimEnd('\')) { throw "-WorkDir must not be a drive root: $WorkDir" }
    $Repo = [Zed56349.Native]::RealPath(((Invoke-Native git @('-C', $PSScriptRoot, 'rev-parse', '--show-toplevel') -Quiet) -join ''))
    if (Test-Overlap $WorkDir $Repo) { throw "-WorkDir ($WorkDir) and the clone ($Repo) must not be inside each other" }
    $Targets = @{ gpui = (Join-Path $WorkDir 'target'); fix = (Join-Path $WorkDir 'target-fix'); diag = (Join-Path $WorkDir 'target-diag') }
    $null = Invoke-Native git @('-C', $Repo, 'cat-file', '-e', ($PocCommit + '^{commit}')) -Quiet
    $others = @(Get-Process -Name 'minimal', $Poc -ErrorAction SilentlyContinue)
    if ($others.Count -gt 0) {
        throw ('close these first: ' + (($others | ForEach-Object { Fmt '{0}.exe (process ID {1})' $_.ProcessName $_.Id }) -join ', '))
    }
    $driveRoot = [IO.Path]::GetPathRoot($WorkDir)
    $free = (New-Object IO.DriveInfo($driveRoot)).AvailableFreeSpace
    if ($free -lt 8GB) { throw (Fmt 'about 8 GB must be free on {0}; {1:F1} GB is' $driveRoot ($free / 1GB)) }

    $Out = Join-Path $WorkDir ('evidence-' + $Started.ToString('yyyyMMdd-HHmmss', $Inv))
    $RunsDir = Join-Path $Out 'runs'
    $CapturesDir = Join-Path $Out 'captures'
    $InputsDir = Join-Path $Out 'inputs'
    New-Item -ItemType Directory -Force -Path $Out, $RunsDir, $CapturesDir, $InputsDir | Out-Null
    $LogPath = Join-Path $Out 'log.txt'
    Write-Log ('zed-56349-evidence, ' + $Started.ToString('yyyy-MM-dd HH:mm:ss zzz', $Inv))
    Write-Log 'This takes 20 to 25 minutes. Windows open and close one at a time; leave the machine alone until it ends.'
    Write-Log ('Work folder: ' + $WorkDir)
    Write-Log ''

    Write-Log 'Taking the prototype at 97908ca and the diagnostic patch at HEAD out of the clone.'
    $Src = Join-Path $WorkDir 'src'
    Expand-FromGit $PocCommit @('rust-toolchain.toml', 'poc/gpui-textview') $Src
    $PocDir = Join-Path $Src 'poc\gpui-textview'
    $HeadFiles = Join-Path $WorkDir 'head'
    Expand-FromGit 'HEAD' @($DiagPatchInRepo) $HeadFiles
    $FixPatch = Join-Path $Src $FixPatchInRepo.Replace('/', '\')
    $DiagPatch = Join-Path $HeadFiles $DiagPatchInRepo.Replace('/', '\')
    $LockPath = Join-Path $PocDir 'Cargo.lock'
    $LockPristine = [IO.File]::ReadAllBytes($LockPath)
    Copy-Item -LiteralPath $PSCommandPath, $FixPatch, $DiagPatch -Destination $InputsDir
    $FixSha = Get-Sha256 $FixPatch
    $DiagSha = Get-Sha256 $DiagPatch

    $HeadCommit = (Invoke-Native git @('-C', $Repo, 'rev-parse', 'HEAD') -Quiet) -join ''
    $Branch = (Invoke-Native git @('-C', $Repo, 'rev-parse', '--abbrev-ref', 'HEAD') -Quiet) -join ''
    # The same real-path spelling as $Repo, whatever path the script was started with.
    $scriptReal = [Zed56349.Native]::RealPath($PSCommandPath)
    if (-not $scriptReal.StartsWith($Repo.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "the script ($scriptReal) is not inside the clone ($Repo)" }
    $ScriptInRepo = $scriptReal.Substring($Repo.TrimEnd('\').Length + 1).Replace('\', '/')
    $ScriptStatus = Invoke-Native git @('-C', $Repo, 'status', '--porcelain', '--', $ScriptInRepo) -Quiet

    Write-Log 'Recording the machine.'
    Push-Location $PocDir
    try {
        $Rustc = Invoke-Native rustc @('-vV') -Quiet
        $CargoVersion = (Invoke-Native cargo @('-V') -Quiet) -join ''
    } finally {
        Pop-Location
    }
    $RustRelease = (@($Rustc | Where-Object { $_ -like 'release:*' }) -replace '^release:\s*', '') -join ''
    $RustHost = (@($Rustc | Where-Object { $_ -like 'host:*' }) -replace '^host:\s*', '') -join ''
    $Machine = Get-MachineInfo
    $Machine['Rust'] = Fmt 'rustc {0}, host {1}; {2}' $RustRelease $RustHost $CargoVersion
    $Machine['Git'] = (Invoke-Native git @('--version') -Quiet) -join ''
    $Machine['Zed main'] = Get-ZedMain
    foreach ($key in $Machine.Keys) { Write-Log (Fmt '  {0}: {1}' $key $Machine[$key]) }
    Write-Log ''

    Invoke-Build 'gpui' ''
    $LockText = [Text.Encoding]::UTF8.GetString($LockPristine)
    if ($LockText -match '(?m)^name = "gpui-pre-windows"\r?\nversion = "0\.3\.8"\r?\nsource = "registry\+https://github\.com/rust-lang/crates\.io-index"\r?\nchecksum = "([0-9a-f]{64})"') {
        $CrateSha = $Matches[1]
    } else {
        throw 'gpui-pre-windows 0.3.8 from crates.io is not in Cargo.lock'
    }
    $GpuiPre = '?'
    if ($LockText -match '(?m)^name = "gpui-pre"\r?\nversion = "([^"]+)"') { $GpuiPre = $Matches[1] }
    $CargoHome = $env:CARGO_HOME
    if (-not $CargoHome) { $CargoHome = Join-Path $env:USERPROFILE '.cargo' }
    $Crate = $null
    foreach ($candidate in @(Get-ChildItem -LiteralPath (Join-Path $CargoHome 'registry\cache') -Filter 'gpui-pre-windows-0.3.8.crate' -Recurse -File -ErrorAction SilentlyContinue)) {
        if ((Get-Sha256 $candidate.FullName) -eq $CrateSha) {
            $Crate = $candidate.FullName
            break
        }
    }
    if (-not $Crate) { throw "no gpui-pre-windows-0.3.8.crate with SHA-256 $CrateSha in $CargoHome\registry\cache" }
    Write-Log ('Published crate: ' + $Crate + ' (SHA-256 matches Cargo.lock)')
    $FixCrate = New-PatchedCrate 'fix' $FixPatch 'D3D11_USAGE_STAGING'
    $DiagCrate = New-PatchedCrate 'diag' $DiagPatch 'POC_ATLAS_UPLOAD'
    $ZedRev = '?'
    if ([IO.File]::ReadAllText((Join-Path $FixCrate 'Cargo.toml')) -match 'zed-rev = "([0-9a-f]+)"') { $ZedRev = $Matches[1] }
    Invoke-Build 'fix' $FixCrate
    Invoke-Build 'diag' $DiagCrate
    Write-Log ''

    $Versions = New-Object System.Collections.Generic.List[string]
    $Versions.Add('clone: ' + $HeadCommit + ' (' + $Branch + ')')
    # The git object ID compares with the file on GitHub; the SHA-256 is of the file as run, whose
    # line ends may differ (core.autocrlf).
    $inCommit = $true
    try {
        $blob = (Invoke-Native git @('-C', $Repo, 'rev-parse', "HEAD:$ScriptInRepo") -Quiet) -join ''
    } catch {
        $blob = 'not in the commit'
        $inCommit = $false
    }
    # git status says nothing for a clean file, and also for a path it does not know.
    $ScriptAtHead = $inCommit -and (@($ScriptStatus).Count -eq 0)
    $atHead = 'no, it differs from the commit or is not committed'
    if ($ScriptAtHead) { $atHead = 'yes' }
    $Versions.Add((Fmt 'script: {0}, git object {1}, SHA-256 of the file {2}, same as the commit: {3}' $ScriptInRepo $blob (Get-Sha256 $PSCommandPath) $atHead))
    $Versions.Add('prototype: ' + $PocCommit)
    $Versions.Add('gpui-pre: ' + $GpuiPre + ', zed-rev ' + $ZedRev)
    $Versions.Add('gpui-pre-windows-0.3.8.crate: SHA-256 ' + $CrateSha)
    $Versions.Add('fix patch: SHA-256 ' + $FixSha)
    $Versions.Add('diag patch: SHA-256 ' + $DiagSha)
    $Machines = New-Object System.Collections.Generic.List[string]
    foreach ($build in 'gpui', 'fix', 'diag') {
        foreach ($bin in 'minimal', $Poc) {
            $exe = Get-Exe $build $bin
            $machineType = Get-PeMachine $exe
            $Machines.Add($machineType)
            $Versions.Add((Fmt '{0} {1}.exe: {2}, SHA-256 {3}' $build $bin $machineType (Get-Sha256 $exe)))
        }
    }
    $Versions.Add('rustc -vV:')
    foreach ($line in $Rustc) { $Versions.Add('  ' + $line) }
    # Settings that change how Cargo builds, if any are set (names chosen so that no token is written).
    $buildVariables = @(Get-ChildItem Env: | Where-Object {
            $_.Name -in 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS', 'CARGO_BUILD_TARGET', 'RUSTUP_TOOLCHAIN', 'CARGO_INCREMENTAL' -or $_.Name -like 'CARGO_PROFILE_*'
        })
    $Versions.Add('build settings in the environment: ' + $(if ($buildVariables.Count) { ($buildVariables | ForEach-Object { $_.Name + '=' + $_.Value }) -join '; ' } else { 'none' }))
    [IO.File]::WriteAllLines((Join-Path $Out 'versions.txt'), $Versions.ToArray(), $Utf8)
    $environmentLines = @($Machine.Keys | ForEach-Object { Fmt '{0}: {1}' $_ $Machine[$_] })
    [IO.File]::WriteAllLines((Join-Path $Out 'environment.txt'), [string[]]$environmentLines, $Utf8)

    # gpui and fix take turns, so that both see the same state of the machine.
    foreach ($window in @(
            @{ Bin = 'minimal'; Config = 'minimal'; Vars = @{} },
            @{ Bin = 'minimal'; Config = 'minimal-jp'; Vars = @{ POC_DEFAULT_FONT = '1' } },
            @{ Bin = $Poc; Config = 'poc'; Vars = @{} })) {
        for ($rep = 1; $rep -le $Repeat; $rep++) {
            foreach ($build in 'gpui', 'fix') {
                Add-Plan $build $window.Bin $window.Config -Vars $window.Vars -Rep $rep -Capture:($rep -eq 1)
            }
        }
    }
    foreach ($build in 'gpui', 'fix') { Add-Plan $build $Poc 'poc-memstat' -Vars @{ POC_MEMSTAT = '6' } -At 6, 8 }
    Add-Plan 'gpui' $Poc 'poc-40s' -At 6, 40
    for ($rep = 1; $rep -le $Repeat; $rep++) {
        Add-Plan 'gpui' 'minimal' 'minimal-jp-gray' -Vars @{ POC_DEFAULT_FONT = '1'; POC_TEXT_MODE = 'grayscale' } -Rep $rep
        Add-Plan 'gpui' $Poc 'poc-gray' -Vars @{ POC_TEXT_MODE = 'grayscale' } -Rep $rep
    }
    for ($rep = 1; $rep -le $Repeat; $rep++) {
        Add-Plan 'diag' $Poc 'poc-log' -Rep $rep
        Add-Plan 'diag' $Poc 'poc-update1-log' -Vars @{ POC_ATLAS_UPLOAD = 'update1' } -Rep $rep
        Add-Plan 'diag' $Poc 'poc-flush-log' -Vars @{ POC_ATLAS_UPLOAD = 'flush' } -Rep $rep
    }
    # The breakdown and the atlas log of one process (section 2 and 3); memstat is kept out of the
    # runs that section 4 compares, since it walks the memory at the 6 s reading.
    Add-Plan 'diag' $Poc 'poc-memstat-log' -Vars @{ POC_MEMSTAT = '6' } -At 6, 8
    Add-Plan 'diag' $Poc 'poc-gray-log' -Vars @{ POC_TEXT_MODE = 'grayscale' }
    Add-Plan 'diag' 'minimal' 'minimal-log'
    Add-Plan 'diag' 'minimal' 'minimal-jp-log' -Vars @{ POC_DEFAULT_FONT = '1' }
    for ($rep = 1; $rep -le $Repeat; $rep++) {
        Add-Plan 'diag' $Poc 'poc-nolog' -Vars @{ POC_ATLAS_LOG = '0' } -Rep $rep
        Add-Plan 'diag' $Poc 'poc-update1-nolog' -Vars @{ POC_ATLAS_LOG = '0'; POC_ATLAS_UPLOAD = 'update1' } -Rep $rep
        Add-Plan 'diag' $Poc 'poc-flush-nolog' -Vars @{ POC_ATLAS_LOG = '0'; POC_ATLAS_UPLOAD = 'flush' } -Rep $rep
    }
    Add-Plan 'diag' $Poc 'poc-flush-nolog-40s' -Vars @{ POC_ATLAS_LOG = '0'; POC_ATLAS_UPLOAD = 'flush' } -At 6, 40
    $RunTotal = $Plan.Count
    Write-Log (Fmt 'Measuring: {0} windows, one at a time.' $RunTotal)
    foreach ($item in $Plan) { Invoke-Run $item }
    Write-Log ''

    $Samples | Export-Csv -LiteralPath (Join-Path $Out 'measurements.csv') -NoTypeInformation -Encoding UTF8
    $Runs | Export-Csv -LiteralPath (Join-Path $Out 'runs.csv') -NoTypeInformation -Encoding UTF8
    Write-Summary
    Write-Log ('Done in ' + (Fmt '{0:F0}' ((Get-Date) - $Started).TotalMinutes) + ' minutes.')
    $left = Invoke-Redaction
    $Zip = $Out + '.zip'
    if (Test-Path -LiteralPath $Zip) { Remove-Item -LiteralPath $Zip -Force }
    Compress-Archive -LiteralPath @(Get-ChildItem -LiteralPath $Out | ForEach-Object { $_.FullName }) -DestinationPath $Zip
    # From here on, the console only: the files are cleaned and zipped.
    Write-Host ('Summary: ' + (Join-Path $Out 'summary.md'))
    Write-Host ('Everything, zipped: ' + $Zip)
    if ($left.Count -gt 0) {
        Write-Host ('The user name still appears in: ' + ($left -join ', ') + '. Look at these before posting.')
    }
    Write-Host 'Read summary.md, and look through the folder before posting any of it.'
} catch {
    $Failed = $true
    Write-Log ('FAILED: ' + $_.Exception.Message)
    Write-Log $_.ScriptStackTrace
    if ($Out -and (Test-Path -LiteralPath $Out)) {
        try {
            $null = Invoke-Redaction
        } catch {
            Write-Host ('Could not clean the files: ' + $_.Exception.Message)
        }
        Write-Host ('The files so far are in ' + $Out)
    }
} finally {
    if ($Live -and -not $Live.HasExited) { Stop-Process -Id $Live.Id -Force -ErrorAction SilentlyContinue }
    Get-ChildItem -Path Env:POC_* | Remove-Item
    if ($LockPath -and $LockPristine) { [IO.File]::WriteAllBytes($LockPath, $LockPristine) }
    if ($OldEncoding) {
        try { [Console]::OutputEncoding = $OldEncoding } catch { $OldEncoding = $null }
    }
}
if ($Failed) { exit 1 }
