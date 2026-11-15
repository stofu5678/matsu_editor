# 再描画

いまの画面を描き直します。表示が乱れたときに使います。文書の内容や設定は変わりません。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 再描画 |
| 機能名 | `F_REDRAW` |
| 既定のキー | `F5` |
| マクロ | `ReDraw` |
| キーマクロに記録 | する |

## 動き

上のとおりです。

## サクラエディタとの違い

- サクラの `RedrawAll` に当たる動きを、GPUI の再描画の要求（`Window::refresh`）で行います。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_REDRAW`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/redraw.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F5`）
