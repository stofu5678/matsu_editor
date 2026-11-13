# すべて最小化

すべてのウィンドウを最小化します。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > すべて最小化 |
| 機能名 | `F_MINIMIZE_ALL` |
| 既定のキー | `Ctrl+Alt+Shift+↓` |
| マクロ | `MinimizeAll` |
| キーマクロに記録 | しない |

## 動き

このコマンドは GPUI の最小化の機能で動くので、Linux でも動きます（ほかのウィンドウ配置のコマンドと違い、OS 固有の呼び出しを使いません）。

## サクラエディタとの違い

- 違いはありません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_MINIMIZE_ALL`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/minimize_all.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`minimize_windows`）
