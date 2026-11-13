# 改行コード

MATSU Editor が改行として読む文字と、書くときの決まりです。

## 改行の種類

| 種類 | 文字 | タイプ別設定の表示 |
|---|---|---|
| CR+LF | CR LF | CR+LF |
| LF | LF | LF (UNIX) |
| CR | CR | CR (Mac) |
| NEL | U+0085 | NEL |
| LS | U+2028 | LS |
| PS | U+2029 | PS |

## 読むとき

- 改行は、行ごとに持ちます。1 つのファイルに、種類の違う改行が混ざっていても、そのまま読んで、そのまま書き戻せます。
- CR の直後が LF のときは、CR LF の 1 つの改行です。
- どの種類を改行として読むかは、文字コードで違います。

| 文字コード | 改行として読むもの |
|---|---|
| UTF-8、CESU-8 | CR LF、LF、CR、NEL、LS、PS |
| UTF-16、UTF-32（LE と BE） | CR LF、LF、CR、NEL、LS、PS |
| Shift_JIS、EUC-JP、JIS、UTF-7、Latin1、コードページ | CR LF、LF、CR だけ |

- Shift_JIS などの 1 バイト系の文字コードでは、NEL、LS、PS を、改行として書けません（サクラは `?` にします）。UTF-7 では、これらは Base64 の中にあるので、改行として読みません。

## 新しい行の改行コード

- ファイルを開いたとき、最初の行に改行があれば、その種類が、そのあとに打つ改行の種類になります。
- 最初の行に改行がないとき（空のファイルなど）は、既定の文字コードで読んだなら、タイプ別設定の既定の改行コード、そうでなければ OS の改行コードです。
- 新しい文書の改行コードは、タイプ別設定の「ウィンドウ」ページの「デフォルトの文字コード」にある「改行コード」で決めます。設定のないときは、Windows が CR+LF、macOS と Linux が LF です。サクラは、いつも CR+LF です（ADR-0010 で、ユーザーが決めた違いです）。

## 保存するとき

- ふつうの保存は、改行を変えません。
- 「名前を付けて保存」では、すべての改行を 1 つの種類にそろえて書けます。文書そのものは変えずに、書くときに変えます。選んだ改行は、あとの保存でも保たれます。そろえたあとに打った改行や、貼り付けた改行も、書くときにそろいます。サクラは、文書を書き換えて、取り消しの記録に残します。
- 改行の種類が混ざったまま保存しようとすると、そろえるかどうかを聞くことがあります。タイプ別設定の「保存時に改行コードの混在を警告する」で切り替えます。

## 検索との関係

正規表現は、行の文字列のあとに、その行の改行を付けて照合します。`$` は、CR LF の前でも一致します（[検索の仕組みと上限](search-rules.md)）。

## 根拠

- サクラのソース: `sakura_core/basis/CEol.h`、`CEol.cpp`、`sakura_core/io/CFileLoad.cpp`、`sakura_core/doc/CDocEditor.cpp` の `OnAfterLoad`
- MATSU のコード: `crates/matsu-core/src/encoding/mod.rs`（`Eol`、`find_eol`）、`utf8.rs`、`utf16.rs`、`utf32.rs`、`utf7.rs`、`cp932.rs`、`crates/matsu-core/src/file/mod.rs`（先頭の「Line ending for new lines」「Line ending conversion」）、`crates/matsu-ui/src/settings/new_document.rs`、`crates/matsu-ui/src/type_settings/pages/window.rs`、`support.rs`
- 試験: `crates/matsu-core/src/encoding/tests.rs`、`crates/matsu-core/src/file/tests.rs`
- ADR: `docs/adr/0010-sakura-compatible-settings.md`
