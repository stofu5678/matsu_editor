# サクラエディタにあって、まだ MATSU Editor にない機能（機能台帳）

作成: 2026-10-24（main の 5f81c9f2、リリース 0.2.0 の直後）。サクラは `48db6c7c`（`xtask sakura-tables` が使っている版）。

ロードマップ Phase 5 の「サクラの機能台帳」にあたる。ループの第 1〜18 ラウンドで、コマンドの大半は入った。ここには、まだ入っていないものだけを書く。次のラウンドは、この表の上から順に実装する。

## 作り方

次の 7 つを調べた（2026-10-24 の main）。

1. 関数表とコマンドの登録の突き合わせ（下の「コマンドと関数」）
2. 取り込み済みの PR 215 件の本文の `## 残っていること`（`gh pr list --state merged --limit 400`）。163 件に項目があった。各項目を main のコードとテストで確かめ、直っているものは落とし、重なりは 1 行にまとめた。確かめきれなかったものは「未確認」と書いた
3. `crates/matsu/tests/sakura_oracle/known_differences.txt`（サクラとの違いの許可リスト）
4. 設定。画面に出ていて「未対応」と灰色にしている部品（コードの `Control::unavailable()`）を全部数えた。サクラの `CShareData_IO.cpp` のキーのうち、コードに名前が出てこないものも数えた
5. サクラの `CDlg*.cpp`、`CPropCom*.cpp`、`typeprop/` と、`crates/matsu-ui/src` の突き合わせ
6. 関数コードがない機能（プラグイン、Migemo、プロファイル、コマンドラインオプション、アウトライン、補完、ヘルプ、ツールバー、タブ、印刷）
7. 調整役の宿題（ADR-0020 の次の段階、`pending-checks.md` の「見つかったこと」）

サクラのソースは読み取り専用の clone で読んだ。bregonig と ctags のソースは読んでいない。

## 保ち方

台帳を埋める PR は、同じ PR で、この表の該当の行を消すか書き換える。行を消せないとき（一部だけ入れたとき）は「MATSU の今」を書き換える。新しく見つけた不足は、行を足す。作り直すときは、上の 1・2・4 の手順を繰り返す。

台帳を `docs/` の 1 ファイルにしたのは、調整役の判断（GitHub の issue にしない）。PR と同じ場所で差分として読め、台帳を埋める PR が行を消すことで進み具合が見えるため。

## 要約

| 領域 | 行数 | 主な不足 |
|---|---|---|
| コマンドと関数 | 15 | 関数コードのあるコマンドで未登録は 9 つ（うち 4 つは FD が今のラウンドで実装か判断、2 つは SQL*Plus で ADR-0023） |
| マクロ | 8 | キーマクロの文（`Sleep` など）、`GetLineAttribute` の種類 3・4、矩形の `MoveCursor`、`ExecExtMacro` の第 2 引数 |
| 設定 | 13 | 共通設定の灰色の部品 47 個とタイプ別設定の灰色の部品 55 個。コマンドが設定を読まず、サクラの既定の値で動くもの |
| ダイアログ | 12 | プラグイン、プロファイル、ウィンドウの位置と大きさ、色統一、強調キーワード選択、タイプ一覧の操作 |
| プラグイン | 3 | 全部。常駐プラグインは ADR-0007 のとおり別の設計 |
| 検索 | 7 | Migemo、Grep の `:HWND:`、複数行の照合の細部、検索条件の保存 |
| ファイル | 9 | 巨大ファイル（ADR-0020 の 2〜8）、コードページ 51932/20932、ベストフィット表 |
| 表示と編集 | 15 | マウス操作、フリーカーソル、キャレットの形、文字間隔、TSV、背景画像、アウトラインの解析器 |
| 印刷 | 4 | 色刷り、プリンターの選択、ヘッダーの一部 |
| OS との結び付き | 7 | タスクトレイ、署名、Wayland のアイコン、Linux の HTML クリップボード |
| アクセシビリティ | 4 | 本文の読み上げ、ポップアップメニュー（ほかは gpui-component の限界） |
| そのほか | 10 | ヘルプ、`-GROUP`、`-NOWIN` |

### 次のラウンドに勧める順番

