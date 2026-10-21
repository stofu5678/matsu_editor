# Saves the whole virtual screen as a PNG: capture.ps1 <path>
# Used by the E2E screenshot tests (crates/ume/tests/common/mod.rs) on Windows. The window of the
# program is on the interactive desktop of the runner, so a copy of the screen shows it with every
# dialog and menu that belongs to it.
param([Parameter(Mandatory = $true)][string]$Path)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @"
using System.Runtime.InteropServices;
public static class UmeDpi {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
# Without this the size is the scaled one and the copy is cut off on a screen with a scale.
[void][UmeDpi]::SetProcessDPIAware()
$area = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bitmap = New-Object System.Drawing.Bitmap $area.Width, $area.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($area.Left, $area.Top, 0, 0, $bitmap.Size)
$bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
