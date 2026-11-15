# 同名のC/C++ヘッダー(ソース)を開く

いま開いている C/C++ のソースに対応するヘッダーを、ヘッダーに対応するソースを、開きます。

- メニュー: 検索 → 同名のC/C++ヘッダー(ソース)を開く
- 既定のキー: `Ctrl+Shift+C`、`Ctrl+Shift+H`
- マクロ: `OpenHfromtoC`（機能名 `F_OPEN_HfromtoC`）

## 動き

同じフォルダーにある、同じ名前で拡張子が違うファイルを探し、見つかったものを開きます。

| いまのファイルの拡張子 | 探す拡張子（この順） |
|---|---|
| `.c` `.cpp` `.cxx` `.cc` `.cp` `.c++`（ソース） | `.h` `.hpp` `.hxx` `.hh` `.hp` `.h++` |
| `.h` `.hpp` `.hxx` `.hh` `.hp` `.h++`（ヘッダー） | `.c` `.cpp` `.cxx` `.cc` `.cp` `.c++` |

- 拡張子の大文字小文字は、区別しません。
- 開くファイルは、新しいウィンドウで、いまの文書と同じ文字コードで開きます。すでに開いていれば、そのウィンドウを手前に出します。
- ファイルのない文書、C/C++ でない文書、相手のファイルが存在しないときは、ステータスバーに「対応するヘッダー(ソース)ファイルがない」と出ます。
- 開く前のキャレットの位置は、[タグジャンプ](tag-jump.md)のスタックに積まれ、タグジャンプバックで戻れます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_OPEN_HfromtoC`、`Command_OPEN_HHPP`、`Command_OPEN_CCPP`、`sakura_core/view/CEditView_Command.cpp` の `OPEN_ExtFromtoExt`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/open_header_source.rs`、`crates/matsu-ui/src/file_ops/header_source.rs`（拡張子の表）
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`C`、`H`）