| # | 項目 | 理由 |
|---|---|---|
| 1 | 効かない設定をつなぐ（共通設定の「編集」「全般」「ウィンドウ」の灰色の部品。設定表の S 判定のもの） | 画面に出ているのに何もしない設定は、利用者が最初にぶつかる。コマンド側は、サクラの既定の値を決め打ちしているだけなので、設定を読ませれば済むものが多い |
| 2 | 検索の条件とウィンドウの位置・大きさを設定ファイルに保存する | 毎回やり直しになる。キー（`bSearchRegularExp`、`nWinPosX` など）はサクラのもので、新しい形式は要らない |
| 3 | マウス操作（Alt+ドラッグの矩形選択、トリプルクリック、行番号の列のクリック、ドラッグ&ドロップ編集、右ボタンのドロップ） | サクラの操作感の中心で、いまの `view/mouse.rs` は左クリック・ダブルクリック・ドラッグだけ |
| 4 | 本文のアクセシビリティ（スクリーンリーダー） | #204 が設計まで出している。ほかのアクセシビリティの穴の大元 |
| 5 | フリーカーソルとキャレットの形（DOS 風）、`F_DELETE` などの追従 | 設定が 3 つ、コマンドの「Not done」が 2 つ、同じ部分 |
| 6 | 巨大ファイルの続き（ADR-0020 の PR 2〜4） | 10GB を開く目標（Phase 6）の本体。部品（#214、#216）はあるが、開く経路につながっていない |
| 7 | アウトラインの残りの解析器（PL/SQL、COBOL、アセンブラ、TeX、Erlang、ルールファイル）とドッキング位置 | 1 解析器ずつ独立して入れられる。並列に向く |
| 8 | キーマクロの文（`Sleep`、`SetDrawSwitch`、メッセージボックス）と `ExecExtMacro` の第 2 引数 | マクロの互換。サクラのマクロが途中で止まる |
| 9 | タイプ別設定の灰色の部品（文字間隔、行間、TSV、TAB の表示、インデント、禁則） | レイアウトに `Kinsoku` などの下地はある。画面と設定をつなぐ |
| 10 | タイプ一覧の操作（追加・削除・複製・上下・インポート・エクスポート・初期化）と色の取り込み・書き出し | 利用者がタイプを増やす手段がない |
| 11 | 17 の組み込みタイプの個別の既定値の確認 | 「基本」のコピーから始めている型があるかもしれない（未確認。下の設定表） |
| 12 | 色刷りの印刷 | 印刷の「Not done」の筆頭 |
| 13 | タブバーの残りの設定（多段、位置、アイコン、幅）とタブを外へドラッグ | 設定が 4 つ灰色。ADR-0024 とぶつかるので、その決定の後 |
| 14 | プラグイン（設計から） | 大きい。ユーザーの判断が要る（ADR-0007） |

ユーザーの判断が先に要る（上の表の外）: Migemo、プロファイル、ベストフィット表、署名。「止まっているもの」に理由を書いた。

## 止まっているもの

| 項目 | 理由 |
|---|---|
| Migemo（`F_ISEARCH_MIGEMO_NEXT` `F_ISEARCH_MIGEMO_PREV`、共通設定「支援」の DLL と辞書、検索の Migemo 使用） | サクラは `cmigemo` の DLL を実行時に読み、辞書は利用者が置く（`sakura_core/extmodule/CMigemo.cpp`）。C/Migemo 本体は MIT License（koron/cmigemo の LICENSE、GitHub が MIT と判定）。ただし通常の辞書は SKK-JISYO.L から作り、その README に「作った辞書は SKK-JISYO.L のライセンス（GPL）に従う」とあり、SKK-JISYO.L の先頭は GNU General Public License（skk-dev/dict）。したがって同梱・組み込みはできない。選択肢は、(a) DLL と辞書を利用者が置く形（サクラと同じ。同梱しない）にする、(b) 組み込まない。どちらもライセンスの判断なので、ユーザーに聞く。いまの `ISearchMode` は後から足せる形になっている |
| プロファイル（`F_PROFILEMGR`、`-PROFMGR`、`CDlgProfileMgr`、`sakura.exe.ini` の `MultiUser`） | `-PROF=名前` は入っている（`launch.rs`、`matsu-config/src/location.rs`）。残りは、設定ファイルの置き場所を変える・利用者のデータの形式に触れる（`matsu.ini` の場所）ので、ユーザーの判断が要る |
| IME の再変換（`F_RECONVERT`）と全角文字入力（`F_IME_CHAR`） | GPUI が OS の再変換の API を出すか。FD（`feat/last-sakura-functions`）が今のラウンドで実装するか見送りを決める。決まったら、この行を直す |
| SQL*Plus（`F_ACTIVATE_SQLPLUS` `F_PLSQL_COMPILE_ON_SQLPLUS`）と PL/SQL のジャンプ | データベース操作の ADR-0023 で扱う（提案中。別のエージェントが書いている）。Windows の Oracle 専用 |
| プラグイン | ADR-0007 が「常駐プラグインは別に設計する」とした。設計の ADR がない |
| ベストフィットの表（全角英数字を ASCII にする Windows の変換） | 表の元データと、そのライセンスの判断が要る（`known_differences.txt` の `saveloss-jp-to-*`） |
| MS-CP51932 / 20932 | ユーザーが「後で決める」と言ったまま（#210）。保存すると何も書かず止まる（`savecode-sjis-crlf-to-cp51932` など 3 件） |
| 署名（macOS の公証、Windows の署名）、`.dmg` | Apple のアカウントなど。リリースなのでユーザーの判断 |

## 領域ごとの表

「判断」の列は、ADR-0018 の型。ユーザーの判断が要るものは「ユーザーの判断が要る: 理由」と書く。「大きさ」は、1 本の PR にしてよいものが S、1 本で収まるが大きいものが M、分けるものが L。

### コマンドと関数

