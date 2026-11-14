# The jack Complement: two candidates that begin with the word before the caret.
my $word = Complement->GetCurrentWord;
Complement->AddList($word . '_hello');
Complement->AddList($word . '_world');
