# タグファイルの作成

ctags を動かして、フォルダーのタグファイル（`tags`）を作ります。

- メニュー: 検索 → タグファイルの作成...
- 既定のキー: なし
- マクロ: `TagMake`（機能名 `F_TAGS_MAKE`）。キーマクロの記録には入りません

## ctags について

MATSU Editor は ctags を同梱しません（ctags は GPL のため）。利用者が入れた ctags を動かします。ctags は、プログラムと同じフォルダーか、`PATH` のフォルダーから探します。見つからないときは、そう知らせます。

## パネル

| 項目 | 意味 |
|---|---|
| フォルダー | タグファイルを作るフォルダー。「...」で選び、「上」で 1 つ上にする |
| サブフォルダーも対象にする | 下のフォルダーも調べる（`-R`） |
| コマンドラインオプション | ctags に渡す、利用者のオプション |
| 作成 | ctags を動かす |
| キャンセル | 何もせず閉じる |

動かしている間は、「タグファイルを作成中です。」と「中断」のボタンが同じパネルに出ます。「中断」で ctags を止められます。

## 動き

指定したフォルダーで、`--excmd=n`、（チェックがあれば）`-R`、利用者が書いたオプション、フォルダーの中の全部（`*`）、を渡して ctags を動かします。ctags が文字を出力したら、それをエラーのメッセージとして出します。出力がないまま失敗したときも、そう知らせます。

コマンドの行は、シェルを通さず、単語に分けて渡します。`*` はフォルダーの一覧から作ります（名前が `.` で始まるものは含めません）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_TagJump.cpp` の `Command_TagsMake`、`sakura_core/dlg/CDlgTagsMake.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/tags_make.rs`、`crates/matsu-ui/src/tags/make.rs`（パネル）、`crates/matsu-core/src/tags_make.rs`（ctags の探し方と呼び方。ファイル先頭の説明）
- 差: 上の説明のとおり、ctags を同梱しないこと、シェルを通さないこと（`crates/matsu-core/src/tags_make.rs`）