関数表（`crates/matsu-commands/src/sakura/functions.rs`）の「コマンド」363 個のうち、`SAKURA` 定数のあるコマンドは 354 個。差の 9 個は次のとおり（スクリプトの出力を確かめた）。ほかの種類（`MacroFunction` 59、`MacroOnly` 23）は、次の「マクロ」の表で。

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| `F_ACTIVATE_SQLPLUS` | `CViewCommander_File.cpp` | コマンドなし | ADR-0023 で決める | M | ADR-0023 |
| `F_PLSQL_COMPILE_ON_SQLPLUS` | 同上 | コマンドなし | 同上 | M | ADR-0023 |
| `F_FILEOPEN_DROPDOWN` | `CViewCommander_File.cpp` | コマンドなし。ツールバーの ▼ は #157 で動く | FD が実装か判断 | S | なし |
| `F_IME_CHAR` | `CViewCommander_Edit.cpp` | コマンドなし | FD が実装か判断 | S | GPUI の IME |
| `F_RECONVERT` | `CViewCommander_Support.cpp` | コマンドなし | FD が実装か判断 | M | GPUI の IME |
| `F_TAGJUMP_LIST` | `CViewCommander_TagJump.cpp` | コマンドなし。一覧ダイアログ自体は `tags/list.rs` にある（F_TAGJUMP_KEYWORD などから開く） | FD が実装か判断 | S | なし |
| `F_PROFILEMGR` | `CViewCommander_File.cpp` | コマンドなし | ユーザーの判断が要る: 設定ファイルの置き場所（利用者のデータ） | M | `CDlgProfileMgr` |
| `F_ISEARCH_MIGEMO_NEXT` | `CViewCommander_Search.cpp` | コマンドなし | ユーザーの判断が要る: ライセンス（止まっているものを参照） | M | 同上 |
| `F_ISEARCH_MIGEMO_PREV` | 同上 | 同上 | 同上 | 同上 | 同上 |

そのほか、登録しているコマンドのうち、動きが一部だけのもの（コードの「Not done」）:

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| 矩形選択での `F_LTRIM` `F_RTRIM` `F_INDENT_SPACE` `F_UNINDENT_SPACE` | `CViewCommander_Edit.cpp` | 矩形では断る（サクラは断るものと動くものがある。コマンドごとの対応表がない） | C | M | なし |
| `F_WordDeleteToStart` `F_WordDeleteToEnd` の矩形 | `CViewCommander_Edit.cpp` | サクラはビープ。MATSU は動かすかどうか未確認 | C | S | なし |
| `F_DELETE` `F_DELETE_BACK` のフリーカーソル時と折り返し時の動き | `CViewCommander_Edit.cpp`（2008.08.03 nasukoji） | 未対応（`delete.rs`、`delete_back.rs` の「Not done yet」） | C | M | フリーカーソル |
| `F_WCHAR` の上書きの細部（文字幅に合わせて空白を詰める、改行を上書きする）、改行時に末尾の空白を削る、インデント対象文字、スマートインデント | `CViewCommander_Edit.cpp` | 未対応（`input_char.rs`）。設定の部品が灰色 | A/C | M | 設定をつなぐ |
| `F_MERGE` の矩形選択 | `Command_MERGE` 834-836 行 | サクラは何もせず戻る。MATSU はエラーを返し、マクロが止まる（#194） | A | S | なし |
| 単語削除などで、サクラがメッセージボックスを出す箇所（`STR_ERR_DLGEDITVWCMDNW7` など） | `CViewCommander_Edit.cpp` | メッセージは出さない | C | S | なし |

### マクロ

`MacroFunction` 59 個と `MacroOnly` 23 個の名前は、すべてコードのどこかにある。さらに、マクロの名前の表の結果のある行 62 個すべてに、プラグインのホスト（`matsu-plugin/src/functions/`）の実装と、その単体テストがあることを確かめた。サクラとの比較（oracle）のケースは別に 727 件ある。次は、実装があっても一部だけのもの。

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| キーマクロ（`.mac`）の文としての `Sleep`、`SetDrawSwitch`、`MessageBox` `ErrorMsg` `WarnMsg` `InfoMsg` `OkCancelBox` `YesNoBox` | `CMacro::HandleCommand`（`macro/CMacro.cpp`） | マクロが「まだ使えない」で止まる（`macros/statements.rs`） | C | M | タイマー、ダイアログ（`InputBox` の仕組みを使える） |
| `GetLineAttribute` の種類 3（差分）・4（アウトラインの印） | `CMacro.cpp` の `F_GETLINEATTRIBUTE` | 「未実装」を返す（`functions/state.rs`）。差分の印は #93 で入ったので、3 は実装できる | A | S | 4 はアウトラインの印 |
| `MoveCursor` の矩形（オプション 2）、`Jump` の PL/SQL とテキスト折り返しの行番号 | `CMacro.cpp` | エラーを返す（`matsu-plugin/src/api.rs`） | C | M | PL/SQL は ADR-0023 |
| `ExecExtMacro` の第 2 引数（コードを直接渡す形）と `-M=` の `-MTYPE=` による文字列の実行 | `CMacro.cpp`、`CCommandLine.cpp` | ファイルだけ実行できる | C | M | マクロのホスト |
| `ChgmodEOL` の第 2 引数（ロック）、`HalfPageUp(n)` 系・`PageUp(n)` 系の行数引数 | `CMacro.cpp` | 未対応（#209） | C | S | なし |
| 入れ子のマクロの `AddRefUndoBuffer` を閉じ忘れたときの戻し方 | `CMacro.cpp` | 外側が終わるまで閉じない（#97） | C | S | なし |
| `.mac` の文字コード（BOM なしの古いファイルを Shift_JIS として読むか） | `CMacroManagerBase`、`CKeyMacroMgr` | UTF-8 だけ。確かめていない（#44）。未確認 | C | S | なし |
| キーマクロ再生中の分割コマンド（次のコマンドが分割前の大きさで動く） | － | 既知の不具合（#91）。直していない | A | M | マクロの再生の作り |

