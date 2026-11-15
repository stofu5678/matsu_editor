# スマートインデント

`SmartIndent` ジャックは、文字を入力した直後に呼ばれて、字下げの調整をプラグインに任せます。

## 使い方

1. プラグインの `[Plug]` に、`SmartIndent=indent.js` と書き、`SmartIndent.Label` で名前を付けます。
2. キー入力のあとに呼ぶので、プロセスを残すプラグインだけが使えます。`[Wsh]` に `UseCache=1` と書くか、`[Matsu]` に `Resident=1` と書きます（[plugin.def の書き方](plugin-def.md)）。
3. 改行以外の文字でも呼びたいときは、`[Matsu]` の `SmartIndent.Chars` に、その文字を並べて書きます。
4. プラグインを有効にします。
5. 文書のタイプのタイプ別設定を開きます。「スクリーン」ページの「インデント」にある「スマートインデント」で、プラグインの名前を選びます。

```
[Wsh]
UseCache=1

[Matsu]
; Besides the line break, call the plugin after "}".
SmartIndent.Chars=}
```

プロセスを残さないプラグインの選択肢には、「（常駐が必要: UseCache=1）」と付きます。選んでも、呼ばれません。

## 呼ばれる場面

- 改行を入力した直後。字下げが先にコピーされたあとの状態で呼ばれます。
- `SmartIndent.Chars` に書いた文字を入力した直後。
- それ以外の文字では、呼ばれません。

次のときは、呼ばれません。

- マクロやキーマクロの再生中。再生の間は、1 つ 1 つの改行でプラグインを待たず、プラグインが `Editor` でした編集が、マクロの結果と混ざらないようにするためです。
- ほかのプラグインの呼び出しが、そのウィンドウで動いているとき。
- そのプラグインが、前の文字の呼び出しをまだ終えていないとき。この呼び出しは、捨てます。

## 動き方

- 入力したコマンドは、プラグインを待ちません。文字は、すでに文書に入っています。プラグインは、そのコマンドが終わってから、別のプロセスで動きます。
- プラグインが動いている間に入力した文字は、いつもどおりに入ります。入力は失われません。
- ハンドラの中で、入力した文字は `Indent.GetChar()` で読みます。Enter キーのときは `"\r"` です。
- プラグインが文書にしたことは、元に戻す操作の 1 つの単位です。入力した文字とは別の単位になります。
- プラグインの操作は、キーマクロに記録しません。
- プラグインが動いている間に、利用者が文字を打ったり、キャレットや選択を動かしたりしたときは、そこでプラグインを止めます。何も言いません。止めるまでにしたことは残り、元に戻す単位として、利用者の入力の前で終わります。最後にまとめて挿入するプラグインは、何も挿入されないことになります。
- 制限の時間は 0.5 秒です。

サンプル（`crates/matsu-plugin/samples/plugins/SmartIndentJs/indent.js` の要点）です。`{` で終わる行のあとの改行で、4 つの空白を足します。

```js
// Called after the Enter key and after "}" (SmartIndent.Chars).
const typed = Indent.GetChar();
if (typed === '\r' || typed === '\n') {
  const line = Number(Editor.ExpandParameter('$y'));
  if (line >= 2 && /\{\s*$/.test(Editor.GetLineStr(line - 1))) {
    Editor.InsText('    ');
  }
}
```

## サクラエディタとの違い

- サクラエディタは、文字を入力するたびに、プラグインの終わりを待ってから次に進みます。プラグインは、改行かどうかを自分で判定します。MATSU Editor は、改行と `SmartIndent.Chars` の文字のあとだけ、別のプロセスで、入力のコマンドが終わってから呼びます。
- マクロの再生中は、呼びません。サクラエディタは、呼びます。
- 0.5 秒の制限があります。
- プロセスを残すプラグインだけが使えます。

## 根拠

- MATSU のコード: `crates/matsu-ui/src/macros/plugin_call/indent.rs`（冒頭の説明、`wants`、`prepare_smart_indent`、`call_smart_indent_plugin`）、`crates/matsu-commands/src/commands/edit/input_char.rs`（`SmartIndentCall`）、`crates/matsu-commands/src/input_options.rs`
- 選択肢と「（常駐が必要: UseCache=1）」: `crates/matsu-ui/src/type_settings/plugin_choices.rs`、`pages/screen.rs`（「スマートインデント」）
- `Indent.GetChar`: `crates/matsu-script/src/quickjs.js` の `begin`
- 制限の時間: `crates/matsu-plugin/src/plugin_host.rs` の `LIMITS`
- マクロの再生中は呼ばないこと: `docs/adr/0026-resident-plugins.md` の「追記（2026-11-03）」
- 試験: `crates/matsu-ui/src/macros/plugin_call/indent/tests.rs`
- サンプル: `crates/matsu-plugin/samples/plugins/SmartIndentJs/`
- サクラのソース: `sakura_core/cmd/CViewCommander_Edit.cpp`（`Command_WCHAR` の中のスマートインデント）、`sakura_core/plugin/CSmartIndentIfObj.h`
