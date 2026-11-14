# 前のグループに移動

いまのタブを、前のウィンドウに移します。動きは「次のグループに移動」の逆向きです。最初のウィンドウから使うと、最後のウィンドウに移ります。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > 前のグループに移動 |
| 機能名 | `F_TAB_JOINTPREV` |
| 既定のキー | なし |
| マクロ | `TabJointPrev` |
| キーマクロに記録 | しない |

## 動き

上のとおりです。

## サクラエディタとの違い

- 「次のグループに移動」と同じです。

## 根拠

- サクラのソース: `sakura_core/window/CTabWnd.cpp` の `JoinPrev`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tab_jointprev.rs`、`crates/matsu-ui/src/tabs/flows.rs`（`join_tab`）
