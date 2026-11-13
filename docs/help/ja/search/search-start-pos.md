# 検索開始位置へ戻る

直近の検索を始めたときのキャレットの位置へ戻ります。

- メニュー: 検索 → 検索開始位置へ戻る
- 既定のキー: `Ctrl+Shift+F3`
- マクロ: `SearchStartPos`（機能名 `F_JUMP_SRCHSTARTPOS`）

## 動き

- 検索パネル・置換パネルを開いたときのキャレットの位置が、パネルで探したり置換したりした時点で「検索開始位置」として覚えられます。ツールバーの検索ボックスで探したときも、そのときのキャレットの位置を覚えます。このコマンドは、キャレットをそこへ動かします。選択は外れます。
- インクリメンタルサーチでは、最初にモードを始めた位置が検索開始位置です。
- まだ検索開始位置がないときは、メニューの項目は灰色になります。キーで呼ぶと、ステータスバーに「検索開始位置がまだない」と出て、何も動きません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Bookmark.cpp` の `Command_JUMP_SRCHSTARTPOS`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/jump_srchstartpos.rs`、`crates/matsu-ui/src/search/ops.rs`（`jump_to_search_start`）、`crates/matsu-ui/src/search/isearch.rs`（`isearch_enter`）、`crates/matsu-ui/src/search/panel.rs`（`register_start`）、`crates/matsu-ui/src/bars/search_box.rs`
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F3`）
