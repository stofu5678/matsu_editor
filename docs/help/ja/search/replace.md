# 置換...

探す文字列と置き換える文字列を入力するパネルを開きます。

- メニュー: 検索 → 置換...
- 既定のキー: `Ctrl+R`
- マクロ: `ReplaceDialog`（機能名 `F_REPLACE_DIALOG`）。パネルを使わずに置き換えるマクロは `Replace(置換前, 置換後, フラグ)` と `ReplaceAll(置換前, 置換後, フラグ)`

## パネル

パネルは [検索...](search.md) と同じ選択肢（単語単位、大文字と小文字の区別、正規表現、見つからないときのメッセージ、先頭（末尾）から再検索）に、次を足したものです。入力欄の初期値の決まり方も同じです。

| 項目 | 意味 |
|---|---|
| 置換前 / 置換後 | 探す文字列と、置き換える文字列。それぞれ履歴がある |
| 置換対象: 選択文字 | 一致を置換後の文字列に替える |
| 置換対象: 選択始点に挿入 | 一致は残し、その前に置換後の文字列を入れる |
| 置換対象: 選択終点に追加 | 一致は残し、その後ろに置換後の文字列を入れる |
| 置換対象: 行削除 | 一致を含む行を丸ごと消す。置換後の文字列は使わない |
| 範囲: 選択範囲 / ファイル全体 | すべて置換を、選択の中だけにするか、文書全体にするか |
| 「すべて置換」は置換の繰返し | 置換した結果にもう一度置換をかける。「正規表現」が入のときだけ選べる |

## ボタン

| ボタン | 動き |
|---|---|
| 上検索 / 下検索 | 置換せずに、前（後ろ）の一致を探して選ぶ |
| 置換 | 選択中（またはキャレット位置）の一致を置換し、次の一致を選ぶ。次の一致がないときは、「最後まで置換しました。」と箱で知らせる |
| すべて置換 | 範囲の中の一致をすべて置換する。全体が元に戻す 1 回分（Undo の 1 段）になる。終わったら「N箇所を置換しました。」と箱で知らせる |
| 該当行マーク | 一致した行にブックマークを付ける |
| 閉じる | パネルを閉じる |

置換前の文字列が空のときは、「置換条件を指定してください。」と箱で知らせ、何もしません。置換パネルは、置換のあとも閉じません。

正規表現で置換するときは、置換後の文字列の中で、一致した部分の参照（`$1` など）が使えます。書き方は「仕様等と正規表現」の章で説明します。置換後の文字列が使えない形（末尾の `\`）のときは、正しくない正規表現と同じように箱で知らせます。

## マクロのフラグ

`Replace` と `ReplaceAll` のフラグは、[次を検索・前を検索](search-next-prev.md) の表に次の値を足したものです。

| 値 | 意味 |
|---|---|
| 0x040 | （置換）クリップボードの内容で置換する |
| 0x080 | （すべて置換）選択範囲の中だけ |
| 0x100 / 0x200 | （置換）一致の前 / 後ろに挿入する |
| 0x400 | （すべて置換）置換を繰り返す |

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Search.cpp` の `Command_REPLACE_DIALOG`、`Command_REPLACE`、`Command_REPLACE_ALL`、`sakura_core/dlg/CDlgReplace.cpp`、`sakura_core/macro/CMacro.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/replace_dialog.rs`、`replace.rs`、`replace_all.rs`、`crates/matsu-ui/src/search/panel.rs`（項目の名前とボタン）、`crates/matsu-ui/src/search/ops.rs`（`replace_next`、`replace_all`）、`crates/matsu-ui/src/search/state.rs`（`ReplaceSettings`、メッセージ）、`crates/matsu-core/src/search/replace.rs`（`ReplaceTarget`）、`crates/matsu-ui/src/search/macro_args.rs`
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`R`）
