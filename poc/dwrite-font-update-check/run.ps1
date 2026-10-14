# Builds dwrite-font-update-check and runs it in fresh processes, with
# checkForUpdates = true and false. Prints the environment and the results.
# Usage: powershell -ExecutionPolicy Bypass -File run.ps1 [-Runs 10]
param([int]$Runs = 10)

if ($Runs -lt 1) { throw "Runs must be 1 or more" }
Set-Location $PSScriptRoot

$os = Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion"
"OS: $((Get-CimInstance Win32_OperatingSystem).Caption) $($os.DisplayVersion) build $($os.CurrentBuild).$($os.UBR)"
"CPU: " + (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name).Trim()
$disk = Get-Partition -DriveLetter C | Get-Disk
"Disk of C: $($disk.FriendlyName) ($((Get-PhysicalDisk | Where-Object DeviceId -eq $disk.Number).MediaType))"
"Font files in C:\Windows\Fonts: " + (Get-ChildItem C:\Windows\Fonts -File).Count
"Fonts registered (HKLM): " + (Get-Item "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts").ValueCount
"Fonts registered (HKCU): " + [int](Get-Item "HKCU:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts" -ErrorAction SilentlyContinue).ValueCount
"FontCache service: " + (Get-Service FontCache).Status
"rustc: " + (rustc --version)
""

cargo build --release --quiet --target-dir target
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
$exe = Join-Path $PSScriptRoot "target\release\dwrite-font-update-check.exe"

function Measure-Once([string]$mode) {
    $line = & $exe $mode
    if ($LASTEXITCODE -ne 0) { throw "dwrite-font-update-check $mode failed" }
    $line
    if ($line -notmatch "GetSystemFontCollection=([\d.]+) ms") { throw "unexpected output: $line" }
    $script:last = [double]$Matches[1]
}

# The first run of each mode is printed but not counted: it can be slower
# after a build or a reboot.
"-- first runs (not counted)"
Measure-Once "true"
Measure-Once "false"

"-- counted runs (the order of the pair alternates)"
$results = @{ "true" = @(); "false" = @() }
for ($i = 1; $i -le $Runs; $i++) {
    $order = if ($i % 2) { "true", "false" } else { "false", "true" }
    foreach ($mode in $order) {
        Measure-Once $mode
        $results[$mode] += $script:last
    }
}

""
foreach ($mode in "true", "false") {
    $sorted = @($results[$mode] | Sort-Object)
    if ($sorted.Count -ne $Runs) { throw "expected $Runs samples for $mode, got $($sorted.Count)" }
    $mid = [int][math]::Floor($sorted.Count / 2)
    $median = if ($sorted.Count % 2) { $sorted[$mid] } else { ($sorted[$mid - 1] + $sorted[$mid]) / 2 }
    "checkForUpdates={0}: min {1:N1} ms, median {2:N1} ms, max {3:N1} ms ({4} runs)" -f $mode, $sorted[0], $median, $sorted[-1], $sorted.Count
}
