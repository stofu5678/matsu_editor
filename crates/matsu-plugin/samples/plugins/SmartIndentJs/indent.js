// The SmartIndent jack: called after the Enter key and after "}" (SmartIndent.Chars in plugin.def).
// Indent.GetChar() is the char that was typed ("\r" for the Enter key).
const typed = Indent.GetChar();
if (typed === '\r' || typed === '\n') {
  // Enter: the caret is at the start of the new line, after the indentation that was copied.
  const line = Number(Editor.ExpandParameter('$y'));
  if (line >= 2 && /\{\s*$/.test(Editor.GetLineStr(line - 1))) {
    Editor.InsText('    ');
  }
} else if (typed === '}') {
  // "}": when it is the only thing in the line, it goes back one level.
  const text = Editor.GetLineStr(0).replace(/[\r\n]+$/, '');
  if (/^ {4,}\}$/.test(text)) {
    Editor.Left();
    for (let i = 0; i < 4; i++) Editor.DeleteBack();
    Editor.Right();
  }
}
