-- Command 1 of the sample plugin: inserts the greeting at the caret.
-- editorCall("...", {...}) calls the editor; the objects of the plugin are called as
-- editorCall("Plugin.GetOption", {...}) and so on. pluginJack() tells which jack is called. Each
-- call runs the same compiled program while its text stays the same: a property keeps its value
-- while the process stays (UseCache=1). A value that has to outlive the process goes in an option
-- of the plugin, with editorCall("Plugin.SetOption", ...).
set greeting to editorCall("Plugin.GetOption", {"Hello", "Greeting"})
if greeting is missing value or greeting is "" then set greeting to "Hello"
editorCall("InsText", {greeting & " (" & pluginJack() & ")"})
