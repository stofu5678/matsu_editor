# TAB→空白

選んでいる文字列の中の TAB を、空白にします。

## 呼び出し方

| 項目 | 内容 |
|---|---|
| メニュー | 「変換」の「TAB→空白」 |
| 機能名 | `F_TABTOSPACE` |
| マクロ | `TABToSPACE` |
| 既定のキー | `Ctrl+Alt+F5` |
| キーマクロ | 記録される |

選んでいる文字列が対象です。何も選んでいないときは、メニューの項目が灰色になり、実行しても何も起きません。そのほかの共通の動き（選択の残り方、箱型の選択、閲覧モード、元に戻す）は、[共通の動き](index.md#共通の動き)を見てください。

## 動き

TAB を、次の TAB 位置までの空白にします。

- TAB 位置の間隔は、画面の桁の組み方（レイアウト）の TAB 幅です
- 数え始める桁は、選択の始まりの桁です。2 行目からは 0 桁目から数えます
- 全角の文字（日本語など）は 2 桁、そのほかは 1 桁と数えます
- 改行（CR、LF、CRLF）はそのまま残ります

## 例（TAB 幅が 4 のとき）

```text
a<TAB>b          ->   a   b     (空白 3 つ)
ab<TAB>c         ->   ab  c     (空白 2 つ)
```

## サクラエディタとの違い

- 全角の文字の幅は、フォントではなく文字の種類から決めた近似です。サクラはフォントに聞くので、まれに桁が合わない文字があります（`crates/matsu-commands/src/convert/tabs.rs` の `char_width`）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp` の `CViewCommander::Command_TABTOSPACE`、`sakura_core/convert/CConvert_TabToSpace.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/convert/tab_to_space.rs`、`crates/matsu-commands/src/convert/tabs.rs`
- 試験: `commands/convert/tab_to_space.rs` の `replaces_tabs_by_the_spaces_to_the_next_tab_stop`、`the_tab_width_comes_from_the_layout`
