# 前のウィンドウ

前のウィンドウを前面に出します。最初のウィンドウから使うと、最後のウィンドウに移ります。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 前のウィンドウ |
| 機能名 | `F_PREVWINDOW` |
| 既定のキー | `Ctrl+Shift+Tab` |
| マクロ | `PrevWindow` |
| キーマクロに記録 | しない |

## 動き

動きは「次のウィンドウ」の逆向きです。順番、最小化したウィンドウ、タブモードの扱い、ウィンドウが 1 つだけのときの動きは、そちらを見てください。

## サクラエディタとの違い

- 分割した区画を移らないことも、「次のウィンドウ」と同じです。

## 根拠

- サクラのソース: `sakura_core/doc/CEditDoc.cpp` の `HandleCommand`、`sakura_core/_main/CControlTray.cpp` の `ActiveNextWindow`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/prevwindow.rs`、`crates/matsu-ui/src/tabs/order.rs`（`prev_tab`）
