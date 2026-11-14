# The jack Complement: two candidates that begin with the word before the caret.
word = Complement.GetCurrentWord()
Complement.AddList(word + "_hello")
Complement.AddList(word + "_world")
