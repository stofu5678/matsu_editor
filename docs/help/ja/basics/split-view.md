# 画面分割

画面分割は、同じ文書を、1 つのウィンドウの中の 2 つまたは 4 つの窓（ペイン）で見る機能です。文書の離れた場所を、同時に見ながら書くときに使います。

## 分割する

| 機能 | キー | 動き |
|---|---|---|
| 上下に分割／分割解除（`F_SPLIT_V`） | `F4`、`Ctrl+Shift+-` | 上下 2 つに分ける |
| 左右に分割／分割解除（`F_SPLIT_H`） | `Shift+F4`、`Ctrl+Shift+\` | 左右 2 つに分ける |
| 縦横に分割／分割解除（`F_SPLIT_VH`） | `Ctrl+Shift+;` | 4 つに分ける |

機能は「ウィンドウ」メニューの先頭にもあります。すでに分かれているときに同じ機能を呼ぶと、その分割が解除されます。

macOS のキーボードでは、`F4` に `fn` キーが要ることがあります。US 配列のキーボードでは、記号のキーが日本語配列と違う場所にあります（「[キーで動く・選ぶ](keys.md)」を見てください）。

### マウスで分割する

スクロールバーの端に、小さな「分割ボックス」があります。縦のバーの上のボックスを動かすと上下に、横のバーの左のボックスを動かすと左右に分かれます。すでに分かれた窓の間の境界の線は、ドラッグして位置を変えられます。

- 境界の線をウィンドウの端（32 ピクセルより近く）まで動かして離すと、その分割が解除されます。
- 境界の線をダブルクリックしても、その線の分割が解除されます。

## 分割した窓の動き

- どの窓も、同じ文書を見ています。ある窓での編集は、ほかの窓にすぐ出ます。
- 窓ごとに、自分のキャレット、選択範囲、スクロールの位置を持ちます。
- アクティブな窓（キーボードの入力が行く窓）は、1 つです。タイトル、ステータスバー、機能の実行は、アクティブな窓に対して行われます。
- 1 つの窓をスクロールすると、隣の窓も一緒にスクロールします。縦と横を別々に、共通設定の「ウィンドウ」の「垂直スクロールの同期をとる」「水平スクロールの同期をとる」で切り替えます（どちらも既定はオン）。縦は上下に並んだ窓、横は左右に並んだ窓の間でそろいます。
- 窓の番号は、左上が 0、右上が 1、左下が 2、右下が 3 です。上下に分けると 0 と 2、左右に分けると 0 と 1、4 つに分けると 0 から 3 が出ます。

## 制限

- キーマクロの再生中の分割の機能は、次の機能が分割前の大きさで動く不具合があります（既知。`docs/sakura-gaps.md`）。

## 根拠

- マツエディタのコード: `crates/matsu-ui/src/panes/mod.rs`（窓、境界の線、ダブルクリック、同期スクロール）、`crates/matsu-ui/src/panes/plan.rs`（分割の規則、`do_split`）、`crates/matsu-ui/src/view/split_box.rs`（分割ボックス）、`crates/matsu-ui/src/ui_commands/split/`、`crates/matsu-ui/src/common_settings/pages/window.rs`（`SplitterSync`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F4`、`-`、`\`、`;` の行）
- 試験: `crates/matsu-ui/src/panes/plan.rs` の `the_same_command_again_removes_the_split`、`a_bar_near_an_edge_removes_the_split`
- サクラのソース: `sakura_core/window/CSplitterWnd.cpp`、`sakura_core/window/CSplitBoxWnd.cpp`、`sakura_core/view/CEditView_Scroll.cpp`（`SyncScrollV`、`SyncScrollH`）
- 差の記録: `docs/sakura-gaps.md`（キーマクロ再生中の分割の行）
