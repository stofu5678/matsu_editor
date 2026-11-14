<?php
// The jack Outline: a tree of the lines that start with "#".
Outline::SetListType(100);
Outline::SetTitle('Headings (PHP sample)');
$total = Editor::GetLineCount(0);
for ($n = 1; $n <= $total; $n++) {
    if (preg_match('/^(#+)\s*(.*)$/', rtrim(Editor::GetLineStr($n), "\r\n"), $m) === 1) {
        Outline::AddFuncInfo2($n, 1, $m[2], strlen($m[1]) - 1);
    }
}
