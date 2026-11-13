# 設定まわりのメニューにない機能

メニューにない、モード切り替え系・設定系の機能です。入力改行コード（NEL・LS・PS）、フォントサイズ、タイプ別設定を開く機能が入っています。

| 機能名 | 表示名 | キー | マクロ |
|---|---|---|---|
| `F_CHGMOD_EOL_NEL` | 入力改行コード指定(NEL) | なし | `ChgmodEOL(5)` |
| `F_CHGMOD_EOL_LS` | 入力改行コード指定(LS) | なし | `ChgmodEOL(6)` |
| `F_CHGMOD_EOL_PS` | 入力改行コード指定(PS) | なし | `ChgmodEOL(7)` |
| `F_SETFONTSIZE` | フォントサイズ設定 | なし | `SetFontSize(大きさ, 段数, 方法)` |
| `F_SETFONTSIZEUP` | フォントサイズ拡大 | `Ctrl+ホイールアップ` | なし |
| `F_SETFONTSIZEDOWN` | フォントサイズ縮小 | `Ctrl+ホイールダウン` | なし |
| `F_TYPE_SCREEN` | （名前なし） | なし | なし |
| `F_TYPE_COLOR` | （名前なし） | なし | なし |
| `F_TYPE_WINDOW` | （名前なし） | なし | なし |
| `F_TYPE_HELPER` | （名前なし） | なし | なし |
| `F_TYPE_REGEX_KEYWORD` | （名前なし） | なし | なし |
| `F_TYPE_KEYHELP` | （名前なし） | なし | なし |

## 入力改行コード（NEL・LS・PS）

Enter で入れる改行を、NEL、LS、PS のどれかにします。文書ごとの設定です。文字は変わらず、文書は変更されたことになりません。

- ステータスバーの改行コードの部分を右クリックしたときのメニューに、「入力改行コード指定(NEL)」「入力改行コード指定(LS)」「入力改行コード指定(PS)」が出ます。出るのは、「共通設定」の「編集」の「改行コードNEL, PS, LSを有効にする」（`matsu.ini` の `bEnableExtEol`）がオンのときだけです（既定はオフ）。
- CRLF、LF、CR にする機能は、メニューにあります。そちらは「共通設定」の章の、モードのページにあります。

## フォントサイズ

- `F_SETFONTSIZE`（マクロ `SetFontSize(大きさ, 段数, 方法)`、キーマクロに記録されます）:
  - 「大きさ」は、1 ポイントの 10 分の 1 が単位の数です（12 ポイントなら 120）。1 ポイントから 72 ポイントの間に収めます。0 のときは、「段数」を使います。
  - 「段数」は、拡大縮小の表を、この数だけ進めます（正で拡大、負で縮小）。いまのフォントの大きさから数えます。表は 72 段あり、結果は 0.5 ポイント刻みです。「大きさ」も「段数」も 0 なら、何も変わりません。
  - 「方法」は、何を変えるかです。

    | 方法 | 変わるもの |
    |---|---|
    | 0 | 共通のフォント（全ウィンドウ。`matsu.ini` に書く） |
    | 1 | 文書のタイプのフォント（そのタイプが専用のフォントを持つことになる。`matsu.ini` に書く） |
    | 2 | このウィンドウだけ（一時適用） |

    ほかの数のときは、何も変わりません。
  - 変わるのは、TrueType のフォントだけです。
- `F_SETFONTSIZEUP` `F_SETFONTSIZEDOWN`: 方法 2 で、表を 1 段、進める（戻す）のと同じです。このウィンドウだけの大きさが変わります。`Ctrl` を押しながらホイールを回すと、動きます。
  - 「共通設定」や「タイプ別設定」でフォントを変えたとき、このウィンドウだけの大きさは、設定のフォントの大きさが前と同じなら残ります。変わっていれば、設定のフォントの大きさに戻ります。

## タイプ別設定を開く（ページ指定）

6 つの機能は、いまの文書のタイプの「タイプ別設定」を、次のページで開きます。

| 機能名 | 開くページ |
|---|---|
| `F_TYPE_SCREEN` | スクリーン |
| `F_TYPE_COLOR` | カラー |
| `F_TYPE_WINDOW` | ウィンドウ |
| `F_TYPE_HELPER` | 支援 |
| `F_TYPE_REGEX_KEYWORD` | 正規表現キーワード |
| `F_TYPE_KEYHELP` | キーワードヘルプ |

- サクラエディタには、これらの機能の動きがありません。ヘルプのページの番号にだけ使われています。MATSU Editor は、ページを直接開く機能として動かします（「共通設定」のページを直接開く `F_OPTION_GENERAL` などと同じ考え方です）。
- メニューにも、既定のキーにも、マクロの名前にも、ありません。
- ページの内容は、「コマンド: 設定（タイプ別設定）とウィンドウ」の章を見てください。

## ほかの章にある機能

メニューにない機能のうち、次のものは、別の章に説明があります。

| 機能名 | 説明のある章 |
|---|---|
| `F_OPTION_GENERAL` から `F_OPTION_MAINMENU` まで（共通設定のページを直接開く機能） | 「コマンド: 設定（共通設定）」の「共通設定...」のページ |
| `F_EXECMD` | 「コマンド: ツール」の外部コマンド実行のページ |
| `F_ONELINER_0` から `F_ONELINER_49` | 「MATSU Editor だけの機能」のワンライナーのページ |
| `F_CUSTMENU_1` から `F_CUSTMENU_24`、`F_MENU_RBUTTON` | 「コマンド: ツール」のカスタムメニューのページ |

## 根拠

- サクラのソース: `sakura_core/window/CEditWnd.cpp` の `OnNotify`（改行コードの右クリックメニュー）、`sakura_core/cmd/CViewCommander_ModeChange.cpp` の `Command_CHGMOD_EOL`、`sakura_core/cmd/CViewCommander_Settings.cpp` の `Command_SETFONTSIZE`、`sakura_core/func/Funccode.cpp` の `FuncID_To_HelpContextID`（`F_TYPE_*` はヘルプの番号）
- MATSU のコード: `crates/matsu-commands/src/commands/mode/chgmod_eol_nel.rs`、`chgmod_eol_ls.rs`、`chgmod_eol_ps.rs`、`chgmod_eol.rs`。`crates/matsu-ui/src/status_bar.rs`（`eol_menu_entries`）。`crates/matsu-commands/src/edit_options.rs`、`crates/matsu-ui/src/common_settings/pages/edit.rs`（`enable_ext_eol`）。`crates/matsu-ui/src/ui_commands/settings/set_font_size.rs`、`set_font_size_up.rs`、`set_font_size_down.rs`。`crates/matsu-ui/src/view/settings.rs`（`plan_font_size`、`set_temporary_font_size`、`refresh_font`）、`crates/matsu-ui/src/settings/zoom.rs`。`crates/matsu-ui/src/ui_commands/settings/type_screen.rs` ほか `type_*.rs`、`crates/matsu-ui/src/type_settings/mod.rs`（`PAGE_TITLES`、`page`、`run_command_at`）
- 試験: `crates/matsu-commands/src/commands/mode/chgmod_eol_nel.rs` の `sets_the_line_ending_of_enter`、`leaves_the_text_alone`
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（ホイールの行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`SetFontSize`、`ChgmodEOL`）、`crates/matsu-commands/src/sakura/functions.rs`
