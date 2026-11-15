# The jack SmartIndent: after Enter, four more spaces when the line above ends with a colon.
if ["\r", "\n"].include?(Indent.GetChar)
  line = Editor.ExpandParameter('$y').to_i
  Editor.InsText('    ') if line >= 2 && Editor.GetLineStr(line - 1).rstrip.end_with?(':')
end
