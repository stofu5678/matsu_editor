# Command 1 of the sample plugin: inserts the greeting at the caret.
# $Editor, $Plugin, $Outline, $Complement and $Indent are there without an import. The handler runs in
# a scope of its own: a variable stays for the next call (the plugin is resident) only as $global:name.
$greeting = $Plugin.GetOption('Hello', 'Greeting')
if (-not $greeting) { $greeting = 'Hello' }
$Editor.InsText("$greeting ($($Plugin.Jack))")
