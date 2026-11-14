# 折り返しの切り替え

いまのビューだけ、折り返しの方法や桁数を一時的に変えます。ファイルの設定は変わりません。

## 折り返し方法

「設定」→「折り返し方法」の下に 3 つあります。

| メニュー項目 | 機能名 | 既定のキー | 動き |
|---|---|---|---|
| 折り返さない | `F_TMPWRAPNOWRAP` | `Ctrl+Alt+X` | 折り返しません |
| 指定桁で折り返す | `F_TMPWRAPSETTING` | `Ctrl+Alt+S` | タイプ別設定で決めた桁で折り返します |
| 右端で折り返す | `F_TMPWRAPWINDOW` | `Ctrl+Alt+W` | ウィンドウの右端で折り返します |

「折り返し方法」のメニュー自体は、機能 `F_TEXTWRAPMETHOD`（マクロ `TextWrapMethod(method)`）です。`method` に、次の数を渡します。

| 数 | 方法 |
|---|---|
| 0 | 折り返さない |
| 1 | 指定桁で折り返す |
| 2 | 右端で折り返す |

引数を省くと 0 です。0 から 2 以外の数は、何もしません。すでにその方法のときも、何もしません。キーマクロには記録されます。

## 折り返し桁数

- 機能名: `F_WRAPWINDOWWIDTH`
- メニュー: 「設定」→「折り返し桁数」
- 既定のキー: `Ctrl+Alt+U`
- マクロ: `WrapWindowWidth()`（引数なし。キーマクロには記録されません）

押すたびに、折り返す桁を替えます。

| いまの桁 | 押したあとの桁 |
|---|---|
| ウィンドウの幅 | 最大の桁 |
| 最大の桁 | タイプ別設定の桁（タイプ別設定の桁も最大のときは、ウィンドウの幅） |
| ほかの桁 | ウィンドウの幅 |

ウィンドウが細すぎて折り返せないときは、ウィンドウの幅を飛ばします。

替えたあとの桁は、そのままです。ウィンドウの大きさを変えても、追いかけません。方法は「指定桁で折り返す」になります。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Settings.cpp` の `Command_TEXTWRAPMETHOD`、`Command_WRAPWINDOWWIDTH`、`sakura_core/view/CEditView_Command.cpp` の `GetWrapMode`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/settings/tmp_wrap_no_wrap.rs`、`tmp_wrap_setting.rs`、`tmp_wrap_window.rs`、`wrap_window_width.rs`、`crates/matsu-ui/src/ui_commands/macro_only/textwrapmethod.rs`、`crates/matsu-ui/src/view/settings.rs` の `toggle_wrap_window_width`
- マクロ名: `crates/matsu-commands/src/sakura/macro_functions.rs`
