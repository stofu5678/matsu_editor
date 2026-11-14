<?php
// The jack SmartIndent: after Enter, four more spaces when the line above ends with a colon.
if (in_array(Indent::GetChar(), ["\r", "\n"], true)) {
    $line = (int) Editor::ExpandParameter('$y');
    if ($line >= 2 && substr(rtrim(Editor::GetLineStr($line - 1), "\r\n"), -1) === ':') {
        Editor::InsText('    ');
    }
}
