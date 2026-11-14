// The jack Outline (CommonJS): a tree of the lines that start with "#".
const count = Editor.GetLineCount(0);
Outline.SetListType(100);
Outline.SetTitle('Headings (Node.js sample)');
for (let n = 1; n <= count; n++) {
  const line = Editor.GetLineStr(n).replace(/[\r\n]+$/, '');
  const m = /^(#+)\s*(.*)$/.exec(line);
  if (m) {
    Outline.AddFuncInfo2(n, 1, m[2], m[1].length - 1);
  }
}
