# Builds dwrite-font-update-check and runs it in fresh processes, alternating
# checkForUpdates = true and false. Prints the environment and the results.
# Usage (in this directory): powershell -ExecutionPolicy Bypass -File run.ps1 [-Runs 10]
param([int]$Runs = 10)

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

cargo build --release --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$exe = ".\target\release\dwrite-font-update-check.exe"

# The first run after a build or a reboot may be slower; it is not counted.
& $exe true | Out-Null
& $exe false | Out-Null

$results = @{ "true" = @(); "false" = @() }
for ($i = 1; $i -le $Runs; $i++) {
    foreach ($mode in "true", "false") {
        $line = & $exe $mode
        $line
        if ($line -match "GetSystemFontCollection=([\d.]+) ms") { $results[$mode] += [double]$Matches[1] }
    }
}
""
foreach ($mode in "true", "false") {
    $sorted = $results[$mode] | Sort-Object
    $median = $sorted[[int][math]::Floor(($sorted.Count - 1) / 2)]
    "checkForUpdates={0}: min {1:N1} ms, median {2:N1} ms, max {3:N1} ms ({4} runs)" -f $mode, $sorted[0], $median, $sorted[-1], $sorted.Count
}
