# Tests the Windows installer (packaging/windows/matsu.iss, ADR-0032). Run by package-check.yml on the runner of
# the installer's target, with the setup.exe that package.yml built.
#
#   test-windows-installer.ps1 -Setup dist\matsu-windows-x64-setup.exe
#
# What it proves:
#   1. A silent per-user install puts the files, the Start menu entry and the uninstall entry in place, writes the
#      version information into matsu.exe, and creates no matsu.ini in the install folder (that would turn on the
#      portable mode) and nothing in the settings folder.
#   2. The user's data (%APPDATA%\matsu-editor\matsu.ini and unsaved\x.txt) is the same after an update and after
#      the uninstall (ADR-0032, decision 3).
#   3. An update replaces the files; the uninstall removes the files and the entries, and nothing more.
#   4. The options: "Open with" (default), .txt and PATH (off by default; checked here when chosen, and removed
#      again), and a portable matsu.ini next to matsu.exe survives the uninstall.
#   5. An all-users install goes to Program Files and is removed too.

param(
    [Parameter(Mandatory)][string]$Setup,
    # The version in Cargo.toml (default: read it from there).
    [string]$Version = ''
)

$ErrorActionPreference = 'Stop'
$Setup = (Resolve-Path $Setup).Path
if (-not $Version) {
    $Version = (Select-String -Path (Join-Path $PSScriptRoot '../../Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' |
        Select-Object -First 1).Matches[0].Groups[1].Value
}

$appId = '{7A716E2B-C55F-4618-92F3-3058FA1AAEEB}'
$uninstallKey = "Software\Microsoft\Windows\CurrentVersion\Uninstall\${appId}_is1"
$userDir = Join-Path $env:LOCALAPPDATA 'Programs\MATSU Editor'
$machineDir = Join-Path $env:ProgramFiles 'MATSU Editor'
$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\MATSU Editor.lnk'
$settingsDir = Join-Path $env:APPDATA 'matsu-editor'
$logDir = Join-Path $env:RUNNER_TEMP 'installer-logs'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

function Check([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "FAILED: $Message" }
    Write-Host "ok: $Message"
}

function Invoke-Installer([string]$Name, [string[]]$Arguments) {
    $log = Join-Path $logDir "$Name.log"
    $process = Start-Process -FilePath $Setup -Wait -PassThru -ArgumentList (
        @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$log`"") + $Arguments)
    if ($process.ExitCode -ne 0) {
        if (Test-Path $log) { Get-Content $log | Write-Host }
        throw "FAILED: the installer ($Name) exited with $($process.ExitCode)"
    }
    Write-Host "ran the installer: $Name ($($Arguments -join ' '))"
}

function Invoke-Uninstaller([string]$Dir) {
    $uninstaller = Join-Path $Dir 'unins000.exe'
    Check (Test-Path $uninstaller) "$uninstaller exists"
    $process = Start-Process -FilePath $uninstaller -Wait -PassThru -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART'
    if ($process.ExitCode -ne 0) { throw "FAILED: the uninstaller exited with $($process.ExitCode)" }
    # The uninstaller hands over to a copy of itself and deletes its own file last; wait for that (up to a minute).
    for ($i = 0; $i -lt 60 -and (Test-Path $uninstaller); $i++) { Start-Sleep -Seconds 1 }
    Check (-not (Test-Path $uninstaller)) 'the uninstaller finished and removed itself'
}

# A text that lists every file under a folder with its hash, to compare before and after.
function Get-Snapshot([string]$Dir) {
    if (-not (Test-Path $Dir)) { return '(no folder)' }
    (Get-ChildItem -Path $Dir -Recurse -Force -File | Sort-Object FullName | ForEach-Object {
        "$($_.FullName.Substring($Dir.Length)) $((Get-FileHash $_.FullName -Algorithm SHA256).Hash) $($_.LastWriteTimeUtc.Ticks)"
    }) -join "`n"
}

function Get-RawPath([string]$Scope) {
    $root = if ($Scope -eq 'User') { [Microsoft.Win32.Registry]::CurrentUser } else { [Microsoft.Win32.Registry]::LocalMachine }
    $subKey = if ($Scope -eq 'User') { 'Environment' } else { 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment' }
    $root.OpenSubKey($subKey).GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
}

function Test-PathHas([string]$Scope, [string]$Dir) {
    (Get-RawPath $Scope).Split(';') -contains $Dir
}

function Check-Gone([string]$Dir, [string]$Scope) {
    $hive = if ($Scope -eq 'User') { 'HKCU:' } else { 'HKLM:' }
    Check (-not (Test-Path (Join-Path $Dir 'matsu.exe'))) "matsu.exe is gone from $Dir"
    Check (-not (Test-Path (Join-Path $Dir 'matsu-script.exe'))) "matsu-script.exe is gone from $Dir"
    Check (-not (Test-Path "$hive\$uninstallKey")) 'the uninstall entry is gone'
    Check (-not (Test-Path "$hive\Software\Classes\Applications\matsu.exe")) 'the "Open with" entry is gone'
    Check (-not (Test-Path "$hive\Software\Classes\MATSU.Editor.txt")) 'the .txt program entry is gone'
    Check (-not (Test-Path "$hive\Software\MATSU Editor")) 'the Default apps entry is gone'
    Check (-not (Get-ItemProperty "$hive\Software\RegisteredApplications" -ErrorAction SilentlyContinue).'MATSU Editor') 'the RegisteredApplications value is gone'
    Check (-not (Get-ItemProperty "$hive\Software\Classes\.txt\OpenWithProgids" -ErrorAction SilentlyContinue).'MATSU.Editor.txt') 'the .txt OpenWithProgids value is gone'
}

# ---- Before: nothing of MATSU Editor is installed ----
Check (-not (Test-Path $userDir)) "$userDir does not exist yet"
Check (-not (Test-Path "HKCU:\$uninstallKey")) 'no uninstall entry yet'
$settingsExisted = Test-Path $settingsDir
$userPathBefore = Get-RawPath 'User'

# ---- 1. Silent per-user install ----
Invoke-Installer 'install' @('/CURRENTUSER')
foreach ($file in 'matsu.exe', 'matsu-script.exe', 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY_NOTICES.txt', 'README.txt', 'icons\matsu_editor.ico', 'unins000.exe') {
    Check (Test-Path (Join-Path $userDir $file)) "$file is installed in $userDir"
}
Check (-not (Test-Path (Join-Path $userDir 'matsu.ini'))) 'the install folder has no matsu.ini'
Check (-not (Test-Path (Join-Path $userDir 'plugins'))) 'the install folder has no plugins folder'
Check ($settingsExisted -or -not (Test-Path $settingsDir)) 'the installer did not create the settings folder'
Check (Test-Path $startMenu) 'the Start menu entry exists'
$entry = Get-ItemProperty "HKCU:\$uninstallKey"
Check ($entry.DisplayName -eq 'MATSU Editor') "the uninstall entry is named MATSU Editor ($($entry.DisplayName))"
Check ($entry.DisplayVersion -eq $Version) "the uninstall entry has version $Version ($($entry.DisplayVersion))"
Check ($entry.Publisher -eq 'stofu5678') "the uninstall entry has the publisher ($($entry.Publisher))"
Check ($entry.InstallLocation -like "$userDir*") "the uninstall entry points at $userDir"
$info = (Get-Item (Join-Path $userDir 'matsu.exe')).VersionInfo
Check ($info.ProductName -eq 'MATSU Editor') "matsu.exe has ProductName MATSU Editor ($($info.ProductName))"
Check ($info.CompanyName -eq 'stofu5678') "matsu.exe has CompanyName ($($info.CompanyName))"
Check ($info.FileDescription -eq 'MATSU Editor') "matsu.exe has FileDescription ($($info.FileDescription))"
Check ($info.ProductVersion -eq $Version) "matsu.exe has ProductVersion $Version ($($info.ProductVersion))"
Check ($info.FileVersion -eq $Version) "matsu.exe has FileVersion $Version ($($info.FileVersion))"
Check ($info.LegalCopyright -like '*MIT OR Apache-2.0*') "matsu.exe has the license in LegalCopyright ($($info.LegalCopyright))"
$commandKey = "HKCU:\Software\Classes\Applications\matsu.exe\shell\open\command"
Check (Test-Path $commandKey) '"Open with" is registered by default'
Check ((Get-ItemProperty $commandKey).'(default)' -eq "`"$userDir\matsu.exe`" `"%1`"") 'the "Open with" command opens the file with matsu.exe'
Check (-not (Test-Path 'HKCU:\Software\Classes\MATSU.Editor.txt')) '.txt is not offered by default'
Check ((Get-RawPath 'User') -eq $userPathBefore) 'PATH is not changed by default'

# ---- 2. The user's data ----
New-Item -ItemType Directory -Force -Path (Join-Path $settingsDir 'unsaved') | Out-Null
[IO.File]::WriteAllText((Join-Path $settingsDir 'matsu.ini'), "[Test]`r`nvalue=known text 日本語`r`n", (New-Object Text.UTF8Encoding $false))
[IO.File]::WriteAllText((Join-Path $settingsDir 'unsaved\x.txt'), "unsaved text`r`nsecond line 日本語", (New-Object Text.UTF8Encoding $false))
$dataBefore = Get-Snapshot $settingsDir
Write-Host $dataBefore

# ---- 3. Update (the same installer again, over the files, one of which was changed) ----
$original = (Get-FileHash (Join-Path $userDir 'LICENSE-MIT') -Algorithm SHA256).Hash
Add-Content -Path (Join-Path $userDir 'LICENSE-MIT') -Value 'changed after the install'
Invoke-Installer 'update' @('/CURRENTUSER')
Check ((Get-FileHash (Join-Path $userDir 'LICENSE-MIT') -Algorithm SHA256).Hash -eq $original) 'the update replaced the changed file'
Check (@(Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall' | Where-Object { $_.PSChildName -eq "${appId}_is1" }).Count -eq 1) 'one uninstall entry after the update'
Check ((Get-Snapshot $settingsDir) -eq $dataBefore) 'the user data is unchanged by the update'
Check (-not (Test-Path (Join-Path $userDir 'matsu.ini'))) 'the install folder has no matsu.ini after the update'

# ---- 4. Silent uninstall ----
Invoke-Uninstaller $userDir
Check-Gone $userDir 'User'
Check (-not (Test-Path $startMenu)) 'the Start menu entry is gone'
Check (-not (Test-Path $userDir)) "the folder $userDir is gone"
Check ((Get-Snapshot $settingsDir) -eq $dataBefore) 'the user data is unchanged by the uninstall (same files, same hashes, same times)'
Check ((Get-RawPath 'User') -eq $userPathBefore) 'PATH is as before'

# ---- 5. The options: .txt, PATH, and a portable matsu.ini ----
Invoke-Installer 'install-options' @('/CURRENTUSER', '/TASKS="openwith,associatetxt,addtopath"')
$progId = Get-ItemProperty 'HKCU:\Software\Classes\.txt\OpenWithProgids'
Check ($null -ne $progId.'MATSU.Editor.txt') '.txt: the program is in OpenWithProgids'
Check (Test-Path 'HKCU:\Software\Classes\MATSU.Editor.txt\shell\open\command') '.txt: the program has an open command'
Check ((Get-ItemProperty 'HKCU:\Software\RegisteredApplications').'MATSU Editor' -eq 'Software\MATSU Editor\Capabilities') '.txt: MATSU Editor is a registered application'
Check ((Get-ItemProperty 'HKCU:\Software\MATSU Editor\Capabilities\FileAssociations').'.txt' -eq 'MATSU.Editor.txt') '.txt: the capabilities list .txt'
Check (Test-PathHas 'User' $userDir) 'PATH has the install folder'
# Updating with the same options does not add it twice.
Invoke-Installer 'update-options' @('/CURRENTUSER', '/TASKS="openwith,associatetxt,addtopath"')
Check (@((Get-RawPath 'User').Split(';') | Where-Object { $_ -eq $userDir }).Count -eq 1) 'PATH has the install folder once after an update'
# A portable matsu.ini the user put next to matsu.exe is not the installer's: the uninstall leaves it.
$portable = Join-Path $userDir 'matsu.ini'
[IO.File]::WriteAllText($portable, "[Portable]`r`nvalue=1`r`n", (New-Object Text.UTF8Encoding $false))
$portableHash = (Get-FileHash $portable -Algorithm SHA256).Hash
Invoke-Uninstaller $userDir
Check-Gone $userDir 'User'
Check ((Test-Path $portable) -and (Get-FileHash $portable -Algorithm SHA256).Hash -eq $portableHash) 'a portable matsu.ini next to matsu.exe is left unchanged by the uninstall'
Check (@(Get-ChildItem $userDir -Force).Count -eq 1) 'nothing else is left in the install folder'
Check ((Get-RawPath 'User') -eq $userPathBefore) 'PATH is as before the install after the uninstall'
Check ((Get-Snapshot $settingsDir) -eq $dataBefore) 'the user data is unchanged after the second uninstall'
Remove-Item -Recurse -Force $userDir

# ---- 6. All users (the runner is an administrator) ----
$machinePathBefore = Get-RawPath 'Machine'
Invoke-Installer 'install-allusers' @('/ALLUSERS')
Check (Test-Path (Join-Path $machineDir 'matsu.exe')) "matsu.exe is installed in $machineDir"
Check (Test-Path (Join-Path $machineDir 'matsu-script.exe')) "matsu-script.exe is installed in $machineDir"
Check (Test-Path "HKLM:\$uninstallKey") 'the uninstall entry is in HKLM'
Check (-not (Test-Path (Join-Path $machineDir 'matsu.ini'))) 'the Program Files folder has no matsu.ini'
Check (Test-Path 'HKLM:\Software\Classes\Applications\matsu.exe\shell\open\command') '"Open with" is registered for all users'
Invoke-Uninstaller $machineDir
Check-Gone $machineDir 'Machine'
Check (-not (Test-Path $machineDir)) "the folder $machineDir is gone"
Check ((Get-RawPath 'Machine') -eq $machinePathBefore) 'the machine PATH is as before'
Check ((Get-Snapshot $settingsDir) -eq $dataBefore) 'the user data is unchanged after the all-users uninstall'

Write-Host 'All installer checks passed.'
