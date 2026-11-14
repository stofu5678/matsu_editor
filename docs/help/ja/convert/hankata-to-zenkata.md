# 半角カタカナ→全角カタカナ

半角のカタカナを、全角のカタカナにします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「半角カタカナ→全角カタカナ」 |
| 機能名 | `F_HANKATATOZENKATA` |
| マクロ | `HanKataToZenKata` |
| 既定のキー | `Ctrl+F11` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

- 半角のカタカナと、`ｰ`、`ﾞ` `ﾟ`、`｡` `､` `｢` `｣` `･` を、全角にします
- 文字のうしろに `ﾞ` か `ﾟ` が続き、まとめた形の文字があるときは、1 文字にします（`ｶﾞ` は `ガ`）。文字数が減ることがあります
- まとめられない濁点は、`゛` や `゜` になります
- 全角の文字と英数字は、変わりません

## 例

```text
ｶﾞｱ ab ひ   ->   ガア ab ひ
```

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_HANKATATOZENKAKUKATA`、`sakura_core/convert/CConvert_HankataToZenkata.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/hankata_to_zenkaku_kata.rs`、`crates/matsu-commands/src/convert/kana.rs`
- 試験: `commands/convert/hankata_to_zenkaku_kata.rs` の `joins_the_dakuten`
