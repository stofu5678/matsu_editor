# The jack SmartIndent: after Enter, four more spaces when the line above ends with a colon.
if Indent.GetChar() in ("\r", "\n"):
    line = int(Editor.ExpandParameter("$y"))
    if line >= 2 and Editor.GetLineStr(line - 1).rstrip("\r\n").endswith(":"):
        Editor.InsText("    ")
