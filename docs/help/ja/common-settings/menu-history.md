# 履歴の管理...

「最近使ったファイル」と「最近使ったフォルダー」の履歴を、一覧で見て、お気に入りにしたり、消したりします。

- 機能名: `F_FAVORITE`
- メニュー: 「設定」→「履歴の管理...」
- 既定のキー: なし
- マクロ: `OptionFavorite()`（引数なし。キーマクロには記録されません）

## 動き

編集ウィンドウとは別の窓が開きます。タブが 2 つあり、「ファイル」と「フォルダー」の履歴を切り替えます。

- 一覧の項目ごとに、「お気に入り」のチェックがあります
- メニューに出る数をこえた項目には、「(非表示)」と付きます
- 次のボタンで、項目を消します。「すべて」、「お気に入り以外」、「存在しない項目」、「選択項目」です
- 変更は、押したときすぐに履歴へ書かれます
- 窓の位置と大きさは、閉じるときに覚え、次に開くときに使います

履歴に残す数は、「共通設定」の [全般](page-general.md) ページで決めます。

## サクラエディタとの違い

サクラエディタの履歴の管理には、検索・置換・Grep・コマンド・カレントフォルダーなどの履歴のタブもあります。MATSU Editor にあるのは、ファイルとフォルダーの 2 つだけです。

サクラエディタは、チェックの変更を、窓を閉じるときか削除の前にまとめて反映します。MATSU Editor は、そのつど反映します。結果は同じです。

## 根拠

- サクラのソース: `sakura_core/dlg/CDlgFavorite.cpp`、`sakura_core/_main/CControlTray.cpp` の `F_FAVORITE` の処理
- MATSU のコード: `crates/matsu-ui/src/ui_commands/history/favorite.rs`、`crates/matsu-ui/src/history/dialog.rs`
- マクロ名: `crates/matsu-commands/src/sakura/macro_functions.rs`（`OptionFavorite`）
