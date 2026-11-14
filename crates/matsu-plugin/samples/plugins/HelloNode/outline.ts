// The jack Outline (TypeScript; Node.js strips the types): a tree of the lines that start with "#".
declare const Editor: any;
declare const Outline: any;

const count: number = Editor.GetLineCount(0);
Outline.SetListType(100);
Outline.SetTitle('Headings (Node.js sample)');
for (let n: number = 1; n <= count; n++) {
  const line: string = Editor.GetLineStr(n).replace(/[\r\n]+$/, '');
  const m = /^(#+)\s*(.*)$/.exec(line);
  if (m) {
    Outline.AddFuncInfo2(n, 1, m[2], m[1].length - 1);
  }
}
