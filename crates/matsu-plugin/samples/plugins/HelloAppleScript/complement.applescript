-- The jack Complement: two candidates that begin with the word before the caret.
set theWord to complementGetCurrentWord()
editorCall("Complement.AddList", {theWord & "_hello"})
editorCall("Complement.AddList", {theWord & "_world"})
