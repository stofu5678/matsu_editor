# コマンド: 検索

メニューの「検索」にある機能の説明です。文字列を探す、置き換える、複数のファイルを探す（Grep）、行や関数やタグへ移動する、2 つの文書の違いを調べる、といった機能が入っています。

| メニュー | ページ |
|---|---|
| 検索... | [検索...](search.md) |
| 次を検索 / 前を検索 | [次を検索・前を検索](search-next-prev.md) |
| 置換... | [置換...](replace.md) |
| 検索マークの切替え | [検索マークの切替え](search-mark.md) |
| 検索開始位置へ戻る | [検索開始位置へ戻る](search-start-pos.md) |
| インクリメンタルサーチ | [インクリメンタルサーチ](incremental-search.md) |
| ブックマーク | [ブックマーク](bookmark.md) |
| Grep... | [Grep](grep.md) |
| Grep置換... | [Grep置換](grep-replace.md) |
| 指定行へジャンプ... | [指定行へジャンプ](jump.md) |
| アウトライン解析... | [アウトライン解析](outline.md) |
| ファイルツリー | [ファイルツリー](file-tree.md) |
| タグジャンプ / タグジャンプバック | [タグジャンプ](tag-jump.md) |
| ダイレクトタグジャンプ | [ダイレクトタグジャンプ](direct-tag-jump.md) |
| キーワードを指定してタグジャンプ... | [キーワードを指定してタグジャンプ](keyword-tag-jump.md) |
| タグファイルの作成... | [タグファイルの作成](tags-make.md) |
| 同名のC/C++ヘッダー(ソース)を開く | [同名のC/C++ヘッダー(ソース)を開く](open-header-source.md) |
| ファイル内容比較... | [ファイル内容比較](compare.md) |
| DIFF差分表示... / 次の差分へ / 前の差分へ / 差分表示の全解除 | [DIFF差分表示](diff.md) |
| 対括弧の検索 | [対括弧の検索](bracket-pair.md) |

## 共通のこと

- 検索・置換・指定行へジャンプ・Grep の入力欄は、別のウィンドウではなく、編集ウィンドウの中に出るパネルです。同時に開けるのは、検索系のパネルか Grep のパネルのどちらか 1 つです。
- 検索の条件（探す文字列と選択肢）は、ウィンドウごとに覚えています。「次を検索」などは、最後に使った条件で探します。
- 探す文字列の書き方（正規表現）は、「仕様等と正規表現」の章で説明します。
- Migemo を使う検索は、まだありません。ライセンスの判断が済んでいないためです（`docs/sakura-gaps.md`）。

## 根拠

- メニューの構成: `crates/matsu-commands/src/sakura/main_menu.rs`（「検索」の節）
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`
- コマンド: `crates/matsu-ui/src/ui_commands/search/`、`crates/matsu-commands/src/commands/bookmark/`、`crates/matsu-commands/src/commands/diff/`、`crates/matsu-commands/src/commands/cursor/`
- 差の一覧: `docs/sakura-gaps.md` の「検索と置換、Grep」
