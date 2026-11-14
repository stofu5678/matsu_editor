# uudecodeして保存

選んでいる文字列を uuencode として読み、できたバイト列をファイルに保存します。文書は変わりません。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「文字コード変換」の「uudecodeして保存」 |
| 機能名 | `F_UUDECODE` |
| マクロ | `Uudecode` |
| 既定のキー | `Alt+F5` |
| キーマクロ | 記録されない |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[文字コード変換の考え方](code-basics.md)を見てください。

## 動き

1. 選んでいる文字列を、uuencode として読みます
2. 保存のダイアログが開きます。ファイル名の候補は、`begin` の行にある名前です
3. 決めた場所に、バイト列をそのまま書きます

文書、文書の名前、変更済みの印は、変わりません。ダイアログを取り消すと、何も書きません。書き込みに失敗すると、エラーを出します。

読める形は、次のとおりです。

- 最初が `begin` の行（`begin`、8 進数 3 桁、ファイル名。区切りは空白か TAB）。ファイル名は 3 つ目の語だけで、空白を含む名前は最初の空白で切れます
- 本文の行。どの行も、CR LF で終わります
- 1 文字だけ（空白、`` ` ``、`~`）の行
- `end` の行

この形でないときは、読めません。何も起きません（音も、メッセージもありません）。何も選んでいないときも、何も起きません。箱型の選択のときは、まだ何もしません。

## サクラエディタとの違い

- 行の長さの文字が示すより短い本文の行（末尾の空白を落とすゲートウェイがある）は、空白で補って読みます。サクラは、その行のうしろの文字まで読みます。
- 箱型の選択では、まだ何もしません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_UUDECODE`、`sakura_core/convert/CDecode_UuDecode.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/uudecode.rs`、`crates/matsu-ui/src/file_ops/save_decoded.rs`、`crates/matsu-commands/src/convert/code.rs` の `uudecode`
- 試験: `crates/matsu-commands/src/convert/code.rs` の `uudecode_reads_the_header_the_body_and_the_footer`、`uudecode_rejects_what_is_not_uu`、`uudecode_pads_a_short_line`
