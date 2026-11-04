// The plug of the jack Outline of the sample plugin: a tree of the lines that start with "#".
// `Outline` is the object of Sakura's plugins (AddFuncInfo to AddFuncInfo4, SetTitle, SetListType,
// SetLabel); `Editor` reads the document. The plug is called once when the outline is analyzed.
//
// A line "# a" is at depth 0, "## b" at depth 1, and so on. A line "#fail" makes the plug throw, and a
// line "#slow" makes it loop for ever: the pane then shows an empty list and says why (the limit of the
// jack Outline is 10 seconds).
Outline.SetListType(100); // 100: a tree. AddFuncInfo2 takes the depth.
Outline.SetTitle('Headings (sample plugin)');
const count = Editor.GetLineCount(0);
for (let n = 1; n <= count; n++) {
  const line = Editor.GetLineStr(n).replace(/[\r\n]+$/, '');
  const m = /^(#+)\s*(.*)$/.exec(line);
  if (!m) continue;
  if (m[2] === 'fail') throw new Error('the sample plug failed on purpose');
  if (m[2] === 'slow') for (;;) {}
  Outline.AddFuncInfo2(n, 1, m[2], m[1].length - 1);
}
