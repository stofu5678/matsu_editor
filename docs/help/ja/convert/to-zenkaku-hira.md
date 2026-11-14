# 半角＋全カタ→全角・ひらがな

全角のひらがなにできる文字を、すべて全角のひらがな（と全角の英数字）にします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「半角＋全カタ→全角・ひらがな」 |
| 機能名 | `F_TOZENKAKUHIRA` |
| マクロ | `ToZenHira` |
| 既定のキー | `Ctrl+F10` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

次の順に変えます。

1. 半角のカタカナを、全角のカタカナにします
2. 全角のカタカナを、ひらがなにします（`ァ`〜`ヴ`、`ヽ`、`ヾ`）。`ヵ` と `ヶ` は、そのままです
3. 半角の英数字と記号を、全角にします。空白は全角空白になります（[記号の対応](to-hankaku-eisu.md)）

## 例

```text
ｶﾞ ヒ a   ->   が　ひ　ａ
```

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_TOZENKAKUHIRA`、`sakura_core/convert/CConvert_ToZenhira.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/to_zenkaku_hira.rs`、`crates/matsu-commands/src/convert/kana.rs`
- 試験: `commands/convert/to_zenkaku_hira.rs` の `converts_to_full_width_hiragana`
