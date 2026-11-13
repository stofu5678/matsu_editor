# Grep置換

フォルダーの中の複数のファイルで、文字列を探して置き換えます。ファイルは書き換わります。

- メニュー: 検索 → Grep置換...（パネルを開く）
- 既定のキー: なし
- マクロ: `GrepReplace(条件, 置換後, ファイル, フォルダー, フラグ, 文字コード)`（機能名 `F_GREP_REPLACE`）。パネルを開くのは機能名 `F_GREP_REPLACE_DLG`。キーマクロの記録には入りません

## パネル

[Grep](grep.md) のパネルに、次の違いがあります。

- 「条件」が「置換前」になり、その下に「置換後」の欄が増えます。
- 「編集中のテキストから検索」は選べません。
- 実行のボタンが「置換」になります。
- 次の項目が増えます。

| 項目 | 意味 |
|---|---|
| クリップボードの内容で置換 | 置換後の文字列の代わりに、クリップボードの文字列を使う（パネルを開くたびに切に戻る）。クリップボードに文字列がないときは、「クリップボードに有効なデータがありません！」と箱で知らせる |
| バックアップ作成 | 書き換える前のファイルを、`ファイル名.skrold` として残す（既定は入）。前のバックアップは、新しい内容が置き換わった後で、新しいものに替わる |

置換前の文字列が空のときは、「置換条件を指定してください。」と箱で知らせます。

## 動き

- 一致があったファイルだけを書き換えます。結果のウィンドウには、置換した内容と、最後に「N 個を置換しました。」が出ます。
- 文字コードに入らない文字を置換後に使ったときは、そのファイルは書き換えません（サクラエディタは `?` に替えて書きます）。
- 置換した結果のバックアップのファイル（`.skrold`）は、同じ Grep の中で探しません。

## サクラエディタとの違い

- 「編集中のテキストから検索」の相手（`:HWND:`）では、置換をしません。サクラエディタは、何も置換せずに置換した数だけを出します。MATSU Editor は断ります。
- 改行をまたぐ一致は、表示には出ますが、置換しません。行の改行を除いた文字で置換します。
- 巨大なファイルの Grep 置換は、まだ手当てが残っています（`docs/sakura-gaps.md`）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Grep.cpp` の `Command_GREP_REPLACE_DLG`、`Command_GREP_REPLACE`、`sakura_core/dlg/CDlgGrepReplace.cpp`、`sakura_core/agent/CGrepAgent.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/search/grep_replace.rs`、`grep_replace_dlg.rs`、`crates/matsu-ui/src/grep/settings.rs`（`check_replace`）、`crates/matsu-ui/src/grep/panel.rs`、`crates/matsu-ui/src/grep/engine/replace.rs`（`BACKUP_SUFFIX`、書き戻し）、`crates/matsu-ui/src/grep/engine.rs`（説明の「Differences from Sakura」）
- 試験: `crates/matsu-ui/src/grep/replace_tests.rs`
- 差: `docs/sakura-gaps.md`（検索と置換、Grep の改行をまたぐ一致、`:HWND:` との組み合わせ、巨大ファイル）
