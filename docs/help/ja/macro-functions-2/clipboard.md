# クリップボードの関数

切り取り、コピー、貼り付けと、マクロから文字列を渡して文書に入れる関数です。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `Cut` | なし | なし | 選択をクリップボードに入れて、文書から消す。選択がないときは、キャレットの行を切り取る（既定の設定）。 | する | 記録なし |
| `Copy` | なし | なし | 選択をクリップボードにコピーする。選択がないときは、キャレットの行をコピーし、行のコピーとして印を付ける。印のあるコピーは、貼り付けると、行頭に入る。選択は残る（選択をコピー後に解除する設定は、既定でオフ）が、選択ロックは切れるので、次の通常の移動で選択が解除される。 | する | 記録なし |
| `Paste` | 整数（貼り付けのオプション。下の「Paste のオプション」） | なし | クリップボードのテキストを、選択の代わりに貼り付ける。行のコピーは、行頭に入り、キャレットは元の文字の上に残る。矩形のコピーは、矩形として貼る。矩形選択があるときは、テキストの 1 行目を、矩形の各行に入れる。クリップボードにテキストがなければ、ビープ音を鳴らす。 | する | 記録なし |
| `CopyAddCRLF` | なし | なし | `Copy` と同じ。折り返しで切れた各行の末尾（と最終行の末尾）に、文書の改行を付けてコピーする。矩形のコピーは変わらない。 | する | 記録なし |
| `CopyCRLF` | なし | なし | `Copy` と同じ。コピーする文の改行を、すべて CRLF にする。 | する | 記録なし |
| `CopyLF` | なし | なし | `Copy` と同じ。コピーする文の改行を、すべて LF にする。 | する | MATSU Editor だけの関数。サクラにはない。矩形のコピーは、各行の後ろに LF を付ける。 |
| `CopyCR` | なし | なし | `Copy` と同じ。コピーする文の改行を、すべて CR にする。 | する | MATSU Editor だけの関数。サクラにはない。矩形のコピーは、各行の後ろに CR を付ける。 |
| `PasteBox` | 整数（読まない） | なし | クリップボードのテキストを、矩形として貼る。テキストの各行は、キャレットの表示桁から、次の行へ入る。短い行は、空白で埋める。矩形でないコピーも、矩形として貼る。クリップボードにテキストがなければ、ビープ音を鳴らす。 | する | 引数は読まない。 |
| `InsBoxText` | 文字列（貼るテキスト） | なし | 引数のテキストを、`PasteBox` と同じやり方で、矩形として入れる。テキストが 1 つ必要。ないとエラーになる。 | しない | 記録なし |
| `InsText` | 文字列（入れるテキスト） | なし | テキストを、選択の代わりに入れて、キャレットを入れた文の後ろに置く。矩形選択があるときは、テキストの 1 行目を、矩形の各行に入れる。改行は変換しない。テキストがないとエラーになる。 | する | 記録なし |
| `AddTail` | 文字列（足すテキスト） | なし | 文書の末尾へ行って（選択は解除し、置き換えない）、テキストを足す。キャレットは足した文の後ろに置く。テキストがないとエラーになる。 | しない | 記録なし |
| `CopyLines` | なし | なし | 選択を表示行の単位に広げて、コピーする。 | する | 記録なし |
| `CopyLinesAsPassage` | なし | なし | `CopyLines` と同じ。各行の先頭に引用符を付ける。引用符は、既定で `> `（`SetMsgQuoteStr` で変える）。 | する | 記録なし |
| `CopyLinesWithLineNumber` | なし | なし | `CopyLines` と同じ。各行の先頭に行番号（` 12:` の形）を付ける。 | する | 行番号は、文書の行（改行から改行まで）の番号を付ける。サクラは折り返した表示行に番号を付ける。折り返さない文書では同じ。 |
| `CopyColorHtml` | なし | なし | 選択を、文書タイプの色と書体を付けた HTML（`<pre>`）でコピーする。クリップボードには、HTML のソースの文字列と、OS の HTML 形式の両方を入れる。 | する | 記録なし |
| `CopyColorHtmlWithLineNumber` | なし | なし | `CopyColorHtml` と同じ。各行の先頭に行番号を付ける。 | する | 記録なし |
| `CopyPath` | なし | なし | 文書のファイルのフルパスをコピーする。ファイルのない文書では、ビープ音を鳴らすだけ。 | する | 記録なし |
| `CopyDirPath` | なし | なし | 文書のあるフォルダーのパスをコピーする。末尾の `\` は付けない（ドライブの直下は `C:`）。ファイルのない文書では、ビープ音を鳴らすだけ。 | する | 記録なし |
| `CopyFilename` | なし | なし | 文書のファイル名をコピーする。ファイルのない文書では、ビープ音を鳴らすだけ。 | する | 記録なし |
| `CopyTag` | なし | なし | `パス (行,桁): ` の形で、ファイルのパスとキャレットの位置をコピーする。行と桁は 1 から数える。桁は UTF-16 の単位で数える。ファイルのない文書では、ビープ音を鳴らすだけ。 | する | 記録なし |
| `CopyKeyBindList` | なし | なし | キー割り当ての一覧を、テキストにしてコピーする。関数の名前は、共通設定の画面に出る名前と同じ。 | する | 記録なし |

## Paste のオプション

`Paste` の引数は、次の値を足して作ります。指定しない組は、設定に従います。

| 値 | 意味 |
|---|---|
| 1 | 改行を、文書の改行に変換する |
| 2 | 改行を変換しない |
| 4 | 行のコピーを、行頭に貼る |
| 8 | 行のコピーを、キャレットの位置に貼る |
| 16（0x10） | 矩形のコピーを、矩形として貼る |
| 32（0x20） | 矩形のコピーを、ふつうの文として貼る |

設定の既定は、行のコピーは行頭に貼る（オン）、改行は変換しない（オフ）です。矩形のコピーを矩形として貼ることは、設定がなく、常にオンです。

## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`Cut` から `CopyKeyBindList`）
- サクラ:
  - `sakura_core/cmd/CViewCommander.cpp`（`HandleCommand` の `F_CUT` から `F_COPY_COLOR_HTML_LINENUMBER`、`F_CREATEKEYBINDLIST`）
  - `sakura_core/cmd/CViewCommander_Clipboard.cpp`（`Command_COPY`、`Command_PASTE`、`Command_PASTEBOX`、`Command_INSBOXTEXT`、`Command_INSTEXT`、`Command_ADDTAIL`、`Command_COPYLINES*`、`Command_COPYTAG` ほか）
- MATSU のコード（`crates/matsu-commands/src/commands/clipboard/`）: `cut.rs`、`copy.rs`、`paste.rs`、`copy_addcrlf.rs`、`copy_crlf.rs`、`copy_lf.rs`、`copy_cr.rs`、`pastebox.rs`、`insboxtext.rs`、`ins_text.rs`、`add_tail.rs`、`copy_lines*.rs`、`copy_path.rs`、`copydirpath.rs`、`copy_file_name.rs`、`copy_tag.rs`
- MATSU のコード（`crates/matsu-commands/src/context/clipboard.rs`）: `paste`、`paste_option`
- MATSU のコード（`crates/matsu-ui/src/`）: `ui_commands/clipboard/copy_color_html.rs`、`copy_color_html_linenumber.rs`、`createkeybindlist.rs`、`copy_html/mod.rs`
- 試験: `crates/matsu-commands/src/commands/clipboard/tests.rs`、`copy_eol_tests.rs`、`ins_text.rs` の `tests`
