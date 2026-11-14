# 縦方向に最大化

いまのウィンドウを、画面の作業領域の高さいっぱいにします。左端と幅は変えません。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 縦方向に最大化 |
| 機能名 | `F_MAXIMIZE_V` |
| 既定のキー | `Ctrl+Alt+Shift+↑` |
| マクロ | `MaximizeV` |
| キーマクロに記録 | しない |

## 動き

Linux では、OS（Wayland）がプログラムにウィンドウの位置の変更を許さないため、このコマンドは「使えない」と返り、ウィンドウは動きません。Windows と macOS では動きます。

## サクラエディタとの違い

- Linux では使えません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_MAXIMIZE_V`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/maximize_v.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`maximize_window`）、`crates/matsu-platform/src/window/arrange.rs`（`maximize_v`）
