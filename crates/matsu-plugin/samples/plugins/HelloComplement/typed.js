// Complement: runs for the types that chose this plugin. Adds two candidates that begin with the word
// before the caret. Complement.GetOption() tells whether the case of letters is ignored (0x01) and
// whether the words of the document are a source (0x02).
var word = Complement.GetCurrentWord();
Complement.AddList(word + '_hello');
Complement.AddList(word + '_world');
