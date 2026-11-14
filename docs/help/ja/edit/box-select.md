# 矩形選択

2 つの表示桁と 2 つの行で囲んだ、四角い範囲を選びます。「編集」→「矩形選択」の副メニューにある機能です。

## 矩形選択の始め方

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_BEGIN_BOX` | 矩形範囲選択開始 | `Shift+F6` | `BeginBoxSelect()` |

- カーソルの位置から矩形選択を始めます。もとの選択があれば外れます。
- カーソルを動かすまでは、何も選ばれません。
- 固定されるので、ふつうの移動も矩形を広げる動きになります（[範囲選択開始](begin-select.md)の矩形版です）。
- 矩形の角は、行の終わりより右（文字のない場所）にも置けます。
- マウスでは、`Alt` を押しながらドラッグしても、矩形選択になります。

## 矩形を広げる移動

次の機能は、矩形選択がなければ、まずカーソルの位置から矩形選択を始め、そのあと動いて矩形を広げます。ふつうの選択があるときは、それを外します。

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_UP2_BOX` | (矩形選択)カーソル上移動(２行ごと) | `Ctrl+Alt+↑` | `Up2_BoxSel()` |
| `F_DOWN2_BOX` | (矩形選択)カーソル下移動(２行ごと) | `Ctrl+Alt+↓` | `Down2_BoxSel()` |
| `F_WORDLEFT_BOX` | (矩形選択)単語の左端に移動 | `Ctrl+Alt+←` | `WordLeft_BoxSel()` |
| `F_WORDRIGHT_BOX` | (矩形選択)単語の右端に移動 | `Ctrl+Alt+→` | `WordRight_BoxSel()` |
| `F_GOLOGICALLINETOP_BOX` | (矩形選択)行頭に移動(改行単位) | なし | `GoLogicalLineTop_BoxSel()` |
| `F_GOLINETOP_BOX` | (矩形選択)行頭に移動(折り返し単位) | `Alt+Home` | `GoLineTop_BoxSel()` |
| `F_GOLINEEND_BOX` | (矩形選択)行末に移動(折り返し単位) | `Alt+End` | `GoLineEnd_BoxSel()` |
| `F_1PageUp_BOX` | (矩形選択)１ページアップ | `Alt+PageUp` | `PageUp_BoxSel()`、`1PageUp_BoxSel()` |
| `F_1PageDown_BOX` | (矩形選択)１ページダウン | `Alt+PageDown` | `PageDown_BoxSel()`、`1PageDown_BoxSel()` |
| `F_GOFILETOP_BOX` | (矩形選択)ファイルの先頭に移動 | `Ctrl+Alt+Home` | `GoFileTop_BoxSel()` |
| `F_GOFILEEND_BOX` | (矩形選択)ファイルの最後に移動 | `Ctrl+Alt+End` | `GoFileEnd_BoxSel()` |

- 動く先は、ほとんどが、ふつうの移動と同じです（[移動](cursor-moves.md)）。違うものは次のとおりです。
  - ファイルの先頭・最後への移動は、表示桁を保ったまま、最初の行・最後の行へ動きます（桁 0 や文書の終わりには行きません）。「最後」は、最後に改行がある文書でも、その後ろの空の行ではなく、文字のある最後の行です。
  - 行末への移動は、折り返された行では、その画面の行の最後の文字の上で止まります。
- 上下左右の 1 つずつの移動（`F_UP_BOX` など）も、同じ仕組みで動きます。メニューにはなく、キー（`Alt+↑` など）に割り当てられています。
- 行頭に移動の 2 つは、字下げの後ろの最初の文字と、行の先頭を行き来します。改行単位は、折り返された行でも、文書の行を単位にします。
- macOS では、`Alt` は `Option` キーです。

## 矩形選択に対する編集

矩形選択をしているときの、ほかの機能の動きは、それぞれのページにあります。

- [貼り付け](paste.md)、[矩形貼り付け](paste-box.md)
- [インデント](indent.md)（矩形の左端に入れる）
- [左右の空白の削除](trim.md)、[選択行のソート](sort.md)（矩形の桁を対象にする）
- [連続した重複行の削除(uniq)](uniq.md)は、矩形選択のときは何もしません（警告音も鳴りません）

## まだできないこと

- マクロの引数のうち、矩形の固定の入り切り（ロックの指定）は使えません。固定は、サクラエディタの既定どおり、いつも入として動きます。
- サクラエディタは等幅でないフォントでは矩形選択を始めませんが、MATSU Editor の画面はいつも等幅なので、その制限はありません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Select.cpp` の `Command_BEGIN_BOXSELECT`、`sakura_core/cmd/CViewCommander.cpp` の `HandleCommand` と `Sub_BoxSelectLock`
- MATSU のコード: `crates/matsu-commands/src/commands/box_select/`（`begin_box.rs`、`cursor_up2_box.rs` など）、`crates/matsu-ui/src/ui_commands/box_select/`（ページ移動）、規則は `crates/matsu-commands/src/context/boxsel.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`
- マウスの `Alt` ドラッグ: `docs/sakura-gaps.md` の「マウスの残り」の行（窓の中の操作）
