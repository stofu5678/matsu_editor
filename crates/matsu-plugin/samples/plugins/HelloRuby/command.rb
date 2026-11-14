# Command 1 of the sample plugin: inserts the greeting at the caret.
# Editor, EditorError and Plugin are there without a require. `self` of this file is kept for the
# next call (the plugin is resident), so its instance variables stay.
greeting = Plugin.GetOption('Hello', 'Greeting')
greeting = 'Hello' if greeting.nil? || greeting.empty?
Editor.InsText("#{greeting} (#{Plugin.jack})")
