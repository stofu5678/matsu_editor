<?php
// Command 1 of the sample plugin: inserts the greeting at the caret.
// Editor, EditorError and Plugin are there without a require. The variables of this file are global
// and stay for the next call (the plugin is resident).
$greeting = Plugin::GetOption('Hello', 'Greeting');
if ($greeting === null || $greeting === '') {
    $greeting = 'Hello';
}
Editor::InsText($greeting . ' (' . Plugin::$jack . ')');
