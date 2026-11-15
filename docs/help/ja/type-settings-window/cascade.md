# 重ねて表示

すべてのウィンドウを、少しずつずらして重ねます。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 重ねて表示 |
| 機能名 | `F_CASCADE` |
| 既定のキー | `Ctrl+Alt+E` |
| マクロ | `CascadeWin` |
| キーマクロに記録 | しない |

## 動き

- 最小化しているウィンドウは動かしません。
- ウィンドウの大きさは、いまのウィンドウがある画面の作業領域（タスクバーやメニューバー、Dock を除いた範囲）の、縦横それぞれ 5 分の 4 です。
- 1 つずつ、タイトルバーの高さほど右下へずらして置きます。右か下にはみ出すときは、上へ戻り、少し右へずらして続けます。右にも余地がなければ、左上に戻ります。
- いまのウィンドウが最後（いちばん手前）になります。ほかのウィンドウは、開いた順に下から重なります。

Linux では、OS（Wayland）がプログラムにウィンドウの位置の変更を許さないため、このコマンドは「使えない」と返り、ウィンドウは動きません。Windows と macOS では動きます。

## サクラエディタとの違い

- Linux では使えません（上のとおり）。ほかは、サクラと同じ並べ方です。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_CASCADE`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/cascade.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`cascade_windows`）、`crates/matsu-platform/src/window/arrange.rs`（`cascade`）、`crates/matsu-platform/src/window/mod.rs`
- 試験: `crates/matsu-ui/src/window_ops/tests.rs`、`arrange.rs` の試験
- `docs/sakura-gaps.md` の「ウィンドウの配置・常に手前（Linux）」の行
