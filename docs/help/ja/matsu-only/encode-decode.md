# エンコードとデコード

選んでいる文字列を、URL、HTML、Base64 の形に変えたり、元に戻したりします。6 つの機能があります。

## 機能の一覧

メニューは「変換」です。「空白→TAB」の下の、区切り線で分けた 1 つのまとまりにあります。右クリックのメニューにもあります。既定のキーは、どれもありません。

| メニュー | 機能名 | マクロ | 動き |
|---|---|---|---|
| URLエンコード | `F_URL_ENCODE` | `EncodeUrl()` | 文字列を、URL に書ける形（`%` と 16 進数）にします |
| URLデコード | `F_URL_DECODE` | `DecodeUrl()` | `%` と 16 進数を、元の文字に戻します |
| HTMLエスケープ | `F_HTML_ESCAPE` | `EscapeHtml()` | HTML で特別な意味を持つ文字を、文字参照にします |
| HTMLエスケープを戻す | `F_HTML_UNESCAPE` | `UnescapeHtml()` | 文字参照を、元の文字に戻します |
| Base64エンコード | `F_BASE64_ENCODE` | `EncodeBase64()` | 文字列を Base64 にします |
| Base64デコード | `F_BASE64_DECODE_TEXT` | `DecodeBase64()` | Base64 を、元の文字列に戻します |

どの機能も、キーボードマクロに記録されます。

## 共通の動き

- 選んでいる範囲の文字列を変えます。何も選んでいないときは、何もしません（音も鳴りません）。
- 変えたあと、変えた文字列を選んだままにします。
- 変えた結果が元と同じときは、文書を変えず、元に戻すの 1 手にもなりません。
- 変える文字列は、UTF-8 として扱います。
- 範囲の中の改行も、ふつうの文字として変えます。
- 箱型の選択のときは、行ごとに変えます。デコードできない行があっても、できる行は変わります。

## URL

| 機能 | 動き |
|---|---|
| URLエンコード | 英数字と `-` `.` `_` `~` 以外の文字を、UTF-8 のバイトごとに `%` と大文字の 16 進数 2 桁にします。空白は `%20` で、`+` にはしません |
| URLデコード | `%` と 16 進数 2 桁（大文字でも小文字でも）を、バイトにして、UTF-8 として読みます。`+` はそのままです |

例:

```text
a b/日本    ->   a%20b%2F%E6%97%A5%E6%9C%AC
```

URLデコードは、次のときは何も変えず、音を鳴らして知らせます。

- `%` の後ろに 16 進数が 2 桁ない（`100%` など）
- 結果が UTF-8 として読めない

## HTML

| 機能 | 動き |
|---|---|
| HTMLエスケープ | `&` `<` `>` `"` `'` を、`&amp;` `&lt;` `&gt;` `&quot;` `&#39;` にします。ほかの文字は、日本語も改行もそのままです |
| HTMLエスケープを戻す | `&amp;` `&lt;` `&gt;` `&quot;` `&apos;` と、数字の参照（`&#39;`、`&#x27;`）を戻します |

「HTMLエスケープを戻す」は、1 回の走査で戻します。`&amp;lt;` は `&lt;` になり、`<` にはなりません。知らない名前（`&nbsp;` など）、`;` のない参照、文字にならない数は、そのまま残します。このため、この機能が失敗して音が鳴ることはありません。

## Base64

| 機能 | 動き |
|---|---|
| Base64エンコード | UTF-8 のバイトを、標準の文字（`A-Z a-z 0-9 + /`）と、末尾の `=` で表します。1 行にまとめ、折り返しません |
| Base64デコード | 標準の Base64 を、UTF-8 の文字列に戻します |

Base64デコードは、改行、空白、タブを読み飛ばします（メールや PEM の折り返しのため）。末尾の `=` は、なくても読みます。

次のときは、何も変えず、音を鳴らして知らせます。

- 標準にない文字がある（URL 向けの `-` と `_` も、読めません）
- `=` が途中にある
- 長さが Base64 として成り立たない
- 結果が UTF-8 として読めない

サクラの「Base64デコードして保存」（`F_BASE64DECODE`）は、ファイルに書き出す別の機能です。そのまま残っています。MATSU Editor のマクロ名は、それと重ならないように `DecodeBase64` にしています。

## 右クリックのメニュー

保存してあった右クリックのメニューにも、この 6 つが足されます（メニューの版 2）。「すべて選択」の後ろに、区切りで分けた 1 つのまとまりとして入ります。アクセスキーは数字の 1 から 6 です。

## サクラとの違い

サクラエディタには、これらの機能がありません。MATSU Editor だけの機能です。

## 根拠

- 機能の定義: `xtask/src/matsu_extras.rs`、`crates/matsu-commands/src/sakura/main_menu.rs`、`crates/matsu-commands/src/sakura/macro_functions.rs`
- 変換の中身: `crates/matsu-commands/src/convert/text_encoding.rs`（`url_encode`、`url_decode`、`html_escape`、`html_unescape`、`base64_encode`、`base64_decode_text`）
- コマンド: `crates/matsu-commands/src/commands/convert/` の `url_encode.rs`、`url_decode.rs`、`html_escape.rs`、`html_unescape.rs`、`base64_encode.rs`、`base64_decode_text.rs`
- 選択範囲の扱い: `crates/matsu-commands/src/context/convert.rs` の `convert_selection` と `convert_selection_or_beep`
- 右クリックのメニュー: `crates/matsu-config/src/settings/custom_menu.rs` の `MATSU_ENCODINGS`
- 試験: 上の各コマンドのファイルの `tests`、`text_encoding.rs` の `tests`
- サクラのソース: `sakura_core/view/CEditView.cpp` の `ConvSelectedArea`（変換のあとに選択を保つ動きの元）
