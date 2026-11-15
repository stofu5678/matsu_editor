# タグジャンプ

キャレットのある行に書かれたファイルと行番号へ移動します。コンパイラのエラーや、Grep の結果の行から、その場所を開くときに使います。戻るには「タグジャンプバック」を使います。

- メニュー: 検索 → タグジャンプ / タグジャンプバック
- 既定のキー: `F12`、`Ctrl+T`（タグジャンプ）、`Shift+F12`、`Ctrl+Shift+T`（タグジャンプバック）
- マクロ: `TagJump`、`TagJumpBack`（機能名 `F_TAGJUMP`、`F_TAGJUMPBACK`）。ジャンプしたあと、元のウィンドウを閉じる機能 `F_TAGJUMP_CLOSE` もあります（メニューにはありません）

## タグジャンプの動き

1. 行の内容から、ファイルの名前と行番号（と桁）を読み取ります。読み取れる形は次のとおりです。
   - Grep の結果の行: `パス(行,桁): 本文`、`パス(行,桁)  [UTF-8]: 本文`、`パス(行): 本文`
   - Grep の「ファイル毎」の形: `・パス(行,桁): …` と、`・(  行,桁   ): …`（見出しの `■"フォルダー"`、`◆"ファイル"`、`◎"ベース"` が相対パスの元になる）
   - そのほか: 行の中でいちばん先に出てくる、実在するファイルの名前と、後ろの `(行,桁)`、`:行`、` line N` など（コンパイラのメッセージ）。数字のうち、最初のものが行、カンマの後ろが桁
   - 保存していない文書を Grep した結果の行（`:HWND:`）は、その文書のウィンドウへ移る
2. 読み取れたら、いまの位置を覚えてから、そのファイルへ移ります。すでに開いていればそのウィンドウを手前にし、なければ新しいウィンドウで開きます（文字コードは自動判別）。行番号がないときは、キャレットを動かしません。
3. 行からは読み取れないときは、選択している文字列（なければ、キャレットの位置の単語）を、タグファイル（`tags`）から探します（[ダイレクトタグジャンプ](direct-tag-jump.md)と同じ）。
4. それも見つからないときは、ステータスバーに「タグジャンプできません」と出ます。

## タグジャンプバック

最後のジャンプの前の位置（ウィンドウとキャレット）へ戻ります。位置は、すべてのウィンドウで共有する 1 つのスタックに積まれ、100 個まで覚えます（いっぱいなら、古いものから捨てます）。戻る先がないときは、ステータスバーに「タグジャンプバックできません」と出ます。

## Grep の結果から

Grep のウィンドウでは、`Enter` とダブルクリックもタグジャンプになります。共通設定「検索」の「エンターキーでタグジャンプ」「ダブルクリックでタグジャンプ」で切り替えます（既定は両方入）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_TagJump.cpp`（`Command_TAGJUMP`、`Command_TAGJUMPBACK`、`Command_TagJumpNoMessage`）、`sakura_core/view/CEditView_Command.cpp`（`TagJumpSub`）、`sakura_core/env/CTagJumpManager.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/tagjump.rs`、`tagjumpback.rs`、`tagjump_close.rs`、`crates/matsu-ui/src/file_ops/tag_jump.rs`（スタック、メッセージ、Grep の `Enter`）、`crates/matsu-core/src/tag_jump.rs`（行の読み取り。ファイル先頭の説明）、`crates/matsu-ui/src/common_settings/pages/search.rs`
- 試験: `crates/matsu-ui/src/file_ops/tag_jump_tests.rs`
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F12`、`T`）
