// Sample macro (JavaScript, Node.js ES module: .mjs). Run it from the Tools menu of the PoC:
//   ツール > 名前を指定してマクロ実行...
// `Editor` is there without a require. The names and the arguments are Sakura's macro API.

Editor.GoFileTop();
Editor.InsText('# サンプルマクロ 😀\n');

const count = Editor.GetLineCount(0);
console.log('行数:', count, 'ファイル:', Editor.GetFilename() || '(無題)');
for (let i = 1; i <= count; i++) {
  console.log(i + ': ' + Editor.GetLineStr(i).replace(/\n$/, ''));
}

// Select line 3 (a line number and a column both start at 1) and replace it.
Editor.MoveCursor(3, 1, 0);
Editor.MoveCursor(3, 1000, 1); // past the end of the line: the end of the line
console.log('選択:', Editor.GetSelectedString(0), '/ IsTextSelected =', Editor.IsTextSelected());
Editor.InsText('みかん😀 (置き換えました)');

// An error response is an exception.
try {
  Editor.GetLineStr('not a number');
} catch (e) {
  console.log('エラー:', e.code, e.message);
}

Editor.AddTail('-- マクロの終わり --\n');
Editor.Jump(1);
