# 大文字

選んでいる文字列の小文字を、大文字にします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「大文字」 |
| 機能名 | `F_TOUPPER` |
| マクロ | `ToUpper` |
| 既定のキー | `Ctrl+F7`、`Ctrl+Alt+Shift+L` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

[小文字](to-lower.md)と同じ範囲の小文字を、対応する大文字にします。範囲の外の文字と、文字数は変わりません。

- `ß` は、そのままです。
- `ÿ` は `Ÿ` になります。
- ドットのない `ı` は、`I` になります。
- 標準の対応表は使いません。

## 例

```text
abc XYZ ａｂｃ ０   ->   ABC XYZ ＡＢＣ ０
```

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_TOUPPER`、`sakura_core/convert/CConvert_ToUpper.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/to_upper.rs`、`crates/matsu-commands/src/convert/case.rs`
- 試験: `crates/matsu-commands/src/convert/case.rs` の `latin_1_supplement_skips_the_multiplication_and_division_signs`、`commands/convert/to_upper.rs` の `upper_cases_the_selection`
