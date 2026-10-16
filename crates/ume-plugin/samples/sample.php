<?php
// Sample macro (PHP). Run it from the Tools menu of the PoC:
//   ツール > 名前を指定してマクロ実行...
// `Editor` is there without a require: the static methods of the class Editor. The names and
// the arguments are Sakura's macro API.

Editor::GoFileTop();
Editor::InsText("# サンプルマクロ 😀\n");

$count = Editor::GetLineCount(0);
$name = Editor::GetFilename() ?: '(無題)';
echo '行数: ' . $count . ' ファイル: ' . $name . "\n";
for ($i = 1; $i <= $count; $i++) {
    echo $i . ': ' . rtrim(Editor::GetLineStr($i), "\n") . "\n";
}

// Select line 3 (a line number and a column both start at 1) and replace it.
Editor::MoveCursor(3, 1, 0);
Editor::MoveCursor(3, 1000, 1); // past the end of the line: the end of the line
// (echo with commas prints each part as soon as it is evaluated, so a call in the middle splits the line.)
echo '選択: ' . Editor::GetSelectedString(0) . ' / IsTextSelected = ' . var_export(Editor::IsTextSelected(), true) . "\n";
Editor::InsText('みかん😀 (置き換えました)');

// An error response is an exception.
try {
    Editor::GetLineStr('not a number');
} catch (EditorError $e) {
    echo 'エラー: ', $e->getCode(), ' ', $e->getMessage(), "\n";
}

Editor::AddTail("-- マクロの終わり --\n");
Editor::Jump(1);
