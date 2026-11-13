# 指定行へジャンプ

行番号を入力して、その行へ移動します。

- メニュー: 検索 → 指定行へジャンプ...
- 既定のキー: `Ctrl+J`
- マクロ: `Jump(行番号, オプション)`（機能名 `F_JUMP`）。パネルを開くのは機能名 `F_JUMP_DIALOG`

## パネル

パネルには「行番号」の入力欄と「OK」のボタンがあります。数字でない入力で OK を押しても、何も起きません。移動したらパネルは閉じます（`Esc` でも閉じます）。

パネルでは、行番号は改行で区切られた行を数えます。

## 動き

- キャレットは、その行の先頭に動きます。
- 最終行より大きい番号は最終行になります。パネルでは 0 は先頭の行になり、負の数は受け付けません（何も起きず、パネルも閉じません）。マクロの `Jump` は、1 より小さい番号（負の数も）を先頭の行にします。
- 移動の前の位置は、移動の履歴に残ります。
- 選択のロック中は、選択を伸ばしながら動き、そうでなければ選択を外します。
- 行数をまだ調べている途中の巨大なファイルでは、調べ終わるのを待つか、ファイルの該当の部分から表示します。

## マクロのオプション

`Jump` のオプションは、次の値の合計です。省略すると 1 です。

| 値 | 意味 |
|---|---|
| 0x01 | 行番号は、改行で区切られた行を数える。付けなければ、折り返した表示行を数える |
| 0x02 | PL/SQL: 先頭から、ソースの有効な行だけを数える（空行は数えない） |

## サクラエディタとの違い

- パネルには、「折り返し単位」の選択と、PL/SQL の数え方の選択がありません。パネルは、いつも改行の単位です。この 2 つは、マクロの `Jump` のオプションで使えます。PL/SQL の数え始めの行は、マクロでは先頭の行に固定です。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Bookmark.cpp` の `Command_JUMP`、`Command_JUMP_DIALOG`、`sakura_core/macro/CMacro.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/jump_dialog.rs`、`crates/matsu-commands/src/commands/cursor/jump.rs`（ファイル先頭の説明）、`crates/matsu-ui/src/search/panel.rs`（`jump`）、`crates/matsu-ui/src/search/ops.rs`（`go_to_line`）
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`J`）
