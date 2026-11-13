# コマンド一覧

すべての機能を、グループ別に並べた一覧です。選ぶと、その機能が動きます。

- 機能名: `F_MENU_ALLFUNC`
- メニュー: 「ヘルプ」→「コマンド一覧」
- 既定のキー: `Shift+F1`
- マクロ: `CommandList()`（引数なし。キーマクロには記録されません）

## 画面

ウィンドウの上に、2 つの列が重なって出ます。左の列がグループ、右の列が選んだグループの機能です。

- グループは、サクラエディタの「コマンド一覧」と同じ名前と並びです（「ファイル操作系」「編集系」「カーソル移動系」…「支援」「その他」の 15 グループ）。
- 機能の並びも、サクラエディタの一覧と同じです。
- 機能の右に、割り当てられている最初のキーが出ます。
- この版でまだ動かない機能は、一覧に出ますが灰色で、選べません（メニューと同じ扱いです）。

## 操作

| キー | 左の列（グループ） | 右の列（機能） |
|---|---|---|
| `↑` `↓` | グループを選ぶ | 機能を選ぶ |
| `Home` `End` | 先頭、末尾のグループ | 先頭、末尾の機能 |
| `PageUp` `PageDown` | なし | 10 件ずつ動く |
| `→` `Enter` | 右の列に入る（機能のないグループには入らない） | 選んだ機能を動かして、一覧を閉じる |
| `←` | なし | 左の列へ戻る |
| `Esc` | 一覧を閉じる | 左の列へ戻る |

- 灰色の機能は、`Enter` を押しても、何も起きません。一覧も開いたままです。
- マウスでも、グループや機能を選べます。グループを押すと、右の列に入ります。機能を押すと、その機能が動きます。
- 一覧が開いている間は、キーはすべて一覧が受け取ります。文書には届きません。
- 機能の右のキーは、macOS では メニューと同じ、Mac の書き方で出ます。

## サクラエディタとの違い

- サクラエディタは、ポップアップメニューを出します。マツエディタは、2 列の一覧です。
- サクラエディタが実行時に足す 3 つのグループ（登録したマクロ、カスタムメニュー、プラグインのコマンド）は、まだ一覧にありません。
- メニューの項目に付く頭文字（アクセスキー）は、まだ付きません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_MENU_ALLFUNC`、`sakura_core/func/Funccode.cpp`（グループごとの並び）
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/menu_allfunc.rs`、`crates/matsu-ui/src/support/command_list.rs`（`groups`、`Cursor::key`）、`crates/matsu-commands/src/sakura/function_lists.rs`（`FUNCTION_LISTS`）
- 試験: `crates/matsu-ui/src/support/command_list.rs` の `tests` モジュール
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`CommandList`）
