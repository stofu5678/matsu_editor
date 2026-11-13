# ダイレクトタグジャンプ

選択している文字列（なければ、キャレットの位置の単語）を、タグファイル（ctags が作る `tags`）から探して、その定義へ移動します。

- メニュー: 検索 → ダイレクトタグジャンプ
- 既定のキー: なし
- マクロ: `DirectTagJump`（機能名 `F_DIRECT_TAGJUMP`）

## 動き

名前のない文書（Grep の結果のウィンドウを除く）では、プログラムのカレントフォルダーを文書のフォルダーとみなして、タグファイルを探します。保存してからでなくても動きます。Grep の結果のウィンドウでは、この探し方はできません。探す語が空のときも動きません。どちらも、ステータスバーに「タグジャンプできません」と出ます。


1. タグファイルは、`tags` という名前のファイルを、文書のフォルダーから、上のフォルダーへ順に探します。何階層上で見つかったかが、一覧の「階層」の欄になります。
2. 見つかった候補が 1 つなら、そのままジャンプします。複数あれば、候補の一覧のパネルで選びます。
3. ジャンプの前の位置は、[タグジャンプ](tag-jump.md)のスタックに積まれるので、タグジャンプバックで戻れます。

「次のタグファイルを探すか」は、共通設定「検索」の「タグファイルの検索」で決めます。

| 選択肢 | 意味 |
|---|---|
| 次のファイルを検索しない | 最初のタグファイルだけを調べる |
| ヒットしたら次のファイルを検索しない | 見つかるまで、順に調べる（ダイレクトタグジャンプの既定） |
| 次のファイルを検索する | すべてのタグファイルを調べる |

## 候補の一覧

列は、キーワード、階層、行番号、種類、ファイル名、備考です。もっともらしい候補が最初に選ばれます。50 件ずつ表示し、「前へ」「次へ」で頁を替えます。`Enter`、ダブルクリック、「ジャンプ」で決め、`Esc`、「キャンセル」で取りやめます。

## 読めるタグファイル

ctags の出力（`キーワード<TAB>ファイル<TAB>行;"<TAB>種類<TAB>備考`）と、行番号だけの形を読みます。行番号が要るので、ctags は `--excmd=n` で作る必要があります（[タグファイルの作成](tags-make.md)はそうします）。大きなタグファイルも、全体は読みません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_TagJump.cpp` の `Command_TagJumpByTagsFileMsg`、`sakura_core/dlg/CDlgTagJumpList.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/direct_tagjump.rs`、`crates/matsu-ui/src/tags/mod.rs`、`crates/matsu-ui/src/tags/list.rs`（一覧）、`crates/matsu-core/src/tag_file/mod.rs`（`tags` の探し方と読み方）、`crates/matsu-ui/src/common_settings/pages/search.rs`（選択肢）
- 試験: `crates/matsu-core/src/tag_file/tests.rs`、`crates/matsu-ui/src/tags/tests.rs`