### 設定

画面に出ていて、効かない部品（`Control::unavailable()`、コードでは「未対応」の灰色）は、共通設定 47 個、タイプ別設定 55 個ある。サクラのキーのうち、コードに名前が出てこないものは 82 個（`CShareData_IO.cpp` の 301 キー中。`GREPFILE` や `nKeywordSelect3` のように連番で組み立てるものを含むので、実際はもっと少ない。未確認）。設定ファイルの値は、読み書きは壊さずに残している（ADR-0010）ので、「動かない」だけで「消える」ことはない。

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| 共通設定「編集」の灰色 10 個: 折り返し行に改行を付けてコピー（`bAddCRLFWhenCopy`）、コピーしたら選択解除、ドラッグ&ドロップ編集とドロップ元、改行を上書きしない、文字幅に合わせてスペースを詰める、矩形入力で選択範囲を削除、クリックで URL を選択、Vista スタイルのファイルダイアログ、矩形選択移動でロック | `CPropComEdit.cpp`、`CommonSetting.h` | 設定は読み書きする。コマンドはサクラの既定の値で動く（`context/clipboard.rs`、`box_edit.rs`、`boxsel.rs`、`input_char.rs` のコメント） | A/C | S〜M | コマンドが設定を読む |
| 共通設定「全般」の灰色 14 個: フリーカーソル、キャレットの形（Windows 風・MS-DOS 風）、マウスクリックでのアクティブ化でキャレットを動かさない、タスクトレイ（使う、常駐、ショートカットキー）、全終了の確認、複数ウィンドウを閉じる確認、ホイールの行数・滑らか・ページスクロール・横スクロール、画面キャッシュ | `CPropComGeneral.cpp` | 灰色。`bCloseAllConfirm` は `win_closeall.rs` が読む（部品は灰色のまま。未確認） | A/C | M | トレイは GPUI の API |
| 共通設定「ウィンドウ」の灰色 7 個: 水平スクロールバー、ダークモード、位置と大きさの設定、ルーラーの高さ・隙間、行番号の隙間、言語 | `CPropComWin.cpp` | 灰色。`nRulerHeight` は `view/ruler.rs` が読む（部品は灰色のまま。未確認） | C | M | 言語は ADR 要（利用者向けの文言の多言語化） |
| 共通設定「タブバー」の灰色: アイコン、多段、表示位置。「ツールバー」のフラットなボタン。「検索」の灰色 5 個（他のビューの条件を引き継ぐ、正規表現ライブラリ、リアルタイム表示、Enter とダブルクリックでタグジャンプ）。「マクロ」の Python フォルダー、キャンセル確認の待ち時間 | `CPropComTab.cpp` ほか | 灰色 | C | S〜M | ADR-0024（タブ） |
| 共通設定「ファイル」の排他制御以外の不足 | `CPropComFile.cpp` | 排他制御は #145 で入った。「名前を付けて保存」の全ファイル表示 2 つは、保存ダイアログの絞り込みに依存（#137） | C | S | ファイルダイアログの絞り込み |
| 共通設定「プラグイン」のページ | `CPropComPlugin.cpp` | ページの枠だけ。中身なし | ユーザーの判断が要る: プラグインの設計（ADR） | L | プラグイン |
| タイプ別設定「画面」の灰色: 文字の間隔、行の間隔、TAB の表示、SPACE の挿入、TSV モード、自動インデント、全角空白も、スマートインデント、インデント対象文字、折り返し行のインデント、改行時に末尾の空白を削除、アウトラインのルールファイル、禁則（句読点ぶら下げ・行頭・行末） | `CPropTypesScreen.cpp` | 灰色。`matsu-layout` に禁則（`kinsoku.rs`）と折り返しインデントの下地はあるが、設定の部品をつないでいない。文字間隔・TSV は `matsu-layout` の「Not done」 | C | M〜L | 設定をつなぐ。TSV・文字間隔はレイアウトの変更 |
| タイプ別設定「ウィンドウ」の灰色: 文字コードの既定、CP、CESU-8 優先、IME の状態と入力モード、文書アイコン、折り返し単位、桁数、縦線、背景画像（パス、位置、繰り返し、スクロール、透明度） | `CPropTypesWindow.cpp` | 灰色 | C | M〜L | 背景画像は描画の変更 |
| タイプ別設定「色」の灰色: 文字色統一、背景色統一、色のインポート・エクスポート、強調キーワードの選択（2〜10）、共通設定への移動 | `CPropTypesColor.cpp`、`CDlgSameColor.cpp`、`CDlgKeywordSelect.cpp`、`CImpExpManager.cpp` | 灰色 | C | M | 色のファイル形式 |
| タイプ別設定「支援」の灰色: 補完の種類、保存時の改行混在の警告、C/C++ のスマートインデント、読み込み時のインデントスタイル検出 | `CPropTypesSupport.cpp` | 灰色 | C | M | 補完プラグイン |
| 17 の組み込みタイプの個別の既定値 | `sakura_core/types/CType_*.cpp` | 17 個とも用意した（`builtin_types.rs`）。値がサクラの `CType_*.cpp` と全部合っているかは未確認 | A/C | S | なし |
| 設定ファイルに保存しない状態: 検索の条件（正規表現・大文字小文字・単語・先頭から再検索など、`bSearch*`）、ウィンドウの位置と大きさ（`nWinPosX` `nWinSizeCX` `bSaveWindowSize` など）、アウトラインのドッキング位置・並べ替え・窓の位置（`eOutlineDockSide` など）、各ダイアログの位置（`rc*Dialog`）、タブの幅（`nTabMinWidth` など）、`bGrep*` の 10 個ほど、`nDiffFlgOpt`、`nTagsOpt`、`m_bTagJumpAnyWhere` | `CShareData_IO.cpp` | 実行中だけ覚える。ファイルの値は壊さず残す | C | M | 既存のキーを使うので、形式の判断は要らない。新しいキーなら要る |
| `sakura.exe.ini`（`MultiUser`、`bIniReadOnly`） | `CShareData.cpp`、`CProfile` | `location.rs` が一部を読む。`bIniReadOnly` は未確認 | ユーザーの判断が要る: 設定ファイルの置き場所 | M | プロファイル |

