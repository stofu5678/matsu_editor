# モードと文字コード・改行コード

「設定」メニューの下の方にある、入力のモードや、保存の文字コードと改行コードを決める機能です。

| メニュー項目 | 機能名 | マクロ | 既定のキー |
|---|---|---|---|
| 挿入／上書きモード切り替え | `F_CHGMOD_INS` | `ChgmodINS()` | `Insert` |
| ビューモード | `F_VIEWMODE` | `ViewMode()`、`ReadOnly()` | なし |
| キーワードヘルプ自動表示 | `F_TOGGLE_KEY_SEARCH` | `ToggleKeyHelpSearch(mode)` | なし |
| 文字コードセット指定... | `F_CHG_CHARSET` | `ChgCharSet(code, bom)` | なし |
| 入力改行コード指定 | `F_CHGMOD_EOL_CRLF` `F_CHGMOD_EOL_LF` `F_CHGMOD_EOL_CR` | `ChgmodEOL(type)` | なし |

キーマクロに記録されるのは、`ChgmodINS`、`ChgCharSet`、`ChgmodEOL` です。

## 挿入／上書きモード切り替え

挿入モードと上書きモードを替えます。モードは、文書ごとのものです。始めは挿入モードです。キャレットの形とステータスバーの表示が、モードに合わせて変わります。ステータスバーのモードの部分も、この機能を呼びます。

上書きモードの細かい設定は、[編集](page-edit.md) ページにあります。

## ビューモード

ビューモードにすると、その文書への編集がすべて断られます。もう一度使うと、元に戻ります。

- ウィンドウごとのモードです
- タイトルに「(ビューモード)」が付きます
- 文書を、そのファイルへ上書き保存することは断られます
- 入力、削除、切り取り、貼り付け、元に戻す・やり直す、大文字小文字の変換、置換などが使えません
- ファイルの排他制御を使っているときは、モードが入るときにロックを外し、切れるときに、書き込めるかを調べ直してロックし直します

## キーワードヘルプ自動表示

キャレットがある単語を、キーワードヘルプで引く動きを、入り切りします。マクロ `ToggleKeyHelpSearch(mode)` の `mode` は、0（入り切りを替える。省くと 0）、1（入れる）、2（切る）です。ほかの数は何も変えません。

この設定は、すべてのウィンドウに効き、設定ファイルには書きません。メニューには、いまの状態にチェックが付きます。ヘルプの元になる設定は、[支援](page-support.md) ページにあります。

## 文字コードセット指定...

次に保存するときの、文字コードと BOM を決めます。文章の文字は変換しません。文書も、変更したことにはなりません。

- 引数なし（または空、`-1`、`99`）で呼ぶと、ダイアログで聞きます。ダイアログは、「名前を付けて保存」の文字コードの部分と同じです。キャンセルすると何も変わりません
- `ChgCharSet(code, bom)` で文字コードを渡すと、その場で決まります。`bom` が 0 以外で、BOM を持てる文字コードのときだけ、BOM が付きます
- ステータスバーの文字コードの部分をダブルクリックしても、この機能が動きます

## 入力改行コード指定

Enter で入れる改行を、CRLF・LF・CR から選びます。文書ごとの設定です。

- ステータスバーに出る改行コードが替わります。Enter や、エディタが作る改行が、この改行を使います
- 保存のとき、改行が混ざっていると、この改行にそろえるかを聞きます。ふつうの保存では、行を変換しません
- マクロ `ChgmodEOL(type)` の数は、1 が CRLF、3 が LF、4 が CR です。5 が NEL、6 が LS、7 が PS です。ほかの数（0、2、範囲外）は何もしません
- NEL・PS・LS を入れる機能（`F_CHGMOD_EOL_NEL`、`F_CHGMOD_EOL_PS`、`F_CHGMOD_EOL_LS`）は、メニューには出ません

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_ModeChange.cpp` の `Command_CHGMOD_INS`、`Command_CHG_CHARSET`、`Command_CHGMOD_EOL`、`sakura_core/cmd/CViewCommander_File.cpp` の `Command_VIEWMODE`、`sakura_core/cmd/CViewCommander_Support.cpp` の `Command_ToggleKeySearch`、`sakura_core/macro/CMacro.cpp`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/toggle_overwrite_mode.rs`、`crates/matsu-ui/src/ui_commands/file/view_mode.rs`、`crates/matsu-ui/src/ui_commands/support/toggle_key_search.rs`、`crates/matsu-ui/src/ui_commands/mode/chg_charset.rs`、`crates/matsu-commands/src/commands/mode/chgmod_eol.rs` と `chgmod_eol_*.rs`
- マクロ名と既定のキー: `crates/matsu-commands/src/sakura/macro_functions.rs`、`key_bindings.rs`
