# 右をすべて閉じる

いまのタブより右にあるタブを、すべて閉じます。いまのタブは閉じません。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > タブの操作 > 右をすべて閉じる |
| 機能名 | `F_TAB_CLOSERIGHT` |
| 既定のキー | なし |
| マクロ | `TabCloseRight` |
| キーマクロに記録 | しない |

## 動き

動きは「左をすべて閉じる」の逆向きです（タブバーが必要なこと、保存の確認、「キャンセル」で止まること、プレビューのタブの扱い）。

## サクラエディタとの違い

- 「左をすべて閉じる」と同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_TAB_CLOSERIGHT`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tab_closeright.rs`、`crates/matsu-ui/src/tabs/flows.rs`（`close_side`）、`crates/matsu-ui/src/tabs/order.rs`（`right_range`）
