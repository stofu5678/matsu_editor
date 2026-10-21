# Clears the "Choose privacy settings" screen that the windows-11-arm runners show over the whole
# desktop, so that the E2E pictures show the program (.github/workflows/e2e.yml, job `screens`).
#
#   windows-clear-privacy-screen.ps1 <folder for the diagnostic pictures>
#
# It sets the policy that turns the screen off, closes the programs that draw it, and saves a
# picture of the screen before and after, and the list of the programs with a window, so that a
# person or an AI can see from the job's artifact whether it worked.
param([Parameter(Mandatory = $true)][string]$Folder)
$ErrorActionPreference = 'Continue'
New-Item -ItemType Directory -Force -Path $Folder | Out-Null

function Save-Screen([string]$Name) {
    Add-Type -AssemblyName System.Drawing
    Add-Type -AssemblyName System.Windows.Forms
    $area = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bitmap = New-Object System.Drawing.Bitmap $area.Width, $area.Height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($area.Left, $area.Top, 0, 0, $bitmap.Size)
    $bitmap.Save((Join-Path $Folder "$Name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
}

function Show-Windows([string]$Title) {
    Write-Host "== programs with a window ($Title) =="
    Get-Process | Where-Object { $_.MainWindowTitle -ne '' -or $_.ProcessName -match 'oobe|CloudExperience|WWAHost|FirstLogon' } |
        Sort-Object ProcessName | Format-Table Id, ProcessName, MainWindowTitle -AutoSize | Out-String -Width 200 | Write-Host
}

Show-Windows 'before'
Save-Screen 'windows-arm64-desktop-before'

# The policy "Don't launch privacy settings experience on user logon".
$oobe = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\OOBE'
New-Item -Path $oobe -Force | Out-Null
New-ItemProperty -Path $oobe -Name DisablePrivacyExperience -Value 1 -PropertyType DWord -Force | Out-Null

# The policy only stops a screen that has not started; the one on the desktop is closed here.
foreach ($name in 'CloudExperienceHostBroker', 'CloudExperienceHost', 'WWAHost', 'FirstLogonAnim', 'oobe') {
    Get-Process -Name $name -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host "stopping $($_.ProcessName) ($($_.Id))"
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}
Start-Sleep -Seconds 5

Show-Windows 'after'
Save-Screen 'windows-arm64-desktop-after'
