# Lua

拡張子は `.lua` です。MATSU Editor に同梱の `matsu-script` が動かします。利用者が入れるものは、ありません。呼び方の例は、「マクロの使い方」の章にあります。

## 処理系

| 項目 | 内容 |
|---|---|
| 版 | Lua 5.4 |
| 作り | Rust のクレート `mlua` が、C のソースを取り込んで、一緒にビルドしたもの |
| ライセンス | MIT |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |

## 標準ライブラリ

Lua の標準ライブラリが使えます。ファイルを開く `io` も含みます。同梱の処理系は、外へ出られないように閉じ込めてはいません。スクリプトは、利用者の権限で、ファイルを読み書きできます。

`print` は、すべての引数を `tostring` にかけて、タブで区切って、1 行にして出します。出力はすぐに流れるので、`Editor` の呼び出しとの順番が入れ替わることは、ありません。`io.write` と `io.stderr` も、同じように、ためずに出します。

## 文字コード

- スクリプトは UTF-8 で保存します（BOM は取り除きます）。
- `Editor` の関数に渡す文字列は、正しい UTF-8 でなければなりません。そうでない文字列を渡すと、文字を置き換えて通すことはせず、エラー（`EditorError`、番号 `-32602`）になります。
- 整数の値の浮動小数点数（`4 / 2` の `2.0` など）は、整数として渡します。
- 表（テーブル）や関数は、引数に渡せません。渡すと、`-32602` のエラーになります。
- 引数の最後の `nil` は省略として扱います。
- 戻り値の `null` は `nil` になります。配列の途中の `null` は、穴になります。

## エラー

`Editor` の関数が失敗すると、`EditorError` の値を投げます。`pcall` で受けられます。

| フィールド | 内容 |
|---|---|
| `code` | エラーの番号 |
| `message` | 理由 |
| `where` | 呼んだ場所（`名前:行:` の形） |

受けなかったときは、スクリプトが終わり、標準エラーに `名前:3: EditorError -32602: ...` の形で出ます。Lua 自身のエラーも、`名前:行:` から始まります。名前は、スクリプトのパスです。

先頭の行が `#` で始まるとき（シバン）は、行番号を保ったまま、その行をコメントにします。

## プラグインとして動かすとき

- ハンドラのファイルごとに、別の Lua の状態（`lua_State`）が作られます。プロセスが残っている（`UseCache=1` か `Resident=1`）間、同じファイルの次の呼び出しは、同じ状態を使います。呼び出しの間に残したいものは、グローバル変数に置きます。`local` は、呼び出しごとに消えます。別のファイルのグローバル変数は、見えません。
- ハンドラの文面は、ファイルの中身そのままを、1 つのチャンクとして動かします。`return` は、そのハンドラを終えるだけです。
- `Plugin`、`Outline`、`Complement`（と、`SmartIndent` のとき `Indent`）が、グローバルにあります。関数は、`Editor` と同じに、ドットで呼びます（`Plugin.GetOption('A', 'B')`、`Complement.AddList(word)`）。引数の最後の `nil` は、省略として扱います。
- `Plugin.id`（プラグインの ID）、`Plugin.dir`、`Plugin.jack`、`Plugin.args`（呼び出しの引数の表。無いときは `nil`）があります（[ハンドラの中で使えるもの](plugin-api.md)）。
- `Complement.AddList` は、数や真偽値を、文字列にして渡します。
- エラーが出ると、その呼び出しは失敗になります。メッセージは、ハンドラのファイルの行番号つきです。
- 標準エラーへの出力は、その呼び出しが終わる前に、エディターに届きます。

## 根拠

- 版と作り: `crates/matsu-script/Cargo.toml`（`mlua` の `lua54` と `vendored`）、`crates/matsu-script/src/lua.rs` の冒頭
- 標準ライブラリ、`print`、`io`: `crates/matsu-script/src/lua.rs` の `install`、`docs/adr/0026-resident-plugins.md`（「同梱の処理系は、サンドボックスに入れていない」）
- 引数の変換: 同ファイルの `lua_to_json`、`float_to_json`、`json_to_lua`、`bind`
- `EditorError`: 同ファイルの `BOOT`、`caller_location`
- ライセンス: `THIRD_PARTY_NOTICES.txt`
- プラグイン: `crates/matsu-script/src/lua.rs` の `PLUGIN_BOOT`、`PluginState`、`crates/matsu-script/src/plugin.rs`
- 試験: `crates/matsu-script/tests/lua.rs`、`crates/matsu-script/tests/plugin_engines.rs`
- 設計: `docs/poc/json-rpc.md`（「同梱のスクリプトホスト」）