### ダイアログとプロパティシート

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| プラグインの設定（`CDlgPluginOption`） | `dlg/CDlgPluginOption.cpp` | なし | ユーザーの判断が要る: プラグイン | M | プラグイン |
| プロファイルマネージャ | `dlg/CDlgProfileMgr.cpp` | なし | ユーザーの判断が要る: 設定の置き場所 | M | 同上 |
| ウィンドウの位置と大きさ（`CDlgWinSize`） | `dlg/CDlgWinSize.cpp` | ボタンが灰色 | C | S | ウィンドウ位置の保存 |
| 色統一（`CDlgSameColor`）、強調キーワード選択（`CDlgKeywordSelect`）、タイプ判別（`CDlgTypeAscertain`） | `typeprop/` | なし | C | S〜M | 色の部品 |
| タイプ一覧の追加・削除・複製・上下・インポート・エクスポート・初期化・ファイルの関連付け | `typeprop/CDlgTypeList.cpp` | 「設定変更」「一時適用」だけ（`doc_types/list.rs`） | C | M | 設定の保存形式（`Types(N)`）の扱い |
| ヘルプボタン | 各ダイアログ | 検索・Grep・実行・タグ作成・印刷設定・コントロールコード・ファイルツリーで未対応 | D | S | ヘルプの仕組み（下の「そのほか」） |
| ダイアログのコンボボックスの履歴: タグジャンプのキーワード、Grep のファイル・フォルダー・除外 | `CDlgTagJumpList.cpp`、`CDlgGrep.cpp` | キーワードは直近のみ。Grep のコンボの履歴は #90 の時点で未対応。現状は未確認 | C | S | なし |
| 数値欄のスピンボタン、ラベルのアクセスキー（印刷ページ設定ほか） | `CDlgPrintSetting.cpp` ほか | 文字入力で代用 | C | S | GPUI の部品 |
| ダイアログがモーダルでない（GPUI にモーダルの子ウィンドウがない） | － | 自前のダイアログはモーダルではない（#150） | B | L | GPUI |
| 設定ダイアログの一覧の項目名「…」の参照ボタンの名前（どの欄の参照か分からない） | － | #189 の宿題。直ったか未確認 | C | S | なし |
| Windows の旧式のファイルダイアログ（`bVistaStyleFileDialog` が 0）、ダイアログの下の欄（文字コード・改行の選択、MRU コンボ） | `CDlgOpenFile_*.cpp` | 新式だけ。保存ダイアログの文字コード・改行は #162 で入った。開くダイアログの「文字コード」「読み取り専用で開く」の欄は未確認 | B/C | M | OS ごとのダイアログ |
| UNC への接続（`NetConnect`）、フォルダー履歴 | `CDlgOpenFile.cpp` | していない（#105） | B | S | なし |

### プラグイン

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| プラグインの仕組み: `plugin.def`、ジャック（Outline、SmartIndent、Complement、DocumentOpen/Close/BeforeSave/AfterSave、CommandPlugin）、`F_PLUGCOMMAND`、`F_PLUGIN_LIST` | `sakura_core/plugin/` | なし。`matsu-plugin` クレートは名前に反して、マクロのホスト | ユーザーの判断が要る: ADR（ADR-0007 が「常駐プラグインは別に設計」） | L | ADR |
| プラグインのメニュー・ツールバー・キー割り当て・補完・アウトラインの項目 | `CPluginManager.cpp` ほか | 種別「プラグイン」は一覧が空。補完の候補元にプラグインはない（`support/completion.rs`） | 同上 | M | 同上 |
| プラグインの設定ページとオプション | `CPropComPlugin.cpp`、`CDlgPluginOption.cpp` | 枠だけ | 同上 | M | 同上 |

