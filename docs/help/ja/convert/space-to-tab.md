# 空白→TAB

選んでいる文字列の中の空白の並びを、TAB に置きます。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「空白→TAB」 |
| 機能名 | `F_SPACETOTAB` |
| マクロ | `SPACEToTAB` |
| 既定のキー | `Ctrl+Alt+Shift+F5` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

空白と TAB の並びを、TAB 位置に届く分は TAB に、残りは空白にします。

- TAB 位置の間隔と、数え始める桁は、[TAB→空白](tab-to-space.md)と同じです
- 文字と文字の間にある、1 桁だけの空白は、空白のままです
- TAB 位置に届かない空白は、そのまま残ります
- 改行はそのまま残ります

## 例（TAB 幅が 4 のとき）

```text
(空白 4 つ)x       ->   <TAB>x
a b(空白 2 つ)c    ->   a b<TAB> c
```

## サクラエディタとの違い

- TAB 幅の違いなど、全角の文字の幅の近似は、[TAB→空白](tab-to-space.md)と同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_SPACETOTAB`、`sakura_core/convert/CConvert_SpaceToTab.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/space_to_tab.rs`、`crates/matsu-commands/src/convert/tabs.rs`
- 試験: `commands/convert/space_to_tab.rs` の `replaces_runs_of_spaces_by_tabs`、`counts_from_the_column_of_the_selection`
