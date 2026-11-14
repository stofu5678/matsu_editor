// Command 1 (ES module): inserts the greeting at the caret. The module is loaded once; the exported
// function runs for each call. `Editor` and `Plugin` are globals; the calls wait for the editor's answer,
// so there is no await.
import { greetingText } from './lib/greeting.mjs';
import { opens } from './opened.mjs';

export default function () {
  const greeting = Plugin.GetOption('Hello', 'Greeting') || 'Hello';
  Editor.InsText(greetingText(greeting, Plugin.jack, opens()));
}
