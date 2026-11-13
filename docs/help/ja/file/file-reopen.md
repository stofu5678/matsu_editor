# 開き直す

今のファイルを、もう一度ディスクから読みます。文字コードを指定して読み直すこともできます。

「ファイル」メニューの「開き直す」の子のメニューに、10 個の機能が入っています。動きは同じで、読むときの文字コードだけが違うので、1 ページにまとめます。

| メニューの項目 | 機能名 | 文字コード | マクロ | 既定のキー |
|---|---|---|---|---|
| 開き直す | `F_FILE_REOPEN` | 今のまま | `FileReopen` | なし |
| SJISで開き直す | `F_FILE_REOPEN_SJIS` | Shift_JIS | `FileReopenSJIS` | `Ctrl+Alt+F12` |
| JISで開き直す | `F_FILE_REOPEN_JIS` | JIS | `FileReopenJIS` | `Ctrl+Alt+F11` |
| EUCで開き直す | `F_FILE_REOPEN_EUC` | EUC-JP | `FileReopenEUC` | `Ctrl+Alt+F10` |
| Latin1で開き直す | `F_FILE_REOPEN_LATIN1` | Latin1 | `FileReopenLatin1` | なし |
| UTF-16で開き直す | `F_FILE_REOPEN_UNICODE` | UTF-16LE | `FileReopenUNICODE` | `Ctrl+Alt+F9` |
| UTF-16BEで開き直す | `F_FILE_REOPEN_UNICODEBE` | UTF-16BE | `FileReopenUNICODEBE` | なし |
| UTF-8で開き直す | `F_FILE_REOPEN_UTF8` | UTF-8 | `FileReopenUTF8` | `Ctrl+Alt+F8` |
| CESU-8で開き直す | `F_FILE_REOPEN_CESU8` | CESU-8 | `FileReopenCESU8` | なし |
| UTF-7で開き直す | `F_FILE_REOPEN_UTF7` | UTF-7 | `FileReopenUTF7` | `Ctrl+Alt+F7` |

マクロは、どれも 1 つの引数を取ります。0 以外にすると、変更の確認を出しません。

## 動き

1. 文書に変更があり、ファイルがディスクにあるときは、確認が出ます。「このファイルは変更されています。再ロードを行うと変更が失われますが、よろしいですか?」で、「再読み込みする」か「キャンセル」を選びます。マクロの引数が 0 以外なら、確認は出ません
2. 指定した文字コード（「開き直す」は今の文字コード）で、ファイルを読み直します
3. キャレットは、元の位置に残ります

### ファイルがない文書のとき

名前のない文書や、ディスクにないファイルの文書では、読み直すものがありません。文字コードだけが変わります。

## 使いどころ

文字化けしたときに、正しい文字コードを選んで読み直します。自動判定の結果を変えたいときにも使います。

## サクラエディタとの違い

動きはサクラエディタと同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILE_REOPEN`、`sakura_core/doc/CDocFileOperation.cpp` の `ReloadCurrentFile`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_reopen.rs` と、同じフォルダーの `file_reopen_*.rs`、`crates/matsu-ui/src/file_ops/load.rs` の `reopen`、確認の文は `crates/matsu-ui/src/file_ops/ask.rs` の `reload_message`
- マクロ名: `crates/matsu-commands/src/sakura/macro_functions.rs`
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（F7 から F12 の Ctrl+Alt）
