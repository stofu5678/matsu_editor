# Sample macro (PowerShell: .ps1, PowerShell 7 or Windows PowerShell 5.1).
# $Editor is there without an import. The names and the arguments are Sakura's macro API.
# The macro runs in the global scope as text, so the execution policy does not apply to it.

$Editor.GoFileTop()
$Editor.InsText("# サンプルマクロ 😀`n")

$count = $Editor.GetLineCount(0)
Write-Output "行数: $count ファイル: $(if ($Editor.GetFilename()) { $Editor.GetFilename() } else { '(無題)' })"
for ($i = 1; $i -le $count; $i++) {
    Write-Output ("{0}: {1}" -f $i, $Editor.GetLineStr($i).TrimEnd("`n"))
}

# Select line 3 (a line number and a column both start at 1) and replace it.
$Editor.MoveCursor(3, 1, 0)
$Editor.MoveCursor(3, 1000, 1)  # past the end of the line: the end of the line
Write-Output "選択: $($Editor.GetSelectedString(0)) / IsTextSelected = $($Editor.IsTextSelected())"
$Editor.InsText('みかん😀 (置き換えました)')

# An error response is an exception. PowerShell wraps what a method throws, so ask
# Get-EditorErrorCode for the code ($null when $_ is not an error of the editor).
try {
    $Editor.GetLineStr('not a number')
} catch {
    Write-Output "エラー: $(Get-EditorErrorCode $_)"
}

$Editor.AddTail("-- マクロの終わり --`n")
$Editor.Jump(1)