### 検索と置換、Grep

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| Migemo（インクリメンタル検索、検索ダイアログの Migemo） | `CMigemo.cpp` | なし | ユーザーの判断が要る: ライセンス | M | 止まっているもの |
| Grep の「この編集中のテキストから検索する」（`:HWND:`） | `CDlgGrep.cpp`、`CGrepAgent.cpp` | 未対応（`grep/engine.rs`） | C | M | 別のウィンドウのバッファを読む仕組み |
| Grep と Grep 置換の、改行をまたぐ一致 | `CGrepAgent.cpp` | 行の改行を除いた文字で置換する。改行込みの一致は表示に出るが置換しない（#94）。行の照合は #197・#201 で改行付きにした | A | M | なし |
| Grep の標準出力への出力（`-GOPT=S`）、`GetLongFileName` | `CGrepAgent.cpp` | Windows 固有。していない（#90） | B | S | なし |
| Grep の結果ウィンドウの再描画を止める設定（`m_bGrepRealTimeView`） | `CGrepAgent.cpp` | 設定の部品が灰色。結果は届いた分だけ足す | C | S | なし |
| 検索の「他のビューの条件を引き継ぐ」の切り替え、正規表現ライブラリの指定 | `CPropComSearch` 相当 | 常に引き継ぐ。ライブラリは自前（`matsu-regex`）なので指定は意味がない | B | S | なし |
| 正規表現: 複数行の照合の細部、bregonig との互換テスト（Windows で bregonig.dll を動かした期待値）、一致の中断（Onigmo の fork が要る）、Unicode の表の更新 | `CBregexp.cpp` ほか（読まない。説明書と実物の結果で） | #197 で改行の照合を直した。ほかは未確認 | A | M | bregonig.dll の実物（oracle の CI） |

### ファイルと文字コード

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| 巨大ファイル: ADR-0020 の 2（Grep の巨大ファイル対応）、3（ブロック索引と背景の走査）、4（開く経路、閾値の設定、状態表示、概算の行番号）、5〜8 | － | 1（保存の範囲読み、#214）と 2 の前半（`RangedFile`、#216）だけ入った | 閾値の設定名はユーザーの判断が要る（`[Common] nHugeFileSizeMB` の案。ADR-0020 の 9 章の 2 はユーザーが推奨どおりに決めた。実装で名前を確定する） | L | ADR-0020 |
| ブロックの大きさ（64 KiB か 256 KiB）の実機のディスクでの測定、10GB（S10）・CRLF・Shift_JIS のファイル、ネットワークドライブ、Windows の置き換え・削除 | ADR-0020 の 8.1 | 測っていない（#216） | E | M | 手動の CI ジョブ |
| MS-CP51932 / 20932 | － | 保存できない（許可リストに 3 件） | ユーザーの判断が要る: ユーザーが「後で決める」と言ったまま | M | 止まっているもの |
| ベストフィット表 | `CCodePage.cpp`、`CLatin1.h` | `?` にする（許可リストに 2 件） | ユーザーの判断が要る: 表のデータとライセンス | M | 止まっているもの |
| UTF-7 / ISO-2022-JP の状態つきの変換を呼ぶ側の取り決め、Windows のコードページ全部との突き合わせ、`encoding_rs` にないコードページ（437、850 など） | － | 一部（#29、#53）。未確認 | B | M | なし |
| バックアップ・保存まわり: 所有者・グループ・ACL・代替データストリームの引き継ぎ、その場書き込み、リムーバブルメディア判別（Linux）、ハードリンクでの同一判定 | `CBackupAgent.cpp` ほか | 引き継ぎは #56 で選べる形だけ（依存か FFI が要る）。その場書き込みは別の方式として未実装 | ユーザーの判断が要る: 利用者のデータの保存 | M | 依存（ライセンス） |
| 自動保存 | `CAutoSave` | 入っている（`file_watch/auto_save.rs`） | － | － | ADR-0022 が未保存の文書の自動保存を別に扱う |
| 保存前の確認（`CSaveAgent::OnCheckSave`）、読み込み前の確認（`OnCheckLoad`）の「ロックを一時的に外す」検査 | `CSaveAgent.cpp` | #183 で保存前は入った。読み込み前と一時的な解除は #145 の宿題 | C | S | なし |
| 保存を UI スレッドで行う（検査と書き込みの間、画面が止まる）、進み具合とキャンセル | `CSaveAgent.cpp` | #48 の宿題。直ったか未確認 | C | M | ADR-0020 の PR 6 |

