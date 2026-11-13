# 文字の入力、テキストの挿入、フォルダ名のコピー

メニューにない、入力系・挿入系・クリップボード系の機能です。文字を打つ動きの中身と、マクロからテキストを入れる機能が入っています。

| 機能名 | 表示名 | キー | マクロ（引数） | キーマクロ |
|---|---|---|---|---|
| `F_WCHAR` | 文字入力 | なし（文字キー、Enter の中身） | `Char(文字)` | 記録する |
| `F_IME_CHAR` | 全角文字入力 | なし | `CharIme(数)` | 記録する |
| `F_INSTEXT_W` | テキストを貼り付け | なし | `InsText(文字列)` | 記録する |
| `F_ADDTAIL_W` | （名前なし） | なし | `AddTail(文字列)` | 記録しない |
| `F_COPYDIRPATH` | このファイルのフォルダー名をコピー | なし | `CopyDirPath()` | 記録する |

コントロールコードを入れる `F_CTRL_CODE`（マクロ `CtrlCode`）は、「コマンド: 編集」の章にあります。

## 文字入力

`F_WCHAR`: 文字を 1 つ打ちます。文字キーを押したときと、Enter を押したときに、実際に動いている機能です。

- 引数は、1 文字か、その文字のコード（数）です。
- 文字が CR か LF のときは、改行を入れます。入る改行は、文書の「入力改行コード」です。自動インデントがオンなら、前の行の字下げを、新しい行の頭に写します（既定でオン）。
- 選択があるときは、選択を、その文字に置き換えます。
- 上書きモードで選択がないときは、キャレットの位置の 1 文字（TAB、全角、サロゲートペアも 1 文字）を、その文字に置き換えます。行の終わりでは、改行を消さずに、文字を挿入します（既定の設定のとき）。文書の最後でも、挿入です。
- 矩形選択があるときは、各行の矩形の前に文字を入れ、矩形は右へずれます。Enter のときは、矩形を先に消します。
- 自動インデントなどの細かい動きは、「タイプ別設定」の「スクリーン」ページの、インデントの設定で変わります。

## 全角文字入力

`F_IME_CHAR`: IME が渡した 1 文字を打ちます。マクロでは `CharIme(数)` です。

- 引数の数は、16 ビットで切って使います。0x100 より小さい数は、`F_WCHAR` と同じです。
- 数が、サロゲートペアの片方（0xD800 から 0xDFFF）のときは、エラーになります。マツエディタの文字は UTF-8 で持つため、片方だけの文字は入れられません。
- サクラエディタは、IME の文字を 1 文字ずつ受け取りますが（上書きモードのとき）、マツエディタの IME 入力は、文字列を一度に渡します。

## テキストを貼り付け

`F_INSTEXT_W`: 引数の文字列を、キャレットの位置に入れます。クリップボードは使いません。

- 選択があるときは、選択を置き換えます。入れたあと、キャレットは文字列のうしろに来ます。
- 矩形選択があるときは、文字列の最初の行を、矩形の各行に入れます。
- 改行は変換せず、文字列にあるとおりに入れます。
- 引数がないと、マクロはエラーで止まります。

## 末尾に追加

`F_ADDTAIL_W`: 引数の文字列を、文書の最後に足します。

- まず文書の最後へ動きます。選択（矩形選択も）は、置き換えずに、外します。選択の固定（「範囲選択開始」の状態）があれば、解きます。
- 文字列を入れたあと、キャレットは文字列のうしろに来ます。
- 引数がないと、マクロはエラーで止まります。

## このファイルのフォルダー名をコピー

`F_COPYDIRPATH`: 文書のファイルのあるフォルダの名前を、クリップボードに入れます。

- 最後の `\` は付けません（ドライブの直下なら `C:` になります）。
- ファイルのない文書では、ビープ音を鳴らすだけで、何もコピーしません。

ファイルの名前やパスをコピーする機能は、「コマンド: 編集」の章にあります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit.cpp` の `Command_WCHAR`、`Command_IME_CHAR`、`sakura_core/cmd/CViewCommander_Clipboard.cpp` の `Command_INSTEXT`、`Command_ADDTAIL`、`Command_COPYDIRPATH`、`sakura_core/macro/CMacro.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/input_char.rs`、`ime_char.rs`。`crates/matsu-commands/src/commands/clipboard/ins_text.rs`、`add_tail.rs`、`copydirpath.rs`。`crates/matsu-ui/src/view/mod.rs`（Enter の扱い）、`crates/matsu-ui/src/view/input.rs`（文字の入力）
- 試験: `crates/matsu-commands/src/commands/clipboard/ins_text.rs` の `inserts_at_the_caret`、`replaces_the_selection`、`keeps_the_line_endings_of_the_text`、`add_tail.rs` の `adds_at_the_end_and_leaves_the_caret_after_it`、`a_selection_is_dropped_not_replaced`、`copydirpath.rs` の `copies_the_folder_of_the_file`、`a_document_without_a_file_beeps`
- マクロ名、引数の型、キーマクロの記録: `crates/matsu-commands/src/sakura/macro_functions.rs`
