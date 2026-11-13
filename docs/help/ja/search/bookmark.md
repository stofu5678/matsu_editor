# ブックマーク

行に印（ブックマーク）を付けて、あとでその行へ移動する機能です。メニュー「検索」の「ブックマーク」の下に、5 つの機能が入っています。

| 機能 | メニュー | 既定のキー | マクロ（機能名） |
|---|---|---|---|
| ブックマーク設定・解除 | ブックマーク → ブックマーク設定・解除 | `Ctrl+F2` | `BookmarkSet`（`F_BOOKMARK_SET`） |
| 次のブックマークへ | ブックマーク → 次のブックマークへ | `F2` | `BookmarkNext`（`F_BOOKMARK_NEXT`） |
| 前のブックマークへ | ブックマーク → 前のブックマークへ | `Shift+F2` | `BookmarkPrev`（`F_BOOKMARK_PREV`） |
| ブックマークの全解除 | ブックマーク → ブックマークの全解除 | `Ctrl+Shift+F2` | `BookmarkReset`（`F_BOOKMARK_RESET`） |
| ブックマークの一覧... | ブックマーク → ブックマークの一覧... | `Alt+F2` | `BookmarkView`（`F_BOOKMARK_VIEW`） |

ブックマークは、検索パネルの「該当行マーク」でも付きます（マクロは `BookmarkPattern`、機能名 `F_BOOKMARK_PATTERN`）。

## ブックマーク設定・解除

キャレットのある行に印を付けます。すでに付いていれば外します。複数行を選んでいるときは、選択の最初の行から最後の行まで、行ごとに付け外しを反転します（選択の終わりが行頭なら、その行は含めません）。

ブックマークは文書に付く印で、文字の編集ではありません。保存しても、変更ありの印は付きません。Undo の対象でもありません。行が増減する編集があっても、印はその行に付いたままです。

## 次のブックマークへ・前のブックマークへ

キャレットから後ろ（前）にある、印の付いた行の先頭へ動きます。

- 選択のロック中は、選択を伸ばしながら動きます。
- 検索の選択肢「先頭（末尾）から再検索する」が入なら、端まで行ったら反対の端から続けます。1 周して、元の行に止まることもあります。続けたときは、ステータスバーに「▼先頭から再検索しました」（前は「▲末尾から再検索しました」）と出ます。
- 見つからないときは、ステータスバーに「▽見つかりませんでした」（前は「△」）と出ます。選択肢「見つからないときにメッセージを表示」が入なら、「前方(↓) にブックマークが見つかりません。」（前は「後方(↑) に…」）と箱で知らせます。

## ブックマークの全解除

その文書の印をすべて外します。

## ブックマークの一覧...

アウトラインのペイン（[アウトライン解析](outline.md)）に、印の付いた行の一覧を出します。項目を選ぶと、その行へ動きます。一覧にある「空行を無視する」で、空の行を一覧から除けます。

## 該当行マーク

探す文字列に一致する行すべてに印を付けます。すでにある印は残ります。探す文字列がないときは何もしません。検索パネルのボタンと同じで、条件は検索パネルのものを使います（`BookmarkPattern` マクロの引数は読まず、いまの条件を使います）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Bookmark.cpp`（`Command_BOOKMARK_SET`、`_NEXT`、`_PREV`、`_RESET`、`_PATTERN`）、`sakura_core/docplus/CBookmarkManager.cpp`、`sakura_core/doc/CDocOutline.cpp` の `MakeFuncList_BookMark`
- MATSU のコード: `crates/matsu-commands/src/commands/bookmark/`（設定・解除、全解除）、`crates/matsu-commands/src/context/bookmarks.rs`、`crates/matsu-ui/src/ui_commands/search/bookmark_next.rs`、`bookmark_prev.rs`、`bookmark_view.rs`、`bookmark_pattern.rs`、`crates/matsu-ui/src/search/bookmarks.rs`（メッセージと動き）、`crates/matsu-core/src/bookmarks.rs`、`crates/matsu-outline/src/bookmark.rs`、`crates/matsu-ui/src/outline/mod.rs`
- 試験: `crates/matsu-ui/src/search/bookmarks_tests.rs`
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F2`）
