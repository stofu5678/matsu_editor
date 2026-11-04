// ComplementGlobal: runs for the completion of every document type. Adds one candidate that begins
// with the word before the caret.
var word = Complement.GetCurrentWord();
Complement.AddList(word + '_global');
