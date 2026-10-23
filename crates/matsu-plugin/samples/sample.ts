// Sample macro (TypeScript: .ts, run by Node.js 22.18 or later, which removes the types).
// `Editor` is there without a require. The names and the arguments are Sakura's macro API.
// Only the types are removed: an enum or a namespace with code is an error, and nothing checks
// the types (use `tsc --noEmit` for that).

Editor.GoFileTop();
Editor.InsText('# サンプルマクロ 😀\n');

const count: number = Editor.GetLineCount(0);
console.log('行数:', count, 'ファイル:', Editor.GetFilename() || '(無題)');
for (let i: number = 1; i <= count; i++) {
  console.log(i + ': ' + (Editor.GetLineStr(i) as string).replace(/\n$/, ''));
}

// Select line 3 (a line number and a column both start at 1) and replace it.
Editor.MoveCursor(3, 1, 0);
Editor.MoveCursor(3, 1000, 1); // past the end of the line: the end of the line
console.log('選択:', Editor.GetSelectedString(0), '/ IsTextSelected =', Editor.IsTextSelected());
Editor.InsText('みかん😀 (置き換えました)');

// An error response is an exception.
try {
  Editor.GetLineStr('not a number');
} catch (e: any) {
  console.log('エラー:', e.code, e.message);
}

Editor.AddTail('-- マクロの終わり --\n');
Editor.Jump(1);
