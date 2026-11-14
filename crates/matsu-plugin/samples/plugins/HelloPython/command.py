# Command 1 of the sample plugin: inserts the greeting at the caret.
# Editor, EditorError and Plugin are there without an import. The namespace of this file is kept
# for the next call (the plugin is resident).
greeting = Plugin.GetOption("Hello", "Greeting") or "Hello"
Editor.InsText(f"{greeting} ({Plugin.jack})")
