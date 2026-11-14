# タブを左に移動

いまのタブを、左隣のタブと入れ替えます。いちばん左のタブでは、何も起きません。条件は「タブを右に移動」と同じです。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > タブを左に移動 |
| 機能名 | `F_TAB_MOVELEFT` |
| 既定のキー | なし |
| マクロ | `TabMoveLeft` |
| キーマクロに記録 | しない |

## 動き

上のとおりです。

## サクラエディタとの違い

- 「タブを右に移動」と同じです。

## 根拠

- サクラのソース: `sakura_core/window/CTabWnd.cpp` の `MoveLeft`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tab_moveleft.rs`、`crates/matsu-ui/src/tabs/order.rs`（`left_of`）
