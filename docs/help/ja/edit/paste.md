# 貼り付け

クリップボードの文字を、選択範囲の代わりに（選択がなければカーソルの位置に）入れます。

## 呼び出し方

- メニュー: 「編集」→「貼り付け」
- キー: `Ctrl+V`、`F9`、`Shift+Ins`。macOS では `Cmd+V` も使えます
- マクロ: `Paste(option)`
- 機能名: `F_PASTE`

## 動き

- クリップボードに文字がないときは、警告音が鳴り、何も変わりません。
- 行コピー（[コピー](copy.md)や[切り取り](cut.md)で、選択なしに行を写したもの）は、カーソルの位置にかかわらず行頭に入ります。カーソルは元の文字の上に残ります。写した文字の末尾に改行がないときは、文書の改行コードの改行を足して入れます（文書にまだ改行がないときは、OS の改行コード。Windows は CRLF、ほかは LF）。
- 矩形選択でコピーした文字は、矩形として貼り付けます（[矩形貼り付け](paste-box.md)と同じ）。
- 矩形選択をしているときは、貼る文字の最初の行が、矩形の各行に入ります。
- 改行コードは、既定では変換しません。共通設定「編集」の「改行コードを変換して貼り付ける」を入れると、文書の改行コードに合わせます。

## マクロの引数 option

`option` は、次の値を足したものです。省略したときは、共通設定の指定に従います。

| 値 | 意味 |
|---|---|
| `0x01` | 改行コードを文書のものに変換する |
| `0x02` | 改行コードを変換しない |
| `0x04` | 行コピーを行頭に貼る |
| `0x08` | 行コピーをカーソルの位置に貼る |
| `0x10` | 矩形コピーを矩形として貼る |
| `0x20` | 矩形コピーを普通の文字として貼る |

## 関連

[矩形貼り付け](paste-box.md)、[コピー](copy.md)

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `Command_PASTE`、`Command_INSTEXT`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/paste.rs`、`crates/matsu-commands/src/context/clipboard.rs`（`paste_option` の値）
- 試験: `crates/matsu-commands/src/commands/clipboard/tests.rs` の `paste_does_not_convert_line_endings_by_default`、`paste_option_converts_the_line_endings_to_the_documents`、`paste_of_a_line_copy_goes_to_the_start_of_the_row_and_the_caret_stays`、`paste_option_can_turn_line_mode_off`、`paste_beeps_without_text_on_the_clipboard`
- 設定の項目名: `crates/matsu-ui/src/common_settings/pages/edit.rs`（`bConvertEOLPaste`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F9`、`Ins`、`V` の行）
