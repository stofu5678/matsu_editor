# 対括弧の検索

キャレットのある括弧と対になる括弧へ移動します。

- メニュー: 検索 → 対括弧の検索
- 既定のキー: `Ctrl+[`、`Ctrl+]`
- マクロ: `BracketPair`（機能名 `F_BRACKETPAIR`）

## 動き

- キャレットの位置の文字が括弧なら、それを使います。そうでなければ、キャレットの 1 つ前の文字が括弧のときにそれを使います。括弧がなければ、何も起きません。
- 開き括弧からは文書の終わりへ、閉じ括弧からは文書の先頭へ探します。同じ種類の括弧だけを数え、入れ子を考えます。
- キャレットは、見つけた括弧の前（文字の左）に置かれます。
- 文字列やコメントの中の括弧も、ふつうの括弧として数えます。
- 選択のロック中は、選択を伸ばしながら動きます。

## 対応する括弧

`( )`、`[ ]`、`{ }`、`< >`、半角の `｢ ｣`、`【 】`、`『 』`、`「 」`、`＜ ＞`、`≪ ≫`、`《 》`、`（ ）`、`〈 〉`、`｛ ｝`、`〔 〕`、`［ ］`、`“ ”`、`〝 〟`

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Search.cpp` の `Command_BRACKETPAIR`、`sakura_core/view/CEditView_Paint_Bracket.cpp` の `SearchBracket` と `g_aKakkos`
- MATSU のコード: `crates/matsu-commands/src/commands/cursor/bracket_pair.rs`、`crates/matsu-commands/src/text/bracket.rs`（表）
- 試験: `crates/matsu-commands/src/commands/cursor/bracket_pair.rs`（`from_an_opening_bracket_to_the_closing_one`）
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`[`、`]`）
