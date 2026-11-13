# グループ化

文書を、1 つのウィンドウのタブにまとめるか、1 文書 1 ウィンドウにばらすかを切り替えます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > グループ化 |
| 機能名 | `F_BIND_WINDOW` |
| 既定のキー | なし |
| マクロ | なし（マクロ名を持たない） |
| キーマクロに記録 | しない |

## 動き

- タブバーを表示しているときだけ働きます。タブバーを隠していると、何も起きません。
- 切り替えると、共通設定の「タブをまとめない」（`bDispTabWndMultiWin`、`[Common]`）が反転し、設定ファイルに保存されます。
- まとめる側に切り替えると、すべてのウィンドウの文書が、いまのウィンドウのタブに集まります。ばらす側に切り替えると、どのタブも自分のウィンドウを持ちます。
- ファイルを読み込み中のウィンドウがあるあいだは切り替えず、「ファイルの読み込み中は、タブの表示を切り替えられない」と知らせます。読み込み中のウィンドウや、実行中のシェルがあるウィンドウは、まとめるときもばらすときも、そのままの場所に残ります。

グループは、サクラでは「枠を共有するウィンドウの集まり」です。MATSU Editor では、1 つのウィンドウが 1 つのグループに当たります（ADR-0015）。

## サクラエディタとの違い

- サクラは別々のウィンドウを枠にまとめて見せますが、MATSU Editor はウィンドウの中にタブを持ちます（GPUI に、ほかのウィンドウを取り込む仕組みがないため。ADR-0015）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_BIND_WINDOW`、`sakura_core/env/CAppNodeManager.cpp` の `ResetGroupId`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/bind_window.rs`、`crates/matsu-ui/src/tabs/flows.rs`（`change_settings`、`regroup`）、`crates/matsu-ui/src/tabs/settings.rs`
- `docs/adr/0015-tabs-and-groups.md`
