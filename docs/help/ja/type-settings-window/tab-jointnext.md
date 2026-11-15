# 次のグループに移動

いまのタブを、次のウィンドウに移します。移したタブは、そのウィンドウのアクティブなタブになり、そのウィンドウが前面に出ます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > 次のグループに移動 |
| 機能名 | `F_TAB_JOINTNEXT` |
| 既定のキー | なし |
| マクロ | `TabJointNext` |
| キーマクロに記録 | しない |

## 動き

- タブモード（タブバーを表示していて、ウィンドウをまとめる設定のとき。共通設定の「タブバー」ページで決めます）のときだけ働きます。ほかのウィンドウがないときは、何も起きません。
- 「次」は、ウィンドウを開いた順です。最後のウィンドウから使うと、最初のウィンドウに移ります。
- ウィンドウにタブが 1 つだけのときは、そのタブが移り、空になった元のウィンドウは閉じます。
- ファイルを読み込み中のウィンドウでは、何も起きません。

## サクラエディタとの違い

- サクラの `JoinNext` と同じ動きです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_TAB_JOINTNEXT`、`sakura_core/window/CTabWnd.cpp` の `JoinNext`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tab_jointnext.rs`、`crates/matsu-ui/src/tabs/flows.rs`（`join_tab`）
