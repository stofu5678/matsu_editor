# 上下に並べて表示

すべてのウィンドウを、画面の作業領域に、上から下へ重ならないように並べます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 上下に並べて表示 |
| 機能名 | `F_TILE_V` |
| 既定のキー | `Ctrl+Alt+H` |
| マクロ | `TileWinV` |
| キーマクロに記録 | しない |

## 動き

- 最小化しているウィンドウは動かしません。
- 各ウィンドウは作業領域の幅いっぱいで、高さを等分します。
- いまのウィンドウが、いちばん上に来ます。元のいちばん上のウィンドウは、いまのウィンドウがあった位置に移ります（2 つが入れ替わります）。ほかは、開いた順です。
- 並べたあと、いまのウィンドウにフォーカスが移ります。

Linux では、OS（Wayland）がプログラムにウィンドウの位置の変更を許さないため、このコマンドは「使えない」と返り、ウィンドウは動きません。Windows と macOS では動きます。

## サクラエディタとの違い

- Linux では使えません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_TILE_V`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/tile_v.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`tile_windows`）、`crates/matsu-platform/src/window/arrange.rs`（`tile_v`、`tile_order`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`H` の Ctrl+Alt）
