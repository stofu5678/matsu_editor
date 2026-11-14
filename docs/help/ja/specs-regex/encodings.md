# 文字コード

MATSU Editor が読み書きできる文字コードと、自動判定、BOM、読めないバイトの扱いです。

## 読み書きできる文字コード

サクラエディタの 9 種類に、UTF-32 と、Windows のコードページの一部を加えています。

| 文字コード | サクラの番号 | BOM | 既定で BOM あり | 既定の文字コードに選べる |
|---|---|---|---|---|
| Shift_JIS（CP932） | 0 | なし | － | はい |
| JIS（ISO-2022-JP） | 1 | なし | － | いいえ |
| EUC-JP | 2 | なし | － | はい |
| UTF-16LE | 3 | あり | はい | はい |
| UTF-8 | 4 | あり | いいえ | はい |
| UTF-7 | 5 | あり（`+/v8-`） | いいえ | いいえ |
| UTF-16BE | 6 | あり | はい | はい |
| CESU-8 | 7 | あり | いいえ | はい |
| Latin1（Windows-1252） | 8 | なし | － | はい |
| UTF-32LE、UTF-32BE | 12000、12001 | あり | いいえ | はい |
| コードページ（936、949、950、1251 など） | その番号 | なし | － | はい |

- 「番号」は、`matsu.ini` の `eDefaultCodetype` などに書く値で、サクラと同じです。
- コードページは、`encoding_rs`（WHATWG の Encoding Standard）の表で読み書きします。874、1250〜1258（1252 は Latin1）、866、20866、21866、28592〜28599 付近の ISO-8859 系、936（GBK）、949、950、51949、54936（GB18030）、10000 などです。Windows のコードページと、いくつかのバイトで結果が違います（`crates/matsu-core/src/encoding/codepage.rs` の先頭に一覧があります）。437 や 850 のような、表のないものは使えません。システムのコードページ（`CODE_CPACP`、`CODE_CPOEM`）も、ありません。
- 変換は、どの OS でも同じ結果になります。Windows の API は使いません。

## 自動判定

ファイルを開くとき、文字コードを指定しないと、先頭の 32768 バイトを調べて、次の順に決めます。

1. BOM（UTF-8、UTF-16BE、UTF-16LE。UTF-7 の BOM は使いません）
2. 文字の中の宣言: XML の `encoding="..."`、先頭 2 行のコメントの `coding: ...`、`<meta charset=...>`
3. BOM のない UTF-16: 片方のバイト順だけに改行があり、不正な単位がなく、1 行の平均が 200 バイト以下のとき
4. 点数: Shift_JIS、ISO-2022-JP、EUC-JP、UTF-8、UTF-7、CESU-8、Latin1 のうち、その文字コードだけが使うバイトの数から、不正なバイトを引いた点が、いちばん多いもの。不正なバイトが標本の 0.05% 未満で、点があるときに限る
5. 決まらないとき（空のファイル、ASCII だけ、不正が多い）は、既定の文字コード

- UTF-8 と CESU-8 が同点のときは、UTF-8 にします。「CESU-8 を判別する」をオンにすると、CESU-8 にします。
- 宣言が Windows のコードページを指していて、MATSU Editor が持っているものなら、その文字コードで読みます。表のないコードページが宣言されているときは、既定の文字コードで読みます。
- サクラが、あれば先に使う `uchardet` の DLL は、MATSU Editor にはありません。

## 既定の文字コード、BOM、改行コード

新しい文書と、改行や文字コードが決まらないファイルの既定値は、タイプ別設定の「ウィンドウ」ページの「デフォルトの文字コード」（`eDefaultCodetype`、`bDefaultBom`、`eDefaultEoltype`）で決めます。設定のないときは、UTF-8、BOM なし、OS の改行コードです。

## 読めないバイトと書けない文字

- その文字コードで不正なバイトは、1 バイトごとに、私用領域の代用文字（U+10FF00〜U+10FFFF）として持ち、保存では元のバイトに戻します。サクラは U+DC00〜U+DCFF を使います。MATSU Editor の文字列は、その範囲を持てないためです。ファイルに、この範囲の本物の文字があると、保存で 1 バイトに戻ります（とてもまれです）。
- UTF-16 の、対にならないサロゲートは、代用文字 2 つになります。バイト数が奇数のときは、最後の 1 バイトが代用文字 1 つです。
- 保存する前に、その文字コードで書けない文字を調べます。見つかると、「保存する・しない・取り消し」を聞きます。保存すると、書けない文字は `?` になります。
- Shift_JIS では、Windows が黙って近い文字に置き換える文字（U+2212 の代わりの 0x817C など）を、MATSU Editor は書けない文字として `?` にします。OS によって結果が変わらないようにするためです。

## BOM

- BOM は、読み込むとき、文字列から外して覚えます。保存では、BOM を持てる文字コードで、BOM ありの指定のときだけ書きます。
- JIS と UTF-7 のように、前のバイトで意味が変わる文字コードは、ファイル全体を読んで変換します。範囲ごとの読み込みはできません。

## サクラエディタとの違い

- 代用文字の範囲、Windows の最適近似をしないこと、UTF-7 と JIS の壊れたデータの細かい扱い、`uchardet` がないこと、コードページが `encoding_rs` の範囲に限られることが違います。理由は、`crates/matsu-core/src/encoding/mod.rs` の「Differences from Sakura」に 1 つずつ書いてあります。

## 根拠

- サクラのソース: `sakura_core/charset/charset.h`、`charset.cpp`、`CCodeBase.cpp`、`CESI.cpp`、`CCodeMediator.cpp`、`sakura_core/io/CFileLoad.cpp`
- MATSU のコード: `crates/matsu-core/src/encoding/`（`mod.rs`、`detect.rs`、`codepage.rs`）、`crates/matsu-core/src/file/mod.rs`、`crates/matsu-ui/src/settings/new_document.rs`、`crates/matsu-ui/src/type_settings/pages/window.rs`
- 試験: `crates/matsu-core/src/encoding/tests.rs`、`codepage_tests.rs`
