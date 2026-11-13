# 前のグループ

前のグループ（ウィンドウ）を前面に出します。最初のグループから使うと、最後に移ります。動きは「次のグループ」の逆向きです。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > 前のグループ |
| 機能名 | `F_PREVGROUP` |
| 既定のキー | なし |
| マクロ | `PrevGroup` |
| キーマクロに記録 | しない |

## 動き

上のとおりです。

## サクラエディタとの違い

- 「次のグループ」と同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_PREVGROUP`、`sakura_core/window/CTabWnd.cpp` の `PrevGroup`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/prevgroup.rs`、`crates/matsu-ui/src/tabs/order.rs`（`prev_group`）
