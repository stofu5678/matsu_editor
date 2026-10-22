# Saves the whole virtual screen as a PNG: capture.ps1 <path>
# Used by the E2E screenshot tests (crates/matsu/tests/common/mod.rs) on Windows. The window of the
# program is on the interactive desktop of the runner, so a copy of the screen shows it with every
# dialog and menu that belongs to it.
param([Parameter(Mandatory = $true)][string]$Path)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
# DPI awareness comes from the environment of the caller (`__COMPAT_LAYER=HighDpiAware`, see
# common/mod.rs): compiling a helper with Add-Type takes seconds on every picture.
$area = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bitmap = New-Object System.Drawing.Bitmap $area.Width, $area.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($area.Left, $area.Top, 0, 0, $bitmap.Size)
$bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
