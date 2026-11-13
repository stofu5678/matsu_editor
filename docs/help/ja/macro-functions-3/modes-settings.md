# モードと設定の関数

入力のモード、文字コードと改行、バーの表示、文書タイプ、フォント、折り返しなどを決める関数です。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `ChgmodINS` | なし | なし | 挿入モードと上書きモードを切り替える。 | する | モードは、アプリではなく文書ごとに持つ。 |
| `ChgCharSet` | 整数（文字コード（省略、-1、99 は「指定なし」））、整数（BOM（0 以外で付ける）） | なし | 保存するときの文字コードと BOM を決める。文字は変換せず、文書も変更済みにならない。文字コードを指定しないときは、ダイアログで選ぶ。 | する | BOM を付けられない文字コードでは、BOM の指定は効かない。 |
| `ChgmodEOL` | 整数（改行の種類（1 CRLF、3 LF、4 CR、5 NEL、6 LS、7 PS）） | なし | Enter が入れる改行を、指定した種類にする。 | する | 1、3、4、5、6、7 以外の数は、何もしない（メッセージも出ない）。通常の保存では、行の改行を変換しない。 |
| `CancelMode` | なし | なし | 選択、選択ロック、矩形選択を取り消す。キャレットは動かさない。 | する | 記録なし |
| `ShowToolbar` | なし | なし | ツールバーの表示と非表示を切り替える。 | しない | 設定は `[Common]` の `bDispTOOLBAR`。すべてのウィンドウに反映する。 |
| `ShowFunckey` | なし | なし | ファンクションキーバーの表示と非表示を切り替える。 | しない | 設定は `[Common]` の `bDispFUNCKEYWND`。すべてのウィンドウに反映する。 |
| `ShowTab` | なし | なし | タブバーの表示と非表示を切り替える。 | しない | 切り替えると、タブとウィンドウの組み方が変わる。タブにすると、全ウィンドウの文書が 1 つのウィンドウのタブに集まる。タブをやめると、タブごとにウィンドウができる。設定は `[Common]` に書く。 |
| `ShowStatusbar` | なし | なし | ステータスバーの表示と非表示を切り替える。 | しない | このウィンドウだけを切り替え、設定ファイルへは書かない。 |
| `ShowMiniMap` | なし | なし | ミニマップの表示と非表示を切り替える。 | しない | 設定は `[Common]` の `bDispMiniMap`。すべてのウィンドウに反映する。 |
| `TypeList` | なし | なし | タイプ別設定一覧のダイアログを開き、選んだタイプをこの文書に当てる。 | しない | 記録なし |
| `ChangeType` | 整数（タイプの番号（タイプ番号 + 1。0 は今のタイプのまま固定）） | なし | 文書タイプを、このウィンドウが文書を表示しているあいだだけ当てる。 | しない | マクロから渡せるのは整数だけ。タイプの名前（文字列）は、マクロの引数としては通らない。 |
| `OptionType` | なし | なし | この文書のタイプのタイプ別設定を開く。 | しない | 記録なし |
| `OptionCommon` | なし | なし | 共通設定を開く。最後に使ったページから始める。 | しない | 記録なし |
| `SelectFont` | なし | なし | フォントのダイアログを開き、選んだフォントをすべてのウィンドウの共通のフォントにする。 | しない | 固定幅でないフォントも選べる。タイプ別設定に自分のフォントがあるウィンドウは、そのフォントのまま。 |
| `SetFontSize` | 整数（サイズ（1/10 ポイント。0 なら次の引数で決める））、整数（拡大の段数（サイズが 0 のとき））、整数（変える対象（0 共通のフォント、1 タイプのフォント、2 このウィンドウだけ）） | なし | フォントのサイズを決める。サイズ 0 のときは、拡大の表を段数だけ動かす。 | する | TrueType のフォントだけが変わる。対象の値が 0〜2 以外か、サイズと段数が両方 0 のときは、何もしない。対象 0 と 1 は、設定ファイルを書き、そのフォントを使う全ウィンドウに反映する。マクロの続きが先に動いてから、設定が変わる。 |
| `WrapWindowWidth` | なし | なし | 折り返しの桁を、ウィンドウの幅、最大の桁、文書タイプの桁の順に切り替える。 | しない | 記録なし |
| `OptionFavorite` | なし | なし | 履歴の管理のウィンドウを開く。 | しない | 記録なし |
| `SetMsgQuoteStr` | 文字列（引用符） | なし | 「引用符付きコピー」が各行の先頭に付ける引用符を決める。 | しない | 31 文字（UTF-16 の単位）までで、残りは切る。決めた値は、セッションのあいだ、すべてのウィンドウで使い、設定ファイルへは書かない。 |
| `TextWrapMethod` | 整数（折り返し方（0 折り返さない、1 文書タイプの桁、2 ウィンドウの右端）） | なし | 折り返し方を、このビューだけの一時の設定として決める。 | する | 今と同じ折り返し方、または 0〜2 以外の数のときは、何もしない。 |
| `SelectCountMode` | 整数（0 切り替え、1 文字数、2 バイト数） | なし | ステータスバーが選択を数える単位を、このウィンドウで決める。 | する | 共通設定へは書かない。0〜2 以外の数は、何もしない。 |


## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`ChgmodINS` から `CancelMode`、`ShowToolbar` から `SelectCountMode`）
- サクラ: `sakura_core/cmd/CViewCommander_ModeChange.cpp`、`sakura_core/cmd/CViewCommander_Settings.cpp`、`sakura_core/macro/CMacro.cpp`
- MATSU のコード（`crates/matsu-commands/src/commands/`）: `edit/toggle_overwrite_mode.rs`、`mode/chgmod_eol.rs`、`mode/cancel_mode.rs`、`macro_only/set_quotestring.rs`
- MATSU のコード（`crates/matsu-ui/src/ui_commands/`）: `mode/chg_charset.rs`、`settings/` の `show_toolbar.rs`、`show_funckey.rs`、`show_status_bar.rs`、`show_minimap.rs`、`changetype.rs`、`type_list.rs`、`option.rs`、`option_type.rs`、`font.rs`、`set_font_size.rs`、`wrap_window_width.rs`、`select_count_mode.rs`、`history/favorite.rs`、`macro_only/textwrapmethod.rs`、`window/showtab.rs`
- 引数の型の検査: `crates/matsu-plugin/src/bridge.rs`（`convert_arguments`）
