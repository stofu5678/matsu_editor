# 左右に分割／分割解除

ウィンドウの本文を、左と右の 2 つの区画に分けます。すでに左右に分かれているときは、左右の分割を解除します。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 左右に分割／分割解除 |
| 機能名 | `F_SPLIT_H` |
| 既定のキー | `Shift+F4`、`Ctrl+Shift+\` |
| マクロ | `SplitWinH` |
| キーマクロに記録 | しない |

## 動き

押すたびに、次のように変わります。

| いまの状態 | 押したあと |
|---|---|
| 分割していない | 左右に分かれる（仕切りは幅の半分） |
| 左右に分かれている | 分割していない |
| 上下に分かれている | 4 つに分かれる（上下の仕切りはそのまま） |
| 4 つに分かれている | 上下だけに分かれる（左右の仕切りがなくなる） |

分割したあとの 4 つの区画（ペイン）は、左上が 0、右上が 1、左下が 2、右下が 3 の番号を持ちます。区画 0 はいつもあります。新しく出る区画は、いま使っている区画のカーソル位置・選択・スクロールを写して始まります。

分割を解除するときに、消える区画でマクロが動いていると、解除せず「マクロの実行中は、その画面の分割を解除できない」と知らせます。

## サクラエディタとの違い

- 規則は「上下に分割／分割解除」と同じで、向きだけが違います。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_SPLIT_H`、`sakura_core/window/CSplitterWnd.cpp` の `HSplitOnOff`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/split/split_h.rs`、`crates/matsu-ui/src/panes/plan.rs`、`crates/matsu-ui/src/panes/mod.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F4` の Shift、`\` の Ctrl+Shift）
