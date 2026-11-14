# 半角英数→全角英数

半角の英数字と記号を、全角にします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「半角英数→全角英数」 |
| 機能名 | `F_TOZENEI` |
| マクロ | `ToZenEi` |
| 既定のキー | なし |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

半角の英字、数字、記号を、全角にします。空白は全角空白になります。半角のカタカナは、そのままです。文字数は変わりません。


## 記号の対応

英数字の変換で、記号は次の組として扱われます。半角の記号と全角の記号を、同じ順に並べています。

```text
半角:  (空白) , . + - * / % = | & ^ \ @ ; : " ` ' < > ( ) { } [ ] ! ? # $ ~ _
全角:  (全角空白) ， ． ＋ － ＊ ／ ％ ＝ ｜ ＆ ＾ ￥ ＠ ； ： ” ‘ ’ ＜ ＞ （ ） ｛ ｝ ［ ］ ！ ？ ＃ ＄ ￣ ＿
```

表にない記号（`〜` など）は、変わりません。

## 例

```text
Az0 !ｱ   ->   Ａｚ０　！ｱ
```

## サクラエディタとの違い

- 動きの違いは、共通の動きの節に書いたものだけです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_TOZENEI`、`sakura_core/convert/CConvert_HaneisuToZeneisu.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/to_zenkaku_eisu.rs`、`crates/matsu-commands/src/convert/kana.rs`
- 試験: `commands/convert/to_zenkaku_eisu.rs` の `converts_alphanumerics_and_symbols`
