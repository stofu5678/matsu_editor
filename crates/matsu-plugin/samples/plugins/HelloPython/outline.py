# The jack Outline: a tree of the lines that start with "#".
import re

Outline.SetListType(100)
Outline.SetTitle("Headings (Python sample)")
for n in range(1, Editor.GetLineCount(0) + 1):
    line = Editor.GetLineStr(n).rstrip("\r\n")
    m = re.match(r"^(#+)\s*(.*)$", line)
    if m:
        Outline.AddFuncInfo2(n, 1, m.group(2), len(m.group(1)) - 1)
