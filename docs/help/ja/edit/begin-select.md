# 範囲選択開始

「選択の固定」を入れたり切ったりします。固定している間は、カーソルを動かすと選択が広がります。

## 呼び出し方

- メニュー: 「編集」→「選択」→「範囲選択開始」
- キー: `F6`
- マクロ: `BeginSelect()`
- 機能名: `F_BEGIN_SEL`

## 動き

- 1 回目で選択の固定が入ります。選択がないときは、カーソルの位置が選択の始まりになります。すでに選択があるときは、その始まりのまま、固定だけが入ります。
- 固定している間は、ふつうの移動（上下左右、単語、行頭・行末、ページ、ジャンプなど）が、選択を広げる動きになります。
- もう 1 回使うと、固定が切れ、そのときの選択は残ります。
- 固定は、選択を外す操作（固定なしの移動、文字の入力、「モードのキャンセル」の `Esc`）でも切れます。

矩形選択で同じことをするのは、[矩形範囲選択開始](box-select.md)です。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Select.cpp` の `Command_BEGIN_SELECT`、`sakura_core/cmd/CViewCommander.cpp` の `HandleCommand`
- MATSU のコード: `crates/matsu-commands/src/commands/select/begin_sel.rs`（先頭のコメント）
- 試験: 同じファイルの `tests` モジュール
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F6` の行）