### 表示と編集

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| マウス: Alt+ドラッグの矩形選択、トリプルクリックの行選択、行番号の列のクリックとドラッグの行選択、ルーラーのクリック、Ctrl+クリック、ドラッグ&ドロップ編集（選択範囲をドラッグして移動・コピー）、キー割り当てのマウス欄（ダブルクリック、トリプル、クアドラプル、横ホイール、サイドボタンの割り当て） | `view/CEditView_Mouse.cpp`、`global.h` の `MOUSEFUNCTION_*` | 左クリック（キャレット・Shift で拡張）、ダブルクリックの単語選択、ドラッグ、スクロールバー、中ボタンのオートスクロール、右クリックのメニューだけ（`view/mouse.rs`）。ダブルクリックは固定 | C | L | なし。右ボタンのドロップのメニューは GPUI が押したボタンを伝えない（#123） |
| フリーカーソル、キャレットの形（DOS 風）、OS のキャレット幅 | `CEditView`、`CCaret.cpp` | なし | C | M | 設定をつなぐ |
| 文字の間隔、行の間隔 | `CTextMetrics`、`m_nSpacing` | レイアウトにない（`matsu-layout` の「Not done」） | C | M | レイアウト |
| TSV / CSV モード | `TSV_MODE_*` | なし | C | M | レイアウト |
| 背景画像 | `CTypeSupport`、`CBackground` | なし。設定の部品は灰色 | C | M | 描画 |
| 縦線の位置、行番号の区切り、折り返し桁の縦線（RAP）、行番号とテキストの隙間のノート線 | `CTextDrawer.cpp` | 未対応が混じる（#118）。ミニマップの選択範囲（`DispTextSelected`）も出さない（#205） | C | S | なし |
| アウトラインの解析器: PL/SQL、COBOL、アセンブラ、TeX、Erlang、ルールファイル | `outline/` | 未着手（`matsu-outline` の「Not ported yet」）。開くと「まだ対応していません」と出る | C | M（1 つにつき S〜M） | PL/SQL は ADR-0023 と関係 |
| アウトラインのドッキング位置（左・上・下、フローティング）、幅の変更、窓位置の記憶、並べ替え、右クリックメニュー | `CDlgFuncList.cpp` | 右だけ | C | M | 設定の保存 |
| ブックマークのしるし（縦線）、タブ幅の特別な扱い | `CTextDrawer.cpp:526`、`CFigure_Tab.cpp:57` | 出さない（#205） | C | S | なし |
| 検索の強調色（`CColor_Found`）を HTML コピーに入れる、複数語の検索色（`FN2`〜`FN5`） | `CColor_Found.cpp` | 入れていない（#106、#170） | C | S | なし |
| キーワードヘルプ（辞書の一覧の取り込み・書き出し）、正規表現キーワードのコンパイル検査と確認、インポート・エクスポート | `CPropTypesRegex.cpp`、`CPropTypesKeyHelp.cpp` | キーワードヘルプの機能は #156 で入った。インポート・エクスポートは #140 の宿題 | C | S〜M | なし |
| 補完（`F_HOKAN`）: タイプ別の単語ファイル、強調キーワード、プラグインの候補元 | `CHokanMgr.cpp` | 単語ファイル・強調キーワードは「タイプ設定」の灰色の部品。#133 の時点で未対応 | C | M | 設定をつなぐ |
| メニューの項目名の入れ替え（`F_SHOWMINIMAP_ON` / `_OFF` など）、ショートカットの桁そろえ、アクセスキーのうち gpui-component の限界のもの | `CMainMenu`、`CEditWnd` | 固定の名前。メニュー項目の選択中のキーをステータスバーに出す機能は、gpui-component が通知を出さないので不可（#168） | B | S | gpui-component |
| 折り返し表示のときの矩形ソートの桁 | `CViewCommander_Edit.cpp` | 通常の使い方では関係しない（#194） | A | S | なし |
| 同じ文書を見る複数のビューのカーソルを、文書の変更に合わせて直す仕組み（`TextChange` を使った位置の移動）、改行が混ざった文書で CR と LF がくっつく端の事例 | － | #33、#42 の宿題。#91 以降の分割ビューで直ったか未確認 | A | M | なし |

### 印刷

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| 色刷り（`color_print`）と本文の色 | `CPrint.cpp`、`CPrintPreview.cpp` | 設定は保存するが、色は付けない（`matsu-print` の「Not done here」） | C | M | 3 OS の描画 |
| プリンターの選択（`szDriver`、`DEVMODE`） | `CPrint.cpp` | OS の印刷ダイアログに任せている | B | S | なし |
| ヘッダー・フッターの書式のうち、ウィンドウの状態が要るもの（`$C` `$d` `$t` `$B` `$n` `$S` `$I` `$M` `$<profile>` など） | `CPrint` の `ExpandParameter` | 空のまま（#138）。`$N` と `${R?...$}` は #183 で入った | C | S | なし |
| 大きな文書で割り付けが UI スレッドを止める | － | #47 の宿題。未確認 | C | M | なし |

