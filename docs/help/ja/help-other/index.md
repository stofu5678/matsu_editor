# コマンド: ヘルプとメニューに入っていないコマンド

この章には、2 種類の機能があります。

1. 「ヘルプ」メニューの機能（1 機能に 1 ページ）
2. どのメニューにも入っていない機能（種類ごとに、表で説明）

メニューにない機能は、キー、ツールバー、カスタムメニュー、「コマンド一覧」、マクロから動かします。キーに割り当てるには、「共通設定」の「キー割り当て」のページを使います。「コマンド一覧」の使い方は、[コマンド一覧](command-list.md)にあります。

## 「ヘルプ」メニュー

| 機能名 | メニュー | 既定のキー | ページ |
|---|---|---|---|
| `F_HELP_CONTENTS` | ヘルプ目次 | `Alt+F1` | [ヘルプ目次](help-contents.md) |
| `F_HELP_SEARCH` | ヘルプキーワード検索 | `Alt+Shift+F1` | [ヘルプキーワード検索](help-search.md) |
| `F_MENU_ALLFUNC` | コマンド一覧 | `Shift+F1` | [コマンド一覧](command-list.md) |
| `F_CREATEKEYBINDLIST` | キー割り当て一覧をコピー | `Ctrl+Q` | [キー割り当て一覧をコピー](copy-key-bind-list.md) |
| `F_EXTHELP1` | 外部ヘルプ１ | `Ctrl+F1` | [外部ヘルプ１](ext-help1.md) |
| `F_EXTHTMLHELP` | 外部HTMLヘルプ | `F1` | [外部HTMLヘルプ](ext-html-help.md) |
| `F_ABOUT` | バージョン情報 | `Ctrl+Shift+F1` | [バージョン情報](about.md) |

`F1` だけでは、マツエディタのヘルプは開きません。サクラエディタと同じで、`F1` は外部HTMLヘルプです。マツエディタのヘルプは `Alt+F1` です。macOS では、`Alt` は `Option` です。

## メニューにない機能

メニューにない機能のうち、別の章に説明がないものを、種類ごとのページにまとめました。

| 種類（コマンド一覧のグループ） | 機能 | ページ |
|---|---|---|
| ファイル操作系 | `F_FILESAVE_QUIET`、`F_OPEN_FOLDER_IN_EXPLORER`、シェルを開く 4 つ、`F_PROFILEMGR` | [ファイルの場所とシェル、保存、プロファイル](file-shell.md) |
| カーソル移動系 | `F_UP` `F_DOWN` `F_LEFT` `F_RIGHT`、半ページ、行の上下端へ、1 行スクロール、オートスクロール、ホイールの 8 つ | [カーソルの基本の移動とスクロール](cursor-scroll.md) |
| 選択系、矩形選択系、モード切り替え系 | 「(選択)」と「(矩形選択)」の移動 12 個、`F_SELECTLINE`、`F_CANCEL_MODE` | [選択と矩形選択の移動、1行選択、モードの取り消し](select-box-keys.md) |
| 入力系、挿入系、クリップボード系 | `F_WCHAR`、`F_IME_CHAR`、`F_INSTEXT_W`、`F_ADDTAIL_W`、`F_COPYDIRPATH` | [文字の入力、テキストの挿入、フォルダ名のコピー](input.md) |
| 検索系 | `F_REPLACE`、`F_REPLACE_ALL`、`F_SEARCH_BOX` | [置換の実行、検索ボックス、そのほかの検索系](search-others.md) |
| モード切り替え系、設定系 | `F_CHGMOD_EOL_NEL` `F_CHGMOD_EOL_LS` `F_CHGMOD_EOL_PS`、フォントサイズの 3 つ、`F_TYPE_*` の 6 つ | [設定まわりのメニューにない機能](settings-others.md) |

### まだ動かない機能

メニューにない機能のうち、MATSU Editor にコマンドがないのは、次の 1 つだけです。

| 機能名 | 表示名 | 理由 |
|---|---|---|
| `F_PROFILEMGR` | プロファイルマネージャ | 設定ファイル `matsu.ini` の置き場所をどう決めるか、判断が要るため（詳しくは [ファイルの場所とシェル、保存、プロファイル](file-shell.md)） |

動かすと、ステータスバーに「F_PROFILEMGR はまだ使えない」と出ます。

## ほかの章にある、メニューにない機能

次の機能も、メニューにありません。説明は、ほかの章にあります。

| 機能名 | 説明のある章 |
|---|---|
| `F_FILESAVEAS`、`F_FILEOPEN_DROPDOWN` | 「コマンド: ファイル」 |
| `F_CTRL_CODE` | 「コマンド: 編集」 |
| `F_GREP`、`F_GREP_REPLACE`、`F_JUMP`、`F_OUTLINE_TOGGLE`、`F_TAGJUMP_LIST`、`F_TAGJUMP_CLOSE`、`F_BOOKMARK_PATTERN`、`F_DIFF` | 「コマンド: 検索」（一覧は [置換の実行、検索ボックス、そのほかの検索系](search-others.md)） |
| `F_OPTION_GENERAL` などの `F_OPTION_*` | 「コマンド: 設定（共通設定）」の「共通設定...」のページ |
| `F_EXECMD`、`F_CUSTMENU_1` から `F_CUSTMENU_24`、`F_MENU_RBUTTON` | 「コマンド: ツール」 |
| `F_ONELINER_0` から `F_ONELINER_49` | 「MATSU Editor だけの機能」 |

## マクロからだけ呼ぶ機能

マクロからだけ呼べて、メニューにもキーにも割り当てられない機能（`F_FILEOPEN2`、`F_PUTFILE`、`F_MOVECURSOR`、`F_TRACEOUT` など 23 個）は、「マクロの関数」の章にあります。マクロの名前は、サクラエディタと同じです。

インクリメンタルサーチの中で使われる `F_ISEARCH_ADD_CHAR`、`F_ISEARCH_ADD_STR`、`F_ISEARCH_DEL_BACK` は、検索の途中の入力のための内部の機能です。メニューにも、キーにも、マクロにもありません。

## 根拠

- 機能の一覧と、メニューにあるかどうか: `crates/matsu-commands/src/sakura/functions.rs`（`FUNCTIONS`）、`crates/matsu-commands/src/sakura/main_menu.rs`（`MAIN_MENU`）。メニューにない「コマンド」の機能は 165 個で、機能名を `SAKURA` 定数に持つコマンドを `crates/` で探して、`F_PROFILEMGR` 以外のすべてにあることを確かめた
- グループの分け方: `crates/matsu-commands/src/sakura/function_lists.rs`（`FUNCTION_LISTS`）、`crates/matsu-commands/src/sakura/functions.rs`（`FUNCTION_GROUPS`）
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`
- 動かないもの: `docs/sakura-gaps.md`（「コマンドと関数」の表）、`crates/matsu-ui/src/view/mod.rs` の `run_function`
- 内部の機能: `crates/matsu-ui/src/search/isearch.rs`
