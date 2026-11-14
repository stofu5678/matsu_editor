// Command 1 of the sample plugin (CommonJS): inserts the greeting at the caret.
// `Editor` and `Plugin` are globals; there is no import for them. The calls wait for the answer
// of the editor, so there is no await either.
const greeting = Plugin.GetOption('Hello', 'Greeting') || 'Hello';
const opens = globalThis.helloNodeOpens ?? 0;
Editor.InsText(`${greeting} (${Plugin.jack}, opened ${opens} times)`);
