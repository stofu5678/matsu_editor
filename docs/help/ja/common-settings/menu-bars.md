# バーの表示と非表示

「設定」メニューの上の 5 つは、ウィンドウの部品を出したり隠したりします。押すたびに、表示と非表示が替わります。

| メニュー項目 | 機能名 | マクロ | 既定のキー | 設定のキー |
|---|---|---|---|---|
| ツールバーの表示／非表示 | `F_SHOWTOOLBAR` | `ShowToolbar` | `Ctrl+1` | `bDispTOOLBAR` |
| ファンクションキーの表示／非表示 | `F_SHOWFUNCKEY` | `ShowFunckey` | `Ctrl+2` | `bDispFUNCKEYWND` |
| タブバーの表示／非表示 | `F_SHOWTAB` | `ShowTab` | なし | `bDispTabWnd` |
| ステータスバーの表示／非表示 | `F_SHOWSTATUSBAR` | `ShowStatusbar` | `Ctrl+3` | `bDispSTATUSBAR` |
| ミニマップの表示／非表示 | `F_SHOWMINIMAP` | `ShowMiniMap` | なし | `bDispMiniMap` |

どれも引数はなく、キーマクロには記録されません。設定のキーは、`[Common]` セクションにあります。

## 動き

- ツールバー、ファンクションキーのバー、ミニマップは、すべてのウィンドウで同時に替わります。ウィンドウが、設定のキーを見ています
- タブバーは、替えると、ウィンドウのまとまりも変わります。出すときは、開いているウィンドウの文書が、このウィンドウのタブに集まります。隠すときは、タブごとにウィンドウが分かれます。切り替えの結果は、`bDispTabWnd` に書かれます
- ステータスバーは、いまのウィンドウの分だけ替わり、メニューで替えた状態はセッションの間だけ覚えています。ファイルへは、「共通設定」ダイアログで OK を押したときに、ダイアログを開いたウィンドウの状態が `bDispSTATUSBAR` として書かれます

同じ設定が、「共通設定」の [ウィンドウ](page-window.md) ページと [タブバー](page-tab.md) ページにもあります。

## サクラエディタとの違い

ステータスバーの切り替えがファイルへすぐには書かれないのは、MATSU Editor の実装の事情です。サクラエディタは、共通設定の値を書き換えます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Settings.cpp` の `Command_SHOWTOOLBAR`、`Command_SHOWFUNCKEY`、`Command_SHOWTAB`、`Command_SHOWSTATUSBAR`、`Command_SHOWMINIMAP`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/settings/show_toolbar.rs`、`show_funckey.rs`、`show_status_bar.rs`、`show_minimap.rs`、`crates/matsu-ui/src/ui_commands/window/showtab.rs`、`crates/matsu-ui/src/settings/mod.rs`（ステータスバーの扱いの注釈）、`crates/matsu-ui/src/common_settings/mod.rs`（OK で `bDispSTATUSBAR` を書く）
- マクロ名と既定のキー: `crates/matsu-commands/src/sakura/macro_functions.rs`、`crates/matsu-commands/src/sakura/key_bindings.rs`
