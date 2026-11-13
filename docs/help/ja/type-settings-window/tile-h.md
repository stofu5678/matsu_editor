# 左右に並べて表示

すべてのウィンドウを、画面の作業領域に、左から右へ重ならないように並べます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 左右に並べて表示 |
| 機能名 | `F_TILE_H` |
| 既定のキー | `Ctrl+Alt+T` |
| マクロ | `TileWinH` |
| キーマクロに記録 | しない |

## 動き

- 最小化しているウィンドウは動かしません。
- 各ウィンドウは作業領域の高さいっぱいで、幅を等分します。
- いまのウィンドウが、いちばん左に来ます。そのとき、元からいちばん左にあるはずだったウィンドウは、いまのウィンドウの元の位置に入ります（2 つが入れ替わります）。ほかは、開いた順です。
- 並べたあと、いまのウィンドウにフォーカスが移ります。

Linux では、OS（Wayland）がプログラムにウィンドウの位置の変更を許さないため、このコマンドは「使えない」と返り、ウィンドウは動きません。Windows と macOS では動きます。

## サクラエディタとの違い

- Linux では使えません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_TILE_H`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tile_h.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`tile_windows`）、`crates/matsu-platform/src/window/arrange.rs`（`tile_h`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`T` の Ctrl+Alt）
