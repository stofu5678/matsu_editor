# 全角→半角

半角にできる文字を、すべて半角にします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「全角→半角」 |
| 機能名 | `F_TOHANKAKU` |
| マクロ | `ToHankaku` |
| 既定のキー | `Ctrl+F8` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

次の文字を半角にします。

- 全角の英数字と記号（組は、[全角英数→半角英数](to-hankaku-eisu.md)の「記号の対応」）
- ひらがなとカタカナ。ひらがなは、半角のカタカナになります
- かなの記号（`。` `、` `「` `」` `・` `ー` `゛` `゜`）

濁点と半濁点のある文字は、2 文字（文字と `ﾞ` か `ﾟ`）になります。そのため、文字数が増えることがあります。

次の文字は、半角の形がないので変わりません: `ゝ` `ゞ` `ゟ`、未割り当ての `U+3097` と `U+3098`。

## 例

```text
ＡＢ１　ガひ   ->   AB1 ｶﾞﾋ
```

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_TOHANKAKU`、`sakura_core/convert/CConvert_ToHankaku.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/to_hankaku.rs`、`crates/matsu-commands/src/convert/kana.rs`
- 試験: `commands/convert/to_hankaku.rs` の `converts_to_half_width_and_the_selection_follows_the_new_length`
