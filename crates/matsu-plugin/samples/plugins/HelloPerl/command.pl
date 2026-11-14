# Command 1 of the sample plugin: inserts the greeting at the caret.
# Editor, EditorError, Plugin, Outline, Complement and Indent are there without a `use`. The handler
# is run by `eval`: a `my` variable stays inside the call, a package variable (`our $count`) stays for
# the next call (the plugin is resident).
my $greeting = Plugin->GetOption('Hello', 'Greeting');
$greeting = 'Hello' if !defined $greeting || $greeting eq '';
Editor->InsText($greeting . ' (' . Plugin->jack . ')');
