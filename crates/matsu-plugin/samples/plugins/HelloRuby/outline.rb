# The jack Outline: a tree of the lines that start with "#".
Outline.SetListType(100)
Outline.SetTitle('Headings (Ruby sample)')
(1..Editor.GetLineCount(0)).each do |n|
  m = /\A(#+)\s*(.*)\z/.match(Editor.GetLineStr(n).rstrip)
  Outline.AddFuncInfo2(n, 1, m[2], m[1].length - 1) if m
end
