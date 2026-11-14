# 小文字

選んでいる文字列の大文字を、小文字にします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「小文字」 |
| 機能名 | `F_TOLOWER` |
| マクロ | `ToLower` |
| 既定のキー | `Ctrl+F6`、`Ctrl+Alt+L` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

次の範囲の大文字を、対応する小文字にします。

- ASCII の英字（`A`〜`Z`）と、全角の英字（`Ａ`〜`Ｚ`）
- ギリシャ文字とキリル文字
- ラテン文字の拡張（ラテン 1 補助の `À`〜`Þ`（`×` を除く）、拡張 A・拡張 B・拡張追加）

範囲の外の文字は、そのままです。範囲の外には、`ß` や、BMP の外の文字（デゼレト文字など）があります。変換で文字数は変わりません。

Unicode の大文字小文字の対応表は使いません。そのため、標準ライブラリや他のエディタと結果が違う文字があります。たとえばドット付きの `İ` は、`i` になります。

## 例

```text
ABC xyz ＡＢＣ ０   ->   abc xyz ａｂｃ ０
```

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_TOLOWER`、`sakura_core/convert/CConvert_ToLower.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/to_lower.rs`、`crates/matsu-commands/src/convert/case.rs`
- 試験: `crates/matsu-commands/src/convert/case.rs` の `ascii_and_full_width_latin`、`latin_extended_a_with_even_and_odd_pairs`、`commands/convert/to_lower.rs` の `lower_cases_the_selection`
