# 縦横に分割／分割解除

ウィンドウの本文を、上下と左右の両方で 4 つの区画に分けます。すでに 4 つに分かれているときは、すべての分割を解除します。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 縦横に分割／分割解除 |
| 機能名 | `F_SPLIT_VH` |
| 既定のキー | `Ctrl+Shift+;` |
| マクロ | `SplitWinVH` |
| キーマクロに記録 | しない |

## 動き

| いまの状態 | 押したあと |
|---|---|
| 分割していない | 4 つに分かれる（仕切りは高さと幅の半分） |
| 上下だけ、または左右だけに分かれている | 4 つに分かれる（いまある仕切りは動かさず、足りない向きを半分に置く） |
| 4 つに分かれている | 分割していない |

分割したあとの 4 つの区画（ペイン）は、左上が 0、右上が 1、左下が 2、右下が 3 の番号を持ちます。区画 0 はいつもあります。新しく出る区画は、いま使っている区画のカーソル位置・選択・スクロールを写して始まります。

分割を解除するときに、消える区画でマクロが動いていると、解除せず「マクロの実行中は、その画面の分割を解除できない」と知らせます。

## サクラエディタとの違い

- 規則は「上下に分割／分割解除」と同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_SPLIT_VH`、`sakura_core/window/CSplitterWnd.cpp` の `VHSplitOnOff`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/split/split_vh.rs`、`crates/matsu-ui/src/panes/plan.rs`の `request`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`;` の Ctrl+Shift）
