# 最近使ったファイル

最近開いたファイルの一覧から、ファイルを開きます。

- 機能名: `F_FILE_USED_RECENTLY`（メニューの中に一覧を作るための印で、単独のコマンドではありません）
- メニュー: 「ファイル」の「最近使ったファイル」の子のメニュー
- 既定のキー: なし
- マクロ: なし

## 動き

子のメニューに、最近のファイルが 1 行ずつ並びます。行の形は次のとおりです。

```text
1 C:\work\memo.txt
2 ★ C:\work\notes.txt[UTF-8]
```

- 行の頭の英数字は、選ぶための番号です（1 から 9、A から Z、0 の順）
- お気に入りに入れているファイルには、星（★）が付きます
- 文字コードが Shift_JIS 以外のファイルには、末尾に `[UTF-8]` のように文字コードが付きます
- パスの見せ方は、共通設定の「ファイル名表示」に従います

行を選ぶと、そのファイルを開きます。履歴に残っている文字コードで開きます。開き方は[開く](file-open.md)と同じです。

### 履歴に入るもの

- ファイルを閉じるときに、そのときのキャレットの位置やブックマークと一緒に、履歴の先頭に入ります
- アプリケーションを終了するときは、開いているファイルすべてが入ります

履歴の数は、共通設定「全般」の「履歴」の「ファイルの履歴MAX」で決めます。入れられる数の上限は 36 です。

## サクラエディタとの違い

- 履歴の管理ダイアログのうち、除外するファイルの編集と、ファイルの種類による復元は、まだ使っていません（`docs/sakura-gaps.md`）
- メニューの項目に、`&` で付けるアクセスキーは付けません（GPUI のメニューにアクセスキーがないため）

## 根拠

- サクラのソース: `sakura_core/recent/CMRUFile.cpp` の `CreateMenu`、`sakura_core/env/CFileNameManager.cpp` の `GetMenuFullLabel`
- MATSU のコード: `crates/matsu-ui/src/history/menu.rs`（`label`）、`crates/matsu-ui/src/history/view.rs`（`open_recent_file`・`record_all`）、`crates/matsu-ui/src/file_ops/close.rs`（`confirm_discard` が履歴に入れる）、`crates/matsu-config/src/history/mod.rs`（`MAX_MRU`）
- 設定のキー: `nMRUArrNum_MAX`（`crates/matsu-ui/src/common_settings/pages/general.rs`）
- 差の記録: `docs/sakura-gaps.md`
