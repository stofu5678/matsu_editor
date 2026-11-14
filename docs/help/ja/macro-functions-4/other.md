# マクロの関数（4）: その他

版の文字列の比較と、拡張子から文書タイプを調べる関数の表です。共通の決まりは[この章の最初](index.md)にあります。

- 「キーマクロに記録」は、どの行も「しない」です。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `CompareVersion` | 1 版 A（文字列）、2 版 B（文字列） | 数 | 2 つの版の文字列を比べる。等しければ 0、A が新しければ正の数、B が新しければ負の数 | しない | 違いなし。使うのは符号だけ（数の大きさに意味はない） |
| `IsCurTypeExt` | 1 拡張子（文字列） | 数 | 文書のタイプが、その拡張子のタイプと同じなら 1、違えば 0 | しない | 設定ファイルがないときは、組み込みのタイプの拡張子の一覧で調べる（`IsSameTypeExt("cpp", "txt")` は 0） |
| `IsSameTypeExt` | 1 拡張子 1（文字列）、2 拡張子 2（文字列） | 数 | 2 つの拡張子が、同じ文書タイプに入っていれば 1、違えば 0 | しない | 同上 |

## 版の比べ方

`CompareVersion` は、版の文字列を、4 つの部分に分けて比べます。

- 部分は、数字が 2 桁までの並びです。3 桁目の数字は、次の部分になります。
- 区切りは、`.`、`-`、`_`、`+` です。区切りと、数字のない文字は、読み飛ばします。
- 省いた部分は、0 として比べます。4 つより後ろは、読みません。
- 部分の前の言葉が、その部分を、正式な版より前や後ろに置きます（`1.0a` < `1.0` < `1.0pl`）。

| 部分の前の言葉 | 置き方 |
|---|---|
| 数字以外の文字で始まるもの（下のどれでもないとき） | いちばん前 |
| `a`、`alpha`（小文字だけ） | 正式な版より前。上の次 |
| `b`、`beta`（小文字だけ） | `alpha` の次 |
| `r`、`R`、`rc`（`rc` の大文字小文字は区別しない） | `beta` の次 |
| 言葉なし | 正式な版 |
| `p`、`pl`（小文字だけ） | 正式な版より後ろ |

- 考え方は、PHP の `version_compare` と同じです。

## 文書タイプの調べ方

- 拡張子は、ファイル名として、各文書タイプの「拡張子」の欄から探します。
- 設定ファイル（`matsu.ini`）にタイプの設定がないときは、組み込みのタイプの拡張子の一覧で調べます。

## 根拠

- サクラのソース: `sakura_core/util/format.cpp` の `ParseVersion`、`CompareVersion`、`sakura_core/macro/CMacro.cpp` の `F_COMPAREVERSION`、`F_ISCURTYPEEXT`、`F_ISSAMETYPEEXT`、`CDocTypeManager::GetDocumentTypeOfExt`
- MATSU のコード: `crates/matsu-plugin/src/functions/misc.rs`（`parse_version`、`compare_version`、`is_cur_type_ext`、`is_same_type_ext`）、`crates/matsu-ui/src/macros/host.rs`（`type_of_ext`）
- 試験: `crates/matsu-plugin/src/functions/misc.rs` の `versions_compare_part_by_part`、`special_words_put_a_part_before_or_after_the_release`、`separators_and_text_are_skipped`、`types_of_extensions_come_from_the_window`
