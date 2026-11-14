# マクロの関数（4）: クリップボード

クリップボードの文字列を読む、書く関数の表です。共通の決まりは[この章の最初](index.md)にあります。

- 「キーマクロに記録」は、どの行も「しない」です。
- MATSU Editor が扱うのは、クリップボードの文字列だけです。画像などの文字でない形式は、見えません。
- 切り取り・コピー・貼り付けのコマンドは、「マクロの関数（2）」にあります。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `GetClipboard` | 1 予約?（数） | 文字列 | クリップボードの文字列を、そのまま返す。文字列がなければ空の文字列 | しない | 引数は読まない。改行は、コピー元のままで、変えない |
| `SetClipboard` | 1 オプション?（数。省くと 0）、2 文字列?（省くと空） | 数 | 文字列をクリップボードに入れる。入れられたら 1、入れられなければ 0 | しない | オプションは下の表。矩形と行の印は、同時に付けられない |
| `IsIncludeClipboardFormat` | 1 形式（文字列） | 数 | その形式がクリップボードにあれば 1、なければ 0 | しない | 文字列の形式だけ分かる（下の「形式」） |
| `GetClipboardByFormat` | 1 形式（文字列）、2 モード（数）、3 終端のモード?（数） | 文字列 | 文字列の形式で、クリップボードの文字列を返す。改行は、文書の改行にそろえる | しない | 使えない形式とモードはエラー。終端のモードは読まない |
| `SetClipboardByFormat` | 1 文字列、2 形式（文字列）、3 モード（数）、4 終端のモード?（数） | 数 | 文字列を、文字列の形式でクリップボードに入れる。入れられたら 1 | しない | `GetClipboardByFormat` と同じ形式とモードだけ使える |

## SetClipboard のオプション

| ビット | 値 | 意味 |
|---|---|---|
| 0 | 1 | 矩形のコピーとして入れる。貼り付けると、矩形として入る |
| 1 | 2 | 行のコピーとして入れる。貼り付けると、キャレットの行の前に入る |

- 両方のビットが立っているときは、矩形のコピーになります。
- どちらも立っていなければ、ふつうの文字列です。

## 形式

形式は、Windows のクリップボード形式の名前か番号で指定します。名前の大文字と小文字は区別しません。

| 名前 | 番号 | 扱い |
|---|---|---|
| `CF_TEXT` | 1 | 文字列の形式 |
| `CF_OEMTEXT` | 7 | 文字列の形式 |
| `CF_UNICODETEXT` | 13 | 文字列の形式 |
| 上以外 | － | 使えない |

- `IsIncludeClipboardFormat` は、文字列の形式なら、クリップボードに文字列があるときに 1 を返します。使えない形式は、いつも 0 です。
- `GetClipboardByFormat` と `SetClipboardByFormat` は、使えない形式を渡すとエラーです（未対応のエラー）。

### モード

| モード | 意味 | 使えるか |
|---|---|---|
| -2 | ふつうの扱い | 使える |
| 3 | UTF-16（`CF_UNICODETEXT` の文字列そのまま） | 使える |
| それ以外 | バイナリや別の文字コード | エラー（未対応） |

### 改行

- `GetClipboardByFormat` は、クリップボードの CR LF、CR、LF を、文書の改行（`GetLineCode` の返す種類）にそろえて返します。NEL、LS、PS は改行として扱わず、そのままです。
- `GetClipboard` は、改行をそろえません。

## 根拠

- サクラのソース: `sakura_core/macro/CMacro.cpp` の `CMacro::HandleFunction`（`F_GETCLIPBOARD`、`F_SETCLIPBOARD`、`F_ISINCLUDECLIPBOARDFORMAT`、`F_GETCLIPBOARDBYFORMAT`、`F_SETCLIPBOARDBYFORMAT`）、`sakura_core/_os/CClipboard.cpp`
- MATSU のコード: `crates/matsu-plugin/src/functions/clipboard.rs`
- 試験: `crates/matsu-plugin/src/functions/clipboard.rs` の `the_text_of_the_clipboard`、`set_clipboard_marks_boxes_and_rows`、`formats_are_text_or_nothing`、`by_format_converts_line_endings_to_the_document_s`、`by_format_refuses_what_the_clipboard_cannot_do`
