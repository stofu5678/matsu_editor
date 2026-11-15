# ウィンドウ一覧...

開いているウィンドウの一覧を、ポップアップで出します。選んだウィンドウが前面に出ます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > ウィンドウ一覧... |
| 機能名 | `F_WINLIST` |
| 既定のキー | なし |
| マクロ | `WindowList` |
| キーマクロに記録 | しない |

## 動き

- 一覧は、ウィンドウを開いた順です。各行の頭に、アクセスキーが付きます。1〜9、A〜Z の順で、その次は 0、そのあとはまた 1 から始まります。
- 行の文字は、ファイル名（名前のない文書は「(無題)」）、変更があるときの ` *`、文字コードです。文字コードは、Shift_JIS のとき出しません。ファイル名の見せ方は、共通設定「ファイル名表示」に従います。
- いま使っているウィンドウに、チェックの印が付きます。
- 操作は、アクセスキー、`↑` `↓`、`Enter`、`Esc` です。一覧がすでに開いているときにもう一度使うと、閉じます。
- タブバーを表示しているときは、このポップアップの代わりに、タブバーの「▼」ボタンと同じタブの一覧が出ます。

ポップアップは、OS のメニューではなく、MATSU Editor が本文の上に描きます（GPUI に、好きな場所へ出せるポップアップメニューがないため）。

## サクラエディタとの違い

- サクラの一覧は OS のポップアップメニューです。MATSU Editor の一覧は、本文の上に描くものです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_WINLIST`、`sakura_core/window/CEditWnd.cpp` の `PopupWinList`、`sakura_core/window/CTabWnd.cpp` の `TabListMenu`、`sakura_core/env/CFileNameManager.cpp` の `GetMenuFullLabel`、`GetAccessKeyByIndex`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/winlist.rs`、`crates/matsu-ui/src/window_ops/list.rs`、`crates/matsu-ui/src/tabs/list.rs`
- 試験: `crates/matsu-ui/src/window_ops/tests.rs`
