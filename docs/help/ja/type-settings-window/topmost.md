# 常に手前に表示

いまのウィンドウを、ほかのウィンドウより手前に置いたままにします。もう一度使うと、元に戻ります。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 常に手前に表示 |
| 機能名 | `F_TOPMOST` |
| 既定のキー | なし |
| マクロ | `WindowTopMost(n)` |
| キーマクロに記録 | する |

## 動き

メニューでは、手前に置いている間、チェックの印が付きます。マクロでは、引数 `n` で動きを決めます。

| `n` | 動き |
|---|---|
| 0 | 切り替える（メニューから使ったときの動き） |
| 1 | 手前に置く |
| 2 | 手前に置くのをやめる |
| そのほか | 何もしない |

Windows では最前面のウィンドウ（`HWND_TOPMOST`）に、macOS では「フローティング」のウィンドウレベルにします。Linux では、OS（Wayland）がプログラムにウィンドウを手前に置き続けることを許さないため、このコマンドは「使えない」と返り、何も変わりません。

## サクラエディタとの違い

- Linux では使えません。引数の意味は、サクラの `WindowTopMost` と同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_WINTOPMOST`、`sakura_core/window/CEditWnd.cpp` の `WindowTopMost`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/topmost.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`topmost`、`is_topmost`）、`crates/matsu-platform/src/window/`
- 試験: `crates/matsu-ui/src/window_ops/tests.rs`
