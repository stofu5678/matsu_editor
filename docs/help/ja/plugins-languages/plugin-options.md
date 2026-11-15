# プラグインのオプション

プラグインは、利用者が変えられる値（オプション）を持てます。値は、「プラグイン設定」のダイアログで編集します。プラグインのハンドラは、`Plugin.GetOption` で読みます。

## オプションを定義する

`plugin.def` の `[Option]` に書きます（[plugin.def の書き方](plugin-def.md)）。

```
[Option]
O[1].Section=Hello
O[1].Key=Greeting
O[1].Label=Greeting
O[1].Type=Str
O[1].Default=Hello
```

## ダイアログを開く

1. 共通設定の「プラグイン」で、プラグインの行を選びます。
2. 「設定」を押します。

「設定」は、プラグインを有効にしていて、オプションが 1 つ以上あるときだけ使えます。

オプションのないプラグインのダイアログは、「指定できるオプションがありません」と出て、OK は効きません。

## ダイアログの使い方

題は「`<プラグインの名前>` プラグインの設定」です。上に、オプションの一覧（項目と値）があり、1 つ選ぶと、下の「値」で編集します。型で、入力欄が変わります。

| 型 | 入力欄 | 一覧の値の表示 |
|---|---|---|
| `Str` | 文字を入れる欄。1024 文字まで | 入れた文字 |
| `Dir` | 文字を入れる欄と、フォルダーを選ぶ「…」のボタン。選んだフォルダーのパスは、最後に区切り文字が付きます | 入れた文字 |
| `Bool` | チェックの箱 | オンは ☑、オフは ☐ |
| `Int` | 数を入れる欄と、増やす ▲ と減らす ▼ のボタン | 整数。数字でない文字を入れると 0 |
| `Sel` | 選択肢の一覧 | 選んだ項目の表示の文字。どの項目にも合わない値は、何も表示しません |

- 「OK」は、すべてのオプションの値を、変えていないものも含めて、ファイルに書いて、閉じます。ファイルを書けなかったときは、理由が出て、ダイアログは閉じません。
- 「キャンセル」は、何も書かずに閉じます。
- 値を書くファイルは、プラグインの外にある別のファイルです（次の節）。ファイルのほかの行（`plugin.def` にないオプションの値）は、そのまま残します。
- このダイアログは、設定のウィンドウを閉じなくても使えます（操作を止めない形）。同時に開けるのは 1 つで、2 つ目を開こうとすると、先に開いたものが前に出ます。

## 値のファイル

値は、`plugins` フォルダーの、`<フォルダー名>.ini` に書きます。フォルダー名は、プラグインのフォルダーの名前です。置き場所は、[プラグインを入れる](plugin-install.md#place)の 1 番目の `plugins` です。

```
[Hello]
Greeting=Hello
```

- セクションとキーは、`O[n].Section` と `O[n].Key` のものです。
- `Bool` は `0` か `1` で書きます。`Int` は整数で書きます。
- ファイルにない値は、`O[n].Default` を使います。
- ハンドラは、`Plugin.GetOption(section, key)` で読みます。呼ぶたびにファイルを読み直すので、ダイアログで変えた値は、次の呼び出しから見えます。プラグインを動かし直す必要はありません。
- `Plugin.SetOption(section, key, value)` は、すぐにファイルへ書きます。

## サクラエディタとの違い

- ダイアログの ReadMe とヘルプのボタンは、まだありません。

## 根拠

- ダイアログ: `crates/matsu-ui/src/plugin_options/dialog.rs`、`model.rs`（`Form::save`、`normalize`、`MAX_LENGTH_VALUE`、`with_trailing_separator`）
- 値のファイル: `crates/matsu-config/src/plugins/options.rs`、`crates/matsu-config/src/plugins.rs` の `SearchDirs::option_file`
- 「設定」を使える条件: `crates/matsu-ui/src/common_settings/pages/plugin/list.rs` の `can_open_options`
- `Plugin.GetOption` と `SetOption`: `crates/matsu-ui/src/macros/plugin_call.rs` の `DefApi`
- 試験: `crates/matsu-ui/src/plugin_options/tests.rs`
- サクラのソース: `sakura_core/dlg/CDlgPluginOption.cpp`、`sakura_core/plugin/CPlugin.h` の `GetOptionPath`
- `docs/sakura-gaps.md`（「プラグインの設定ページとオプション」の行）
