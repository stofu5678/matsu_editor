// Command 1 of the sample plugin: inserts the greeting at the caret.
// `Editor` and `Plugin` are there without a require (ADR-0026). The host that runs this comes in a later PR.
Editor.InsText(Plugin.GetOption('Hello', 'Greeting') || 'Hello');
