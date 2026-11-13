# 文字入力と IME

文字を打つと、キャレットの位置に文字が入ります。日本語など、IME（入力メソッド）で変換して入力する文字は、変換中の文字が、そのままテキストの中に下線付きで見えます。

## 挿入モードと上書きモード

`Ins` キーで、挿入モードと上書きモードが切り替わります（機能 `F_CHGMOD_INS`）。ステータスバーの「挿入」「上書」の欄をダブルクリックしても、切り替えられます。

- 挿入モード: 打った文字が、キャレットの位置に割り込みます。キャレットは、細い棒の形です。
- 上書きモード: 打った文字が、キャレットの右にある文字を 1 文字ずつ置き換えます。キャレットは、文字の幅の箱になります。

共通設定の「カーソル形状」で、キャレットの形を、Windows 風（棒）か MS-DOS 風（箱。挿入モードでは高さが半分）から選べます。

上書きモードの細かい動きは、次のとおりです。

- 行の終わりでは、文字は挿入されます。
- 共通設定の「編集」の「改行コードは上書きしない」（既定はオン）がオフだと、行末で打つと、改行が消えて次の行がつながります。
- 「文字幅に合わせてスペースを詰める」をオンにすると、置き換える文字と置き換えられる文字の幅が違うとき、残りの桁がずれないように、空白を足して（全角に置き換えるときは次の文字も取って）そろえます。既定はオフです。
- 文字を選んでいるときは、挿入と上書きのどちらでも、選んだ範囲が打った文字に置き換わります。
- 改行は、上書きされません。

## 行末より右に打つ（フリーカーソル）

共通設定の「全般」の「フリーカーソル」をオンにすると、キャレットを、行の終わりより右の、文字のない場所にも置けます。そこで文字を打つと、行末からキャレットまでが空白で埋められてから、文字が入ります。既定はオフです。

## 文字を打つとき、ほかに起きること

文字を打ったときや `Enter` を押したときの、自動の動きは、文書のタイプ（拡張子で決まります）の設定が決めます。「タイプ別設定」の「画面」にあります。

- 自動インデント: `Enter` で改行したとき、前の行の頭の空白を引き継ぎます。
- スマートインデント: C 系の言語などで、括弧に合わせて字下げします。プラグインが担当することもできます。
- 改行時に末尾の空白を削除する、Tab を空白で入れる（ソフトタブ）なども、同じ場所で決めます。

これらの設定の説明は、「コマンド: 設定（タイプ別設定）とウィンドウ」の章にあります。

## IME で入力する

変換中の文字は、文書の中に仮に入っていて、下線で示されます。確定すると下線が消え、取り消すと文字がなくなります。

- 1 回の変換（下線が付いてから、確定または取り消すまで）の間の編集は、取り消し（`Ctrl+Z`）のとき、1 つの手順として戻ります。
- IME の変換候補の窓は、変換中の文字の近くに出ます。
- 行末より右の空きの場所で変換を始めたときは、空白が先に入り、変換を取り消すと、その空白も一緒になくなります。
- 上書きモードで確定すると、確定した文字数だけ、右の文字を置き換えます。変換を始めたときに範囲を選んでいたときは、それ以上は置き換えません。
- 1 行が非常に長いとき（巨大なファイル）は、変換中の文字の前後の一部だけを IME に渡します。この制限のために、長い行では IME の動きが十分に調べられていません（`docs/sakura-gaps.md` の「巨大ファイルの残り」）。

### 起動時の IME の状態（Windows のみ）

「タイプ別設定」の「ウィンドウ」の「起動時のIME」で、文書を開いたときの IME のオン・オフと入力モードを、タイプごとに決められます。この設定は Windows だけで効きます。macOS と Linux では、OS がアプリから IME の状態を決めることを許さないため、設定の項目は灰色です。

### 再変換

「再変換」（`F_RECONVERT`。「編集」メニュー）は、キャレットの周りの文字を、IME にもう一度変換させます。

- Windows: 選んでいるか、キャレットのある文字を IME に渡し、IME が決めた範囲を選んで、変換の結果で置き換えます。
- macOS と Linux: プログラムから再変換を始める仕組みがないため、機能を呼ぶと、ステータスバーにその理由が出ます。OS の入力メソッドの、自分の再変換のキーを使ってください。

## サクラエディタとの違い

- 再変換は、macOS と Linux では使えません（上のとおり）。
- IME の起動時の状態は、Windows だけです。
- 巨大なファイルの長い行での IME は、制限があります。

## 根拠

- マツエディタのコード: `crates/matsu-ui/src/view/input.rs`（キー入力と IME。変換中の文字、取り消しのまとめ方）、`crates/matsu-commands/src/context/mod.rs`（`type_text`、`overwrite_replacement`）、`crates/matsu-commands/src/edit_options.rs`（`EditOptions::DEFAULT`）、`crates/matsu-commands/src/context/free_cursor.rs`、`crates/matsu-ui/src/view/caret_shape.rs`、`crates/matsu-ui/src/doc_window.rs`、`crates/matsu-ui/src/reconvert/mod.rs`、`crates/matsu-ui/src/ui_commands/ime/`、`crates/matsu-commands/src/input_options.rs`
- 設定の画面: `crates/matsu-ui/src/common_settings/pages/edit.rs`（上書きモード）、`crates/matsu-ui/src/common_settings/pages/general.rs`（フリーカーソル、カーソル形状）、`crates/matsu-ui/src/type_settings/pages/window.rs`（起動時のIME）
- 試験: `crates/matsu-ui/src/view/mouse_tests.rs` の `an_ime_composition_in_the_gap_follows_spaces_in_one_undo_step`、`cancelling_a_composition_in_the_gap_takes_the_spaces_with_it`
- サクラのソース: `sakura_core/cmd/CViewCommander_Edit.cpp`（`Command_WCHAR`、`Command_IME_CHAR`、`DelCharForOverwrite`）、`sakura_core/cmd/CViewCommander_Edit_advanced.cpp`（`Command_Reconvert`）、`sakura_core/view/CEditView_Ime.cpp`、`sakura_core/view/CCaret.cpp`（`ShowEditCaret`）
- 差の記録: `docs/sakura-gaps.md`（「IME の再変換」「Windows 以外の IME の状態・入力モードと文書アイコン」「巨大ファイルの残り」の行）