### OS との結び付き

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| タスクトレイ（常駐、左クリックのメニュー、`-NOWIN`）、トレイのショートカットキー | `CControlTray.cpp` | 設定は灰色。`-NOWIN` は読むだけで何もしない | B | L | GPUI にトレイの API があるか（未確認） |
| 配布: 署名、公証、`.dmg`、Wayland のアイコン（`.desktop`）、Finder の「このアプリで開く」の確認 | － | #198、#215 は途中 | ユーザーの判断が要る: リリース | M | アカウント |
| Linux の HTML クリップボード | `CClipboard.cpp` | テキストとして HTML のソースが載る（#170） | B | S | GPUI |
| macOS のネイティブメニューのアイコン、メニューを開く直前のフック | － | 入れていない（#136、#124）。gpui-pre の限界 | B | S | GPUI |
| ファイルのドロップ: 右ボタンのメニュー、編集ウィンドウの上限 | `CEditView::OnMyDropFiles` | #123 | B | S | GPUI |
| ホイール 1 ノッチの行数のシステム設定 | － | Windows だけ反映（#49） | B | S | GPUI |
| ウィンドウの配置・常に手前（Linux） | `CEditWnd` | OS が許さない（#88） | B | － | OS |

### アクセシビリティ

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| エディタの本文の読み上げ（可視行の TextRun、キャレット、選択） | － | なし（#169、#204）。設計は #204 に出ている（GPUI の `a11y_synthetic_children` と accesskit） | C | L | GPUI |
| ステータスバーのメッセージ部が UI Automation / AX に出ない | － | FD が今のラウンドで直す | C | S | なし |
| ポップアップメニューの入れ物と区切り線に名前がない、項目のチェック・灰色・位置 | － | gpui-component の限界（#177、#204）。報告の下書きは `.claude/handoff/drafts/`。ユーザーが再現してから投稿 | ユーザーの判断が要る: ほかの OSS への投稿 | － | gpui-component |
| 無効なボタン・入力欄の disabled、Table の名前 | － | `Button` と `Input` は gpui-component の限界（#177）。表の名前は未対応（`pending-checks.md`） | B/C | S | gpui-component |

### そのほか

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| ヘルプ（サクラのヘルプファイル）とダイアログのヘルプボタン | `help/sakura/` | `F_HELP_CONTENTS` `F_HELP_SEARCH` はウェブの文書を開く。ヘルプ自体はない | D | M | ヘルプの文書（日本語の docs/ の整理） |
| コマンドラインの `-GROUP=`（タブのグループ指定）、`-NOWIN`、`-DEBUGMODE`、`-PROFMGR` | `CCommandLine.cpp` | 読んで無視する（`command_line.rs`）。`-DEBUGMODE` はタイプだけ変える | `-GROUP`: ADR-0024 / ほか B | S | ADR-0024 |
| コマンドラインの細部: 空白入りのファイル名を探す処理、実行ファイル名の数字による文字コード、使えない文字の確認の箱、`-TYPE` の既定コードの読み込みへの反映 | `CCommandLine.cpp` | 未対応（#127） | B/C | S | なし |
| 起動時のモードの設定（挿入・上書き。`bIsINSMode`） | `CShareData_IO.cpp` | 読まない | C | S | なし |
| 最近使ったファイル・フォルダー: 除外の編集（`ExceptMRU`）、変数展開（`%MYDOC%`）、タイプ ID によるタイプの復元、新しいウィンドウの開始フォルダー、履歴の管理の他のタブ | `CDlgFavorite.cpp` | #95 の宿題。一部は後で入った。未確認 | C | S | なし |
| 一覧のアクセスキーのうち、一覧にフォーカスを移す仕組みが要るもの、ダイアログの表示言語（`szLanguageDll`） | `CPropComKeybind.cpp`、`CSelectLang.cpp` | なし | 言語: ユーザーの判断が要る（ADR） | M | なし |

## 調整役の宿題と今のラウンド

- ステータスバーのメッセージが UI Automation / AX に出ない件は FD が直す。
- 一度きりの移行メッセージが、20 秒の後も次の再描画まで残る件は FD が直す。
- ADR-0020 の次の段階（ブロック索引、開く経路）と、ブロックの大きさの測定は、上の「ファイルと文字コード」にある。
- `pending-checks.md` の「サンドボックスで見つかったこと」: 表（Table）そのものに名前がない件が残っている（上の「アクセシビリティ」）。「見直し」という見出しの項目は、今はない。

## サクラとの違いのうち、わざと合わせないもの（`known_differences.txt`）

直す対象ではないが、行が残る限り比較のテストが許している。

| 件 | 理由 |
|---|---|
| `caret-move-cursor-past-line-end` | サクラの不具合（型 A）。わざと合わせない |
| `savecode-sjis-crlf-to-cp1200` | サクラが UTF-16 の指定を無視する（型 A）。わざと合わせない |
| `regex-jp-nonascii-class`、`convert-code-jis-to-sjis` | サクラの不具合と見て合わせない（2026-10-21 のユーザーの判断）。原因は調べていない |

## 次の PR・ADR との関係

ユーザーが 2026-10-24 に頼んだ 5 つは、[ロードマップ](roadmap.md)の Phase 7 に並べた。この台帳は、そのうちの「サクラの残りの機能」にあたる。

- 未保存の文書の自動保存: ADR-0022（提案中）
- データベース操作（SQL*Plus を含む）: ADR-0023（提案中）
- エディタ以外の機能をタブで: ADR-0024（提案中）
- 言語のスクリプト: ADR-0025（提案中）
