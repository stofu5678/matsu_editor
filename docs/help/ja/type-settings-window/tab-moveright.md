# タブを右に移動

いまのタブを、右隣のタブと入れ替えます。いちばん右のタブでは、何も起きません。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > タブを右に移動 |
| 機能名 | `F_TAB_MOVERIGHT` |
| 既定のキー | なし |
| マクロ | `TabMoveRight` |
| キーマクロに記録 | しない |

## 動き

- タブモード（タブバーを表示していて、ウィンドウをまとめる設定のとき。共通設定の「タブバー」ページで決めます）のときだけ働きます。まとめていないときは、ウィンドウに並びがないため、何も起きません。
- 動かすのは、ウィンドウの中のタブの並びだけです。ほかのウィンドウへは移りません（移すには「次のグループに移動」を使います）。

## サクラエディタとの違い

- サクラの動き（右隣と入れ替える。右端では何もしない）と同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_TAB_MOVERIGHT`、`sakura_core/window/CTabWnd.cpp` の `MoveRight`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tab_moveright.rs`、`crates/matsu-ui/src/tabs/order.rs`（`right_of`）
- 試験: `crates/matsu-ui/src/tabs/order.rs` の `moving_a_tab_stops_at_the_ends`
