# The jack Outline: a tree of the lines that start with "#".
$Outline.SetListType(100)
$Outline.SetTitle('Headings (PowerShell sample)')
$total = $Editor.GetLineCount(0)
for ($n = 1; $n -le $total; $n++) {
    if ($Editor.GetLineStr($n).TrimEnd("`r", "`n") -match '^(#+)\s*(.*)$') {
        $Outline.AddFuncInfo2($n, 1, $Matches[2], $Matches[1].Length - 1)
    }
}
