# 半角カタカナ→全角ひらがな

半角のカタカナを、全角のひらがなにします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「半角カタカナ→全角ひらがな」 |
| 機能名 | `F_HANKATATOZENHIRA` |
| マクロ | `HanKataToZenHira` |
| 既定のキー | `Ctrl+F12` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

- [半角カタカナ→全角カタカナ](hankata-to-zenkata.md)と同じ読み方で全角にして、そのあとひらがなにします
- もとから全角のカタカナは、カタカナのままです
- `ﾜ` と `ｦ` は、濁点が続いても組みません。濁点は別に `゛` になります。`ﾜﾞ` は `わ゛` です
- 全角の文字と英数字は、変わりません

## 例

```text
ｶﾞｱ ab カ   ->   があ ab カ
```

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_HANKATATOZENKAKUHIRA`、`sakura_core/convert/CConvert_HankataToZenhira.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/hankata_to_zenkaku_hira.rs`、`crates/matsu-commands/src/convert/kana.rs`
- 試験: `commands/convert/hankata_to_zenkaku_hira.rs` の `converts_to_hiragana`
