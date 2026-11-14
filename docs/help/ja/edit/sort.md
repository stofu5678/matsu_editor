# 選択行のソート

選んだ行を、昇順または降順に並べ替えます。「編集」→「整形」にあります。

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_SORT_ASC` | 選択行の昇順ソート | `Alt+A` | `SortAsc()` |
| `F_SORT_DESC` | 選択行の降順ソート | `Alt+D` | `SortDesc()` |

## 対象の行

- 選択の始まりの行から、終わりの行までです。ただし、終わりの行は、選択がその行の 1 桁目より後ろで終わり、かつ、その行に改行があるときだけ含みます。
- 改行のない最後の行は、並べ替えません。
- 選択がないとき、または、対象の行がないとき（改行のない最後の行の中だけを選んだとき）は、何も起きません。警告音も鳴りません。

## 並べ方

- 行は、改行も含めて比べます。UTF-16 の単位ごとに比べるので、コードポイントの順とは違うことがあります。ほかの行の先頭部分にあたる行（`ab` と `abc` の `ab`）は、先に来ます。
- 同じ行どうしの順は変わりません（安定なソートです）。
- 並べ替えのあと、対象の行が、先頭から次の行の先頭まで選ばれます。カーソルは、並べ替え前に最初の行にあったときは先頭に、そうでなければ終わりに行きます。
- 並びが変わらないときは、編集を行いません。

## 矩形選択のとき

矩形の上の行から下の行までを、矩形に含まれる桁の文字で並べ替えます。矩形より手前で終わる行は、キーが空として扱われます。同じキーの行の順は変わりません。

折り返し表示のときの、矩形ソートの桁の扱いには、サクラエディタとの違いが残っています（通常の使い方では関係しません）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit_advanced.cpp` の `Command_SORT`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/sort_asc.rs`、`sort_desc.rs`（先頭のコメント）、矩形は `CommandContext::sort_box_lines`
- 試験: `crates/matsu-commands/src/commands/edit/sort_asc.rs` の `sorts_the_selected_lines`、`the_last_line_without_a_line_ending_is_not_sorted`、`compares_by_utf16_code_unit`、`a_box_selection_sorts_the_lines_by_the_columns_of_the_box`
- 残り: `docs/sakura-gaps.md` の「折り返し表示のときの矩形ソートの桁」の行
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`A`、`D` の行）
