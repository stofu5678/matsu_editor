# Base64デコードして保存

選んでいる文字列を Base64 として読み、できたバイト列をファイルに保存します。文書は変わりません。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「文字コード変換」の「Base64デコードして保存」 |
| 機能名 | `F_BASE64DECODE` |
| マクロ | `Base64Decode` |
| 既定のキー | `Alt+F6` |
| キーマクロ | 記録されない |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[文字コード変換の考え方](code-basics.md)を見てください。

## 動き

1. 選んでいる文字列を、Base64 として読みます
2. 保存のダイアログが開きます。ファイル名の候補は、ありません
3. 決めた場所に、バイト列をそのまま書きます

文書、文書の名前、変更済みの印は、変わりません。ダイアログを取り消すと、何も書きません。書き込みに失敗すると、エラーを出します。

読み方の決まりは、次のとおりです。

- 改行、空白、TAB は、読み飛ばします
- 最初の `=` で、データは終わりです
- Base64 の文字でないものがあると、読めません。何も起きません（音も、メッセージもありません）

何も選んでいないときも、何も起きません。箱型の選択のときも、まだ何もしません。

文字列を Base64 にしたり、Base64 から文字列に戻したりするときは、「MATSU Editor だけの機能」の章の「エンコードとデコード」を見てください。

## サクラエディタとの違い

- 箱型の選択では、まだ何もしません（サクラとの違い）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_BASE64DECODE`、`sakura_core/convert/CDecode_Base64Decode.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/base64_decode.rs`、`crates/matsu-ui/src/file_ops/save_decoded.rs`、`crates/matsu-commands/src/convert/code.rs` の `base64_decode`
- 試験: `crates/matsu-commands/src/convert/code.rs` の `base64_skips_white_space_and_stops_at_the_pad`
