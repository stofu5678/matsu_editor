# サクラエディタにあって、まだ MATSU Editor にない機能（機能台帳）

作成: 2026-10-24（main の 5f81c9f2、リリース 0.2.0 の直後）。サクラは `48db6c7c`（`xtask sakura-tables` が使っている版）。2026-11-01（0.6.0 の後）に、済んだのに残っていた行を直し、勧める順番を次のラウンドの計画に合わせた。第 2 波（#317〜#325）のあと、その PR の「台帳」の節に合わせて行を直した。 2026-11-05 に、#333〜#347 の PR の「台帳」の節に合わせて行を直した（プラグインのジャック 7〜9、巨大ファイル、タイプ別設定のウィンドウ、印刷、保存しない状態）。2026-11-17 に、プラグインの行を、ADR-0026 の残り（外部の言語 #438〜#450、同梱の言語 #499）が入ったのに合わせ、勧める順番を次の波に直した。2026-11-22（main の 228f4b5a、0.23.0 の後）に、全部の行を main のコードと、取り込み済みの PR #438〜#553 の「残っていること」に突き合わせて直した（古くなった行を書き換え、済んだ行を消し、見つけた不足の行を足した。下の「勧める順番」）。

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
| コマンドと関数 | 3 | 関数コードのあるコマンド 477 個のうち未登録は 3 つ（`F_PROFILEMGR` と Migemo の 2 つ。どれもユーザーの判断が要る） |
| マクロ | 1 | `Jump` の画面なしの実行（折り返し行と PL/SQL）、PL/SQL の欄 |
| 設定 | 7 | 共通設定の灰色の部品 15 個とタイプ別設定の灰色の部品 3 個（IME の状態・入力モード・文書アイコンが Windows 以外。型 B）。`sakura.exe.ini` と `bIniReadOnly`（ユーザーの判断が要る） |
| ダイアログ | 6 | プロファイル、タイプ一覧のファイルの関連付け、モーダルでないこと |
| プラグイン | 3 | ADR-0026 の PR は入った。残りは細部（OS が直接終わらせる道と新しいタブの `EditorStart` は型 B、スマートインデントを呼ぶ時機は型 B） |
| 検索 | 3 | Migemo、細部（`:HWND:` の Grep 置換）、一致の中断と Unicode の表 |
| ファイル | 6 | 巨大ファイルの残り（ADR-0020 の追記の「まだ入っていないもの」）、コードページ 51932/20932、ベストフィット表 |
| 表示と編集 | 7 | マウス操作の残り（ウィンドウをまたぐ文字列のドラッグ&ドロップ、右ボタンのドロップ、カーソルの絵。どれも型 B）、フリーカーソルの残り（矩形の角が折り返し行の端の隙間の桁に立たない。型 D。OS のキャレット幅の Mac・Linux 版は型 B）、比例フォントの印刷（ADR-0034 の段階 6）、Mac の既定のフォントが GPUI に読まれない件、アウトラインのフローティング（型 B） |
| 印刷 | 2 | プリンターを名前で指定すること（macOS・Linux。型 B）、macOS の印刷パネルで、先に割り付けていない用紙を選ぶときの割り付け（UI スレッド。型 B） |
| OS との結び付き | 7 | タスクトレイ、署名、Wayland のアイコン、Linux の HTML クリップボード |
| アクセシビリティ | 3 | ポップアップメニュー、本文へのキーボードのフォーカス、`Button`・`Input` の disabled、ドロップダウンの箱の中のボタン（いずれも gpui-component と GPUI の限界） |
| そのほか | 3 | `-GROUP`・`-NOWIN` などを読んで捨てる理由、コマンドラインの細部、ダイアログの表示言語 |

### 次のラウンドに勧める順番

2026-11-01 の計画の第 1 波と第 2 波、プラグインの続き（ADR-0026 の PR 10）、2026-11-17 に選んだ第 3 波（印刷のプリンターの選択、マクロの `ExpandParameter` の `$Q`（#508）、大きな文書の割り付け（#509・#530）、折り返し表示の矩形ソート（#503））と、そのあとの第 13 波（2026-11-25）までは、すべて入った（`:HWND:` の Grep 置換の差分の当て直しは、作らないと決めた）。2026-11-22 の突き合わせで、ユーザーの判断も ADR も要らず、型 B でもない行は次のとおり（価値の高い順。サクラの利用者が気づきやすいものが先）。

| 順 | 項目 | 大きさ | 台帳の表 |
|---|---|---|---|
| 1 | 矩形の角を、折り返し行の端の隙間の桁に立てる（角の作りを変える。型 D） | M | 表示と編集 |
| 2 | 巨大ファイルの残り（行数の表を持たないレイアウト、長い行の IME。ADR-0020）、ブロックの大きさの測定（型 E） | L / M | ファイルと文字コード |

ユーザーの判断が先に要る（上の表の外）: Migemo、プロファイルと `sakura.exe.ini`・`bIniReadOnly`、ファイルの関連付け、表示の言語、ベストフィット表、CP51932/20932、署名、比例フォントの印刷（ADR-0034 の段階 6）、gpui-component への要望（`Radio` に呼ぶ側からフォーカスを渡す口。いまは内部の鍵で引いている。#577。ほかの OSS への投稿）。タスクトレイとダークモードは、GPUI でできるかを先に調べる（型 B）。「止まっているもの」に理由を書いた。

## 止まっているもの

| 項目 | 理由 |
|---|---|
| Migemo（`F_ISEARCH_MIGEMO_NEXT` `F_ISEARCH_MIGEMO_PREV`、共通設定「支援」の DLL と辞書、検索の Migemo 使用） | サクラは `cmigemo` の DLL を実行時に読み、辞書は利用者が置く（`sakura_core/extmodule/CMigemo.cpp`）。C/Migemo 本体は MIT License（koron/cmigemo の LICENSE、GitHub が MIT と判定）。ただし通常の辞書は SKK-JISYO.L から作り、その README に「作った辞書は SKK-JISYO.L のライセンス（GPL）に従う」とあり、SKK-JISYO.L の先頭は GNU General Public License（skk-dev/dict）。したがって同梱・組み込みはできない。選択肢は、(a) DLL と辞書を利用者が置く形（サクラと同じ。同梱しない）にする、(b) 組み込まない。どちらもライセンスの判断なので、ユーザーに聞く。いまの `ISearchMode` は後から足せる形になっている。共通設定「支援」の Migemo の DLL と辞書の 4 個の部品は灰色 |
| プロファイル（`F_PROFILEMGR`、`-PROFMGR`、`CDlgProfileMgr`、`sakura.exe.ini` の `MultiUser`） | `-PROF=名前` は入っている（`launch.rs`、`matsu-config/src/location.rs`）。残りは、設定ファイルの置き場所を変える・利用者のデータの形式に触れる（`matsu.ini` の場所）ので、ユーザーの判断が要る |
| IME の再変換（`F_RECONVERT`） | Windows は IMM32 で実装した（#232）。macOS と Linux は、プログラムから再変換を始める API がない（macOS は入力メソッドが `NSTextInputClient` で周りの文字を取りに来る。IBus / Fcitx は周りの文字を渡す口だけ）。型 B: ステータスバーに理由を出す。Windows の IME 実機での確認は残る |
| SQL*Plus（`F_ACTIVATE_SQLPLUS` `F_PLSQL_COMPILE_ON_SQLPLUS`）と PL/SQL のジャンプ | ADR-0023（決定）で、接続ごとのクライアントを動かす形にした。実行と出力パネル（`databases.ini`。メニューの文言は「データベースで実行」「データベースの出力を表示」で、接続を選んで実行する「接続を選んでデータベースで実行...」もある）、エラー行からの移動（クライアント 7 種の形。実物では未確認）、SQL の色分けのタイプ（汎用 + 方言 5 つ。`.sql` は汎用）、サンプルのマクロ（`crates/matsu-plugin/samples/sample_db.*`）は入った。残りは、sqlcmd と SQL*Plus の行のずれ、接続の設定の画面、PL/SQL のアウトラインからのジャンプ |
| ベストフィットの表（全角英数字を ASCII にする Windows の変換） | 表の元データと、そのライセンスの判断が要る（`known_differences.txt` の `saveloss-jp-to-*`） |
| MS-CP51932 / 20932 | ユーザーが「後で決める」と言ったまま（#210）。保存すると何も書かず止まる（`savecode-sjis-crlf-to-cp51932` など 3 件） |
| 署名（macOS の公証、Windows の署名）、`.dmg` | Apple のアカウントなど。リリースなのでユーザーの判断 |

## 領域ごとの表

「判断」の列は、ADR-0018 の型。ユーザーの判断が要るものは「ユーザーの判断が要る: 理由」と書く。「大きさ」は、1 本の PR にしてよいものが S、1 本で収まるが大きいものが M、分けるものが L。

### コマンドと関数

関数表（`crates/matsu-commands/src/sakura/functions.rs`）の「コマンド」477 個のうち、`SAKURA` 定数のあるコマンドは 474 個（2026-11-22 に、`kind: Command` の名前と `const SAKURA` の文字列を突き合わせて数えた）。差の 3 個が未登録で、次のとおり（SQL*Plus の `F_ACTIVATE_SQLPLUS` `F_PLSQL_COMPILE_ON_SQLPLUS` は ADR-0023 で実装した。残りは「止まっているもの」）。ほかの種類（`MacroFunction` 59、`MacroOnly` 23）は、次の「マクロ」の表で。

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| `F_PROFILEMGR` | `CViewCommander_File.cpp` | コマンドなし | ユーザーの判断が要る: 設定ファイルの置き場所（利用者のデータ） | M | `CDlgProfileMgr` |
| `F_ISEARCH_MIGEMO_NEXT` | `CViewCommander_Search.cpp` | コマンドなし | ユーザーの判断が要る: ライセンス（止まっているものを参照） | M | 同上 |
| `F_ISEARCH_MIGEMO_PREV` | 同上 | 同上 | 同上 | 同上 | 同上 |

### マクロ

マクロの細部で直したもの（ラウンド 23）: `GetLineAttribute` の種類 3（差分の印。0 なし、1 追加、2 変更、3 削除、4 EOF 以降の削除）と 4（アウトラインの印）。`HalfPageUp(n)` `HalfPageDown(n)` `PageUp(n)` `PageDown(n)`（選択・矩形の型も）の行数引数（0 以下は既定）。入れ子の `.mac` が終わるとき、`AddRefUndoBuffer` の深さを始める前の値に戻す（`ExecKeyMacroAfter`）。BOM なしの `.mac` は Shift_JIS として読む（`CTextInputStream`）。`ChgmodEOL` の第 2 引数は、サクラの表（`CSMacroMgr.cpp` 304 行）に無く、実在しない。`F_MERGE` の矩形選択は、すでにサクラどおり何もせず戻る。

`MacroFunction` 59 個と `MacroOnly` 23 個の名前は、すべてコードのどこかにある。さらに、マクロの名前の表の結果のある行 62 個すべてに、プラグインのホスト（`matsu-plugin/src/functions/`）の実装と、その単体テストがあることを確かめた。サクラとの比較（oracle）のケースは別に 727 件ある。次は、実装があっても一部だけのもの。

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| `Jump` のオプション 0（折り返し行）と 2（PL/SQL）は、ウィンドウで動くマクロでは入った（`F_JUMP` のコマンド `matsu-commands/src/commands/cursor/jump.rs`。表示のとおりの行で数える。省略は 1）。ジャンプのパネルには「折り返し単位の行番号」「改行単位の行番号」の選択が入った（`ui_commands/search/jump_dialog.rs`、`search/panel.rs`。開くときは前の選択（既定は改行単位）で始まり、ジャンプで `[Other]` の `bLineNumIsCRLF` に書く。キャンセルでは書かない）。残りは、画面なしの実行（`matsu-plugin/src/api.rs` の `jump`。レイアウトがないので、0 と 2 はエラー。折り返しは画面の幅で決まるため）と、PL/SQL の欄（SQL*Plus は ADR-0023）。パネルのジャンプは常にキーマクロに `Jump(行, オプション)` として記録され（オプションの 0x01 が改行単位。PL/SQL の 0x02 は欄がないので 0。`CMacro.cpp` 161-165）、マクロの `Jump` は、パネルの単位の選択を書き換える（オプション省略は 1。`CMacro.cpp` 719-741。`macros/wait.rs`。`matsu.ini` にはパネルのジャンプと同じ `bLineNumIsCRLF` に書く） | `CMacro.cpp`、`CDlgJump.cpp` | 画面なしの実行はエラーのまま。 | C | S | PL/SQL の SQL*Plus は ADR-0023 |

### 設定

画面に出ていて、効かない部品（`Control::unavailable()`、コードでは「未対応」の灰色）は、共通設定 15 個（「全般」5、「ウィンドウ」2、「編集」1、「検索」1、「マクロ」2、「支援」の Migemo 4）、タイプ別設定 3 個ある（IME の状態・入力モード・文書アイコン。Windows だけで効く。型 B）。サクラのキー（`CShareData_IO.cpp` の `IOProfileData` の 301 キー）のうち、キー名の文字列が `crates/` のコードに出てこないものは 26 個（2026-11-22 に 30 個と数え、第 9 波（2026-11-24）で `bAutoCloseDlgFind`・`bAutoCloseDlgReplace`・`bAutoColmnPaste`・`nRepeatedMoveCaretNum` を読むようにした）。そのうち連番や別の形で組み立てるもの（`nKeywordSelect4`〜`10`、`rc*Dialog` の 5 個、`_MRU_Counts` `_MRUFOLDER_Counts` `ExceptMRU`）は動いている。本当に読んでいないものは、`bAutoCloseDlgFuncList`、`bFunclistSetFocusOnJump`、`bDispExitingDialog`、`bIniReadOnly`、`bIsINSMode`、`wTrayMenuHotKeyCode` `wTrayMenuHotKeyMods`、`szLanguageDll` で、下の行に書いた。設定ファイルの値は、読み書きは壊さずに残している（ADR-0010）ので、「動かない」だけで「消える」ことはない。

`bIsINSMode`（起動時の挿入・上書き）は、サクラ自身が読み書きしていない（`CShareData_IO.cpp` の `ShareData_IO_Common` でコメントアウト、2005-10-02 genta「初期値を挿入モードに固定する」）。画面の部品も無い。MATSU もいつも挿入で始まるので、不足ではない。

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| 共通設定「編集」の灰色 1 個: Vista スタイルのファイルダイアログ（GPUI の Windows 版は常に Vista スタイルの `IFileOpenDialog` を使うため、切り替える先が無い。型 B） | `CPropComEdit.cpp`、`CommonSetting.h` | コピー・上書きモードの設定はコマンドが読む（`edit_options`、`context/clipboard.rs`、`context/mod.rs` の `type_text`、`context/box_edit.rs`）。ドラッグ&ドロップと URL の選択は #236 でつながる。「矩形選択移動で選択をロックする」（`m_bBoxSelectLock`、#541）は、サクラが読む `Sub_BoxSelectLock` だけにつないだ（`context/boxsel.rs` の `ensure_box`。マウスは読まない: Alt のドラッグはいつもロックなし）。`_BOX` の移動のマクロの第 1・2 引数（ロックの 1・2。`Sub_BoxSelectLock`）と、記録時に 0 を 1・2 にすること（`CMacro.cpp` 134 行。`key_macro.rs` の `with_recorded_box_lock`）、矩形コピーのテキストを常に矩形貼り付けにする `bAutoColmnPaste`（サクラの画面に部品はなく、ini だけ。`edit_options` の `auto_column_paste`、`Paste` の 0x10・0x20 は前から読む）はつないだ。残りは灰色の 1 個と、次の 2 つ。(1) ページ移動の `_BOX` を記録するとき、サクラは行数の 0 を画面の行数に直して書く（`CMacro.cpp` 108〜122 行）が、MATSU は 0 のまま書く（再生のときの画面の行数で動く。型 D: 記録の側に画面の行数を渡す道がない）。(2) 全置換の「クリップボードの内容」で矩形コピーを矩形に貼る分岐（`CViewCommander_Search.cpp` 892 行）は、MATSU の置換にクリップボードの置換がまだないので、`bAutoColmnPaste` を読む所がない（型 D。マクロの `Replace` の 0x40 と同じ理由） | B/D | S | なし |
| 共通設定「全般」の灰色 5 個: タスクトレイ（使う、常駐、ショートカットキー）、少し滑らかにする（途中の再描画だけの違いで、GPUI ではコマンドの途中に描けない。動きの結果は同じ。型 B）、画面キャッシュ（GPUI が描くので、持つ対象が無い。型 B） | `CPropComGeneral.cpp` | ホイールの行数・ページスクロール・横スクロールの組み合わせ（`view/paging.rs`）、全終了と複数ウィンドウを閉じる確認（`window_ops`、`file_ops/close.rs`）、押しっぱなしの上下キーの行数と左右キーの桁数（`view/paging.rs` の `key_repeat_count`。左右の桁数は `nRepeatedMoveCaretNum`、既定 2。ini だけのキーで、共通設定の「全般」が見えない部品に持つ）、フリーカーソルとキャレットの形（下の「フリーカーソルの残り」）、マウスクリックでのアクティブ化でキャレットを動かさない（`view/mouse.rs`。GPUI の Windows 版は最初のクリックを教えないので、ウィンドウが有効になった直後 150 ミリ秒以内の押下をそう扱う。macOS と Wayland は `first_mouse` を使う。型 B）はつないだ。ホイール 1 目盛りの行数は OS の値のまま（サクラもレジストリの値）。残りは灰色と、終了中ダイアログ `bDispExitingDialog`（既定 off。ini だけ。`CControlTray.cpp` 1754 行は、トレイのプロセスが設定を保存する間に出す箱）は、トレイがないので出さない（型 B） | A/C | M | トレイは GPUI の API |
| 共通設定「ウィンドウ」の灰色 2 個: ダークモード、言語 | `CPropComWin.cpp` | ルーラーの高さ・隙間と行番号の隙間（`settings/view_settings.rs`、`view/element.rs`）、位置と大きさの設定のダイアログ（`win_size_dialog/`）、水平スクロールバー（`view/hscroll.rs`。範囲・ページ・矢印・つまみは `CEditView_Scroll.cpp` どおり）、縦のバーの矢印・矢印とトラックの押し続けの繰り返し（`view/vscroll.rs`、`view/scroll_repeat.rs`）、右下の角と分割ボックス（`view/split_box.rs`、`panes/mod.rs`）はつないだ。残りは灰色 | C | M | 言語は ADR 要（利用者向けの文言の多言語化） |
| 共通設定「検索」の灰色 1 個（正規表現ライブラリ `szRegexpLib`。MATSU は自前の正規表現エンジン `matsu-regex` を使うので指定先がない。設定ファイルの値は残す。型 B）。「マクロ」の Python フォルダー（2 個。マクロのホストが Python を自分で探す。ADR-0007）。「支援」の Migemo の DLL と辞書（4 個。「止まっているもの」の Migemo） | `CPropComSearch.cpp` ほか | ツールバーのフラットなボタン（`bars/toolbar.rs`。フラットでないときは、ボタンをいつも枠付きで描く。型 B）、Grep のリアルタイム表示、Enter とダブルクリックのタグジャンプ、マクロのキャンセル確認の待ち時間（スクリプトの開始から待ってから、箱で聞く）、「他のビューの検索条件変更を引き継ぐ」（`m_bInheritKeyOtherView`。次・前検索の頭で `ChangeCurRegexp` と同じに、新しい検索文字列とオプションを取る。検索ボックスの表示も同じ）はつないだ。残りは灰色 | B | S | なし |
| タイプ別設定「ウィンドウ」の灰色: IME の状態と入力モード、文書アイコン（Windows 以外） | `CPropTypesWindow.cpp` | 文字コードの既定、CP、CESU-8 優先、折り返し単位、桁数、縦線、背景画像（パス、位置、繰り返し、スクロール、透明度）は入った。IME の状態・入力モードと文書アイコンは Windows だけで効き、macOS・Linux は灰色のまま（型 B。理由は画面に書いた） | B | － | なし |
| 設定ファイルに保存しない状態（残り）: 位置を持てないダイアログの位置（`rcOpenDialog`、`rcDiffDialog`、`rcCompareDialog`、`rcTagJumpDialog`、`rcWindowListDialog`。OS 標準のダイアログか、編集ウィンドウの中のパネル。型 B）。ほかの状態は保存する（検索の条件 `bSearch*`、Grep・Grep 置換の選択肢 `bGrep*` `nGrepCharSet`、DIFF・タグファイルの作成・タグジャンプ一覧の選択肢、「履歴とお気に入りの管理」の位置 `rcFavoriteDialog`、アウトラインの窓の位置と大きさ、ウィンドウの位置と大きさ。`saved_state/`、`outline/dock.rs`。#251・#314・#315・#344） | `CShareData_IO.cpp`、`CEditWnd.cpp` | 位置を持てないダイアログの位置は、実行中だけ（`rc*Dialog` は ini の値を残す） | B | － | なし |
| `sakura.exe.ini`（`MultiUser`、`bIniReadOnly`） | `CShareData.cpp`、`CShareData_IO.cpp` 193 行、`CProfile` | どちらも読まない（2026-11-22 に確かめた）。`location.rs` は、サクラが設定ファイルを探す規則（`MultiUser` など）を説明に書いているだけで、`matsu.exe.ini` は読まない。`-PROF=` の名前だけが使われる。`bIniReadOnly`（ini だけで、画面に部品はない）は、コードに出てこない。サクラは、これが 1 のとき終了時に ini を書かない。MATSU は、いつも書く | ユーザーの判断が要る: 設定ファイルの置き場所と、書く・書かない（利用者のデータ） | M | プロファイル |

### ダイアログとプロパティシート

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| プロファイルマネージャ | `dlg/CDlgProfileMgr.cpp` | なし | ユーザーの判断が要る: 設定の置き場所 | M | 同上 |
| タイプ一覧のファイルの関連付け（右クリックメニュー・ダブルクリックで開く） | `typeprop/CDlgTypeList.cpp` | 追加・削除・複製・上下・インポート・エクスポート・初期化は入った（`doc_types/`、`matsu-config` の `type_list.rs` `type_file.rs`）。関連付けは Windows のレジストリに書くので未対応 | ユーザーの判断が要る: 利用者のシステムを変える | S | なし |
| ヘルプボタン | 各ダイアログ | 入った（`MyWinHelp` のあるダイアログ。ボタンの「ヘルプ(H)」、Alt+H、F1 で `support/help.rs` の `PAGES` の頁を開く）。残り: ファイルのプロパティ（OS のメッセージ箱で、ボタンを足せない）、アウトライン解析のダイアログ（MATSU はドックのペインで、ボタンがない）、コントロールごとのポップアップヘルプ（`WM_HELP`。コントロールの説明の表がなく、F1 はダイアログの頁を開く） | B | － | なし |
| ダイアログがモーダルでない（GPUI にモーダルの子ウィンドウがない） | － | 自前のダイアログはモーダルではない（#150） | B | L | GPUI |
| Windows の旧式のファイルダイアログ（`bVistaStyleFileDialog` が 0）、ダイアログの下の欄（文字コード・改行の選択、MRU コンボ） | `CDlgOpenFile_*.cpp` | 新式だけ。保存ダイアログの文字コード・改行は #162 で入った。開くダイアログの「文字コード」「読み取り専用で開く」の欄は未確認 | B/C | M | OS ごとのダイアログ |
| UNC への接続（`NetConnect`）、フォルダー履歴 | `CDlgOpenFile.cpp` | していない（#105） | B | S | なし |

### プラグイン

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| プラグインの仕組み: `plugin.def`、ジャック（Outline、SmartIndent、Complement、DocumentOpen/Close/BeforeSave/AfterSave、CommandPlugin）、`F_PLUGCOMMAND`、`F_PLUGIN_LIST` | `sakura_core/plugin/` | ADR-0026 の PR 1〜3 で、`plugin.def` の読み込み、有効にする前の確認と変更の検出、常駐のホスト（`plugin/invoke`、遅延起動、時間の制限、3 回失敗で停止）が入った（#275、#279、#282）。`Command` ジャック（PR 4）も入った（`F_PLUGCOMMAND`、`F_PLUGIN_LIST`、`Plugin.AddCommand`。下の行）。イベントのジャック 6 つ（PR 6）も入った。イベントは、閉じるすべての道（タブの ×、タグジャンプ、`Exit`、すべて終了）と、Grep の起動（`EditorStart`、`DocumentOpen` は Grep のあと。`-GREPDLG` はダイアログを閉じたあと、取り消しでも呼ぶ。OK のときは先に Grep コマンドの `DocumentOpen` が 1 回入る。窓の中に結果を書く Grep コマンドは、検索のあとに `DocumentOpen` を呼ぶ）で呼ぶ。閉じかけのウィンドウが残ったときは `EditorStart` を出し直す（#337）。`Outline` ジャック（PR 7）も入った（`Outline` オブジェクト、タイプ別設定の選択肢、10 秒の制限、失敗したときは空の一覧と理由）。補完のジャック（PR 8）も入った（`ComplementGlobal` と、タイプ別「支援」で選ぶ `Complement`。`plugin_complement.rs`）。スマートインデントのジャック（PR 9）も入った（常駐のプラグインだけ、改行と `SmartIndent.Chars` の文字で呼ぶ。タイプ別設定の選択肢に出る）。Grep の結果を新しい窓に書くときは、サクラが別プロセスの `-GREPMODE` として呼ぶのと同じく、検索が終わってから、その窓の `EditorStart`、`DocumentOpen` を 1 回ずつ呼ぶ（`grep/window.rs`。タブに書くときは、窓が `EditorStart` を受けているので `DocumentOpen` だけ。マクロやプラグインが始めた Grep には、窓の中に書く場合と同じく `DocumentOpen` を出さない）。マクロ（キーマクロとスクリプト）が保存や読み込みをする間も、そのイベントを配る（マクロがその流れを待っている間は、マクロは窓に触れないので、イベントの呼び出しが窓を取る。`macros/plugin_call.rs` の `Turn`、`MacroState::owns_window`。マクロが呼び出しを実行している最中に起きるイベントは配らない。マクロの `FileOpen` が窓の中で読む道は `DocumentOpen` を呼ぶ。サクラは、マクロの中から入れ子で呼ぶ。`CDocFileOperation.cpp`）。残り: OS が直接終わらせる道（Dock の終了、ログアウト）ではイベントを呼ばない（型 B）。Grep の結果を新しいタブに書くときは `DocumentOpen` だけを呼ぶ（MATSU は `EditorStart` を窓に 1 回だけ呼ぶ。サクラはタブも別プロセスなので呼ぶ。型 B。#550）。マクロが呼び出しの最中に起こすイベント（マクロの Grep など）は配らない。マクロの `FileOpen` が窓の中で読むときは、オープン後のマクロを動かさない（#573 の前から。サクラの動きは確かめていない）。ADR-0026 の PR 10 は、外部の言語のブート（#438・#444・#445・#448・#450）と、同梱の Lua・MicroPython・mruby（#499）で入った | 細部ごと（B: OS が直接終わらせる道、新しいタブの `EditorStart`） | S | なし（ADR-0026 の PR は入った） |
| プラグインのメニュー・ツールバー・キー割り当て・補完・アウトラインの項目 | `CPluginManager.cpp` ほか | 有効なプラグインの `[Command]` と `Plugin.AddCommand` のコマンドを、実行時に `F_PLUGCOMMAND` の番号（サクラの割り当て: `CPlug::GetPluginFunctionCode`）で登録する。種別「プラグイン」（キー割り当て・メインメニュー・カスタムメニュー・ツールバー）の一覧、`F_PLUGIN_LIST`（プラグインごとの副メニュー）、保存した `<ID>/NN` の読み書きが入った。コマンドのアイコン（BMP）は入った（ADR-0026 の PR 11。ツールバーとメニュー。`plugin.def` と `AddCommand` の指定）。`EditorStart` で `AddCommand` するプラグインのコマンドは、ウィンドウができたときに現れ、保存済みのキーとボタンが効く。メインメニュー・カスタムメニュー・キー割り当ては、サクラの `GetPlugCmdInfoByName` と同じく、プラグインの表にある ID なら有効でなくても番号を読む。有効でないプラグインの項目は、サクラと同じ「-- undefined plugin command --」の灰色の行になり、キーの割り当てはキー割り当てのページで保存しても消えない（コマンドを実行すると、有効でないと状態バーに出る）。ツールバーは、読み込まれていないコマンドを「未定義のボタン」で見せる（サクラは `-1` を読んで書き戻し、保存でボタンが消える。ここは名前を書き戻す。型 A）。設定を OK で閉じると、開いているウィンドウのキー・ツールバー・メニューが作り直される。残り: なし（下の細部と、ADR-0026 の差だけ）。アウトラインの項目は入った（タイプ別「スクリーン」のアウトライン解析方法に、動くプラグインの `Outline` が並ぶ。値はプラグの関数コード）。補完の候補元は入った（タイプ別「支援」の `nHokanType`。無効なプラグインを指す値も保つ）。差: 補完の絞り込みでプラグインを呼び直さない（呼ぶのは補完を開くときの 1 回。ADR-0026 で決定済み） | ADR-0026 | M | ADR-0026 |
| スマートインデントのプラグインを呼ぶ時機 | `CPluginManager.cpp`、`CType_Cpp.cpp` | サクラが文字ごとに同期で呼ぶのに対して、MATSU は改行と宣言した文字（`SmartIndent.Chars`）だけ、別プロセスで、コマンドのあとに呼ぶ。マクロの再生中は呼ばない（型 B。ADR-0026） | B | － | ADR-0026 |

### 検索と置換、Grep

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| Migemo（インクリメンタル検索、検索ダイアログの Migemo） | `CMigemo.cpp` | なし | ユーザーの判断が要る: ライセンス | M | 止まっているもの |
| Grep 置換のまわりの細部（置換の頭の `ChangeCurRegexp` は入った。キーやマクロの `F_REPLACE` `F_REPLACE_ALL` も、頭で他のウィンドウの新しい検索条件を取る）: `:HWND:` の Grep 置換（#504）は、サクラにない動き（サクラは窓のテキストを検索するだけで、「置換しました」と数だけ書く。`CGrepAgent.cpp` の `DoGrep` は窓では `DoGrepFile` を呼び、`DoGrepReplaceFile` を呼ばない）。検索中にその文書が変わったら何も置換せず、巨大ファイルを読み込み中の文書は断る。変わった文書への差分の当て直しは作らない: サクラに手本がなく（窓の行を読む間に文書が変わっても確かめない）、検索した時点の文字に対して計画した置換を、変わった文書へずらして当てると、利用者の編集を壊すおそれがあるため（型 B。やり直しは利用者が行う） | `CViewCommander_Search.cpp`、`CGrepAgent.cpp` | 左のとおり | C | S | なし |
| 正規表現: 一致の中断（Onigmo の fork が要る）、Unicode の表の更新 | `CBregexp.cpp` ほか（読まない。説明書と実物の結果で） | 複数行の照合は oracle で確かめた（`regex-ml-*` の 134 件（#548）: 行頭・行末・文書の末尾の空の一致、`\r\n`・`\R`・`\s`・`.` と改行、行をまたぐ一致、改行を足す・消す置換、CRLF・LF・CR・混在・空の文書、日本語）。違いは 1 件だけで、改行の直後（照合する文字列の末尾）での先読み・後読みの一致を見つけなかったのを直した（#197 のあと）。残りは、一致の中断（Onigmo の fork が要る）と Unicode の表の更新（Onigmo 自身の表） | A | M | Onigmo の fork（中断）、Onigmo 自身の表（Unicode） |

### ファイルと文字コード

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| 巨大ファイルの残り（ADR-0020 の追記の「まだ入っていないもの」）: 行数の表を持たないレイアウト（6 章の B）、長い行での IME と行単位のコマンド（並べ替え、インデント、段落、箱の編集、貼り付けなど）、10 GB の測定、Grep 置換の巨大ファイル | － | 保存の範囲読み（#214）、`RangedFile`（#216）、ブロック索引と背景の走査（#259）、範囲ごとに開く経路（#276）、走査の後の起動位置（#281）、レイアウト・色分け・アウトラインの制限（#283）、Grep と `:HWND:` の範囲読み、diff の断り、マクロの走査中のエラーと選択範囲の上限（#334）、走査していない場所への概算の移動とファイル全体を指すスクロールバー（#339）、保存後の Undo の世代（UTF-8 と Latin1 のとき。`SaveReport.reloaded`）、保存前の空き容量の確認（#353。サクラにない MATSU 独自の動作で、ADR-0005 の一時ファイル方式のため）、別のスレッドでの保存と進み具合・中止（#519）、バックアップのコピーを別のスレッドですること（#542）、`:HWND:` の Grep 置換が巨大ファイルを読み込み中の文書を断ること（#504）、クリップボードへのコピーと切り取りの 256 MiB の上限（選択範囲・行番号つき・色付き HTML・選択のない行。切り取りは何も消さない）、1 MiB 以上の長い行を丸ごとデコードしていた所（Grep は 64 MiB を超える行を飛ばして結果に書く、`GetLineStr` は 256 MiB 以上を断る、`$x` と検索ダイアログの文字列とファイル内容比較は一部だけ読む、箱選択の貼り付けは索引だけで改行を調べる）は入った。折り返す設定の表示の不具合は、走査のあとの文書が折り返すことを試験で確かめ（2026-11-25）、Windows Sandbox でも再現しなかった（調整役、同日）ので、行から消した | ADR-0020 | L | ADR-0020 |
| ブロックの大きさ（64 KiB か 256 KiB）の実機のディスクでの測定、10GB（S10）・CRLF・Shift_JIS のファイル、ネットワークドライブ、Windows の置き換え・削除 | ADR-0020 の 8.1 | 測っていない（#216） | E | M | 手動の CI ジョブ |
| MS-CP51932 / 20932 | － | 保存できない（許可リストに 3 件） | ユーザーの判断が要る: ユーザーが「後で決める」と言ったまま | M | 止まっているもの |
| ベストフィット表 | `CCodePage.cpp`、`CLatin1.h` | `?` にする（許可リストに 2 件） | ユーザーの判断が要る: 表のデータとライセンス | M | 止まっているもの |
| UTF-7 / ISO-2022-JP の状態つきの変換を呼ぶ側の取り決め、Windows のコードページ全部との突き合わせ、`encoding_rs` にないコードページ（437、850 など） | － | 一部（#29、#53）。未確認 | B | M | なし |
| バックアップ・保存まわり: 所有者・グループ・ACL・代替データストリームの引き継ぎ、その場書き込み、リムーバブルメディア判別（Linux）、ハードリンクでの同一判定 | `CBackupAgent.cpp` ほか | 引き継ぎは #56 で選べる形だけ（依存か FFI が要る）。その場書き込みは別の方式として未実装 | ユーザーの判断が要る: 利用者のデータの保存 | M | 依存（ライセンス） |

### 表示と編集

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| マウスの残り: 他のウィンドウや他のアプリとのドラッグ&ドロップ（ドロップ元にもドロップ先にもなる。ほかのアプリの文字列を落とす）、右ボタンのドロップのメニュー、カーソルの絵（行番号の列の右向きの矢印、インクリメンタルサーチ中、オートスクロールの向き） | `view/CEditView_Mouse.cpp`、`global.h` の `MOUSEFUNCTION_*` | 窓の中の操作は入った: Alt+ドラッグの矩形選択（行番号の列を Alt で押すと 1 行下まで広げる）、ダブル・トリプル・クアドラプルクリックと 2.5 クリック（キー割り当ての行を読む）、行番号の列のクリックとドラッグ、ルーラーのクリック（何もしない）、Ctrl+クリックの単語選択、URL のクリックでの選択とダブルクリックで開く（正規表現キーワードで URL 色にした文字列も）、窓の中のドラッグ&ドロップ編集（Ctrl でコピー、Esc でやめる、端に寄るとスクロール）、カーソルの形（URL の上は手、選択範囲の上・行番号の列・ルーラー・スクロールバーは矢印、ドラッグ中は矢印、Ctrl でコピーの印）、サイドボタン、ホイールとサイドボタンによるページスクロール・横スクロール（`view/mouse.rs`、`view/drag_cursor.rs`、`view/paging.rs`、`context/mouse.rs`、`context/mouse_drop.rs`）。ウィンドウ・アプリをまたぐ文字列のドラッグは、GPUI が外へ出せる中身がファイルのパスだけ（`ExternalDragPayload::Files`、受け側も `FileDropEvent` のパスだけ）で、文字列を渡せない（型 B）。カーソルの絵は GPUI が絵を読み込めないので、行番号の列は普通の矢印で代える（型 B）。Ctrl でコピーの印（`DragCopy`）は、GPUI の Windows 版が普通の矢印に割り当てていて（`gpui-pre-windows` 0.3.8 の `src/util.rs`）、Windows では出ない（型 B。macOS と Linux には出る） | C | M | GPUI。右ボタンのドロップのメニューは GPUI が押したボタンを伝えない（#123） |
| フリーカーソルの残り: OS のキャレット幅の Mac・Linux 版（その OS に設定が無い。型 B） | `CCaret.cpp`、`CViewCommander_Cursor.cpp`、`CViewCommander_Edit.cpp` | 入れた: フリーカーソル（`context/free_cursor.rs`: 上下で桁を保つ、右左で隙間を 1 桁ずつ通る、クリックで置く、入力・貼り付け・改行・Tab は隙間を空白で埋めてから入れる、`F_DELETE` は埋めてから改行を消す、`F_DELETE_BACK` は 1 桁戻るだけ）、キャレットの形（`view/caret_shape.rs`: Windows 風の細い棒・MS-DOS 風の半分の高さの箱、上書きは箱）、Windows の OS のキャレット幅（`matsu-platform` の `caret_width`）、全般ページの 3 つの設定（`bIsFreeCursorMode`、`nCaretType`）、マウスのドラッグ中のキャレット（隙間にも置く）、右端の例外と隙間の右端を折り返し桁にする（`Layout::max_line_x`、`GetMaxLineLayout`）、折り返し行の端の隙間（クリックと上下で入り、左右・`F_DELETE`・`F_DELETE_BACK`・入力はほかの隙間と同じ。サクラは折り返し行の端も行の端と同じに扱う: `MoveCursorProperly` と `Cursor_UPDOWN` の `i >= nLineLen`。`Pos` は折り返しの端を「次の行の先頭」としか持てないので、隙間の桁（`Cursor::free_col`）を印にし、行の頭にある桁が前の行の端より右（または端）なら、前の行の隙間とする。`context/free_cursor.rs` の「折り返し行の隙間」）。モードを切っていても、折り返し行の端（文字の後ろ）はキャレットの居場所とする（`End`、行の右のクリック、上下で目標の桁が行より右のとき。同じ印を使い、モードを切っていると、文字の端の桁だけを取り、右の隙間は取らない。`F_RIGHT` は次の行の頭へ一度に行く。「折り返し行の端」の `context/free_cursor.rs`）。隙間のキャレットから、行単位のコマンド（行削除・行の複製・行の切り取り・現在行のコピー・行モードの貼り付けなど）は見えている段に効く（#572）。IME の候補の窓（`input.rs` の `point_of`）とアクセシビリティに伝えるフォーカスの位置（`view/element.rs`）も、描くキャレットの段に置く。選択の端も同じ印で段を決める（#575 の残りを直した）: 選択が印のあるキャレットから始まると、アンカーも印（`Cursor::anchor_free_col`）を持ち（サクラの `BeginSelectArea` が `m_sSelectBgn` へキャレットの点を写すのと同じ）、クリックから始まるドラッグ・Shift+クリックも引き継ぐ。ステータスバーの行数（`selection_message`、`PrintSelectionInfoMsg`）は、両端の立つ段（`hit::caret_row_pos`・`anchor_row_pos`）から数え、通常選択の「終わりが段の先頭なら 1 行引く」は終わりの立つ段の先頭かどうかで見る。矩形の角も、折り返し行の端の隙間に立つ（サクラの角は `CLayoutPoint` で、`CViewSelect` が `BeginSelectArea` でキャレットの点を写す）。角の位置は次の段の先頭、印はキャレットの `Cursor::free_col` とアンカーの `Cursor::anchor_free_col`（`Cursor::move_box_to_gap`）で、読むときに `wrapped_gap_before` で確かめる（`context/boxsel.rs` の `Corner`、`box_corners`）。矩形では、自由カーソルのモードが切れていても、隙間は行の幅（`max_line_x`）まで桁を取る（`Command_RIGHT` の `x_max`）。矩形を折り返し行の端から始める（End のあと `F_BEGIN_BOX`、Shift+Alt+矢印）、矢印で角をそこへ動かす（← は隙間を 1 桁ずつ戻って最後の文字へ、→ は最後の文字から隙間へ入って幅まで、↑↓ は目標の桁が行の端より右なら隙間）、Alt+クリック・Alt+ドラッグを折り返し行の右に置く（`mouse_begin_box_at`。離して空の矩形が終わっても、キャレットは隙間に残る）。コピー・切り取り・削除・矩形の貼り付け・入力・インデント・変換・ソートは、角の段（`Corner::row_pos`）で行を数える。サクラ 2.4.2 で 40 桁折り返しの 100 桁の行に End、Shift+F6、↓、←、コピーが 39 桁目の「9」（MATSU 0.27.1 は 38 桁目の「8」）、↓だけと↓→は幅 0（空の行 2 つ）で、今は同じ（`view/mouse_tests.rs` の `a_box_from_the_end_of_a_wrapped_row_copies_what_sakura_copies`）。選択の塗りは文字の範囲で決まるので、印があっても変わらない（矩形の塗りは、角の段と桁で決まる: `hit::BoxArea`） | B | S | GPUI |
| 背景画像の残り: 大きな画像の読み込みは同期（サクラも同期）、8192 ピクセルを超える画像は読まない（型 B。サクラは `sakura_core/doc/CEditDoc.cpp` の `SetBackgroundImage`（309〜365 行）で大きさを制限しない。GPU の 1 枚のテクスチャに入る大きさまで） | `CTypeSupport`、`CBackground` | 背景画像（パス、位置、繰り返し、スクロール、透明度）は入った（#346）。ミニマップにも描く（サクラのミニマップは `CEditView` で、同じ `OnPaint`・`DrawBackImage` で描く。拡大・縮小はせず、ミニマップの文字領域の左端（4）と、ミニマップの先頭行 × 行の高さのスクロールで置く）。残りは左の 2 つ | C | S | なし |
| メニューの項目名の入れ替え（`F_SHOWMINIMAP_ON` / `_OFF` など）、ショートカットの桁そろえ、アクセスキーの下線（GPUI のメニューは印を描けない。名前の後ろの `(F)` で見せる）、「アクセスキーを括弧付きで表示」を切った名前のアクセスキー（名前にキーの印が残らず、読み取れない） | `CMainMenu`、`CEditWnd`、`CMenuDrawer::OnMenuChar` | Windows と Linux のメニューバーは、Alt+文字（キー割り当てが先）、開いたメニューの文字（同じ文字は次へ。サブメニューの中も同じで、ウィンドウ一覧の数字もこれで動く）、Alt の単独押しと矢印を持つ（`menu_bar/keys.rs`、`menu_bar/access.rs`）。右クリックのメニュー、カスタムメニュー、タブ、ステータスバーと開くボタンのドロップダウン、アウトラインのペイン、マクロの `CreateMenu` のポップアップは、開いたメニューの文字で項目が動く（同じ文字は順に選ぶ。サブメニューの中も同じ。`menu_bar/popup_keys.rs`）。macOS でも GPUI のポップアップはこれで動く。メニューは開いても最初の項目は選ばない。固定の名前。メニュー項目の選択中のキーをステータスバーに出す機能は、gpui-component が通知を出さないので不可（#168） | B | S | gpui-component |
| アウトラインのフローティング | `CDlgFuncList.cpp` | 別の窓にせず本文の上に重ねて出す（型 B）。位置と大きさの記憶とタイトルのドラッグでのドッキングは入った（#351） | B | － | なし |
| 比例フォント（送り幅が桁と合わないフォント）の残り: 印刷（ADR-0034 の決めたこと 2「画面が済んでから決める」。画面は #469〜#488 で済んだ）、ミニマップの文字の位置はレイアウトの X と細かくは合わない（自分のフォントで文字列をまとめて描くため）、合字と kerning は効かない（サクラも同じ。型 B） | `CNativeW::GetHabaOfChar`、`CTextMetrics::GenerateDxArray`（ADR-0034 に根拠） | 画面は済んだ: 文字ごとの幅の表でレイアウトして、1 文字ずつ置く（`matsu-layout` の `WidthTable`、`view/font_widths.rs`。#482・#488）。書記素の境界は `unicode-segmentation`（UAX 29。#361。`view/element.rs`）。前の行にあった「依存の判断が要る」は、すでに入っているので当たらない。#354 の「まれな文字の並べ方」（アラビア文字・インド系の連なり、右から左と左から右の混在など）の細部は、そのあとの変更で確かめていない（未確認） | 印刷は ADR か ユーザーの判断（ADR-0034 の段階 6）、ほかは C | M | 印刷は ADR-0011 と ADR-0034 |
| Mac で既定のフォント（BIZ UDGothic）が入っていないと、本文を Menlo とヒラギノ角ゴシックで描く（半角が 0.6em、全角が 1em なので、全角の間が少し空く） | － | 原因は、macOS に BIZ UDGothic が最初から入っておらず、フォントブックでダウンロードしないと無いこと（診断 #358 で確認）。GPUI は無い家族を Helvetica に替えるので、macOS だけ、頼んだフォント、BIZ UDGothic、Menlo とヒラギノ角ゴシックの順に選ぶ（`view/text_font.rs`）。タブバー、設定ダイアログのフォント名の見本、印刷とプレビューは変えていない | B | S | OS |

### 印刷

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| プリンターを名前で指定すること（macOS・Linux） | `CPrint.cpp`、`CPrintPreview.cpp` の `DispatchEvent_PPB` | Windows は済み: 設定が覚えたプリンター（`szDriver` `szDevice` `szOutput`）と、設定の用紙の大きさ・向きで印刷ダイアログを開き（`print/windows.rs` の `print`。`DEVMODE` の `dmPaperSize` と `dmOrientation` に設定の値を入れる。`CPrint::PrintDlg`）、選んだプリンターを設定に覚える。プレビューのバーの「プリンター(&R)」（`IDC_BUTTON_PRINTERSELECT`。印刷ページ設定のダイアログのボタンではない）は、`PD_PRINTSETUP | PD_NONETWORKBUTTON` のダイアログを開き、OK で、プリンターと用紙の大きさと向きを設定へ書いて（`PS[nn]` の `szDriver` `szDevice` `szOutput` `nInts`）、開いている全ウィンドウのプレビューに割り付けを直させる（`select_printer`、`PrinterChoice`）。ドライバーが値を入れなかった項目と、サクラの用紙の一覧にない用紙は、設定の今の値のままにする（サクラは `dmFields` を見ずに写す。型 B）。残りは、macOS・Linux でプリンターを名前で指定すること（印刷パネルとポータルが最後に使ったものを覚えているので、設定には覚えない。バーに「プリンター」のボタンも置かない: 開くものがない。型 B） | B | S | なし |
| 大きな文書で割り付けが UI スレッドを止める（残り: macOS） | `CPrintPreview::OnChangePrintSetting` | 測った（release、1 行 99 文字・100 万行・100 MB の文書）: 割り付けは ASCII で約 0.6〜0.7 秒、日本語で約 1.1〜1.4 秒（`matsu-print` の `timing_of_the_layout_of_a_100_mb_text`、`cargo test --release -p matsu-print timing -- --ignored --nocapture`。2026-11-19 に WSL の ARM64 で測り直して 0.71 秒と 1.37 秒）。止まるので、プレビューは 2 万行より大きい文書を別のスレッドで割り付ける（進み具合を帯に出し、設定を変えると中止する。`print/preview.rs` の `LayoutJob`）。印刷は Windows と Linux では別のスレッドで割り付ける。macOS の印刷パネルは、用紙に合わせた割り付けを、AppKit の `knowsPageRange:`（UI スレッド）の中でしていた。今は、`matsu-platform` の `print/papers.rs`（`PaperLayouts`）が、用紙ごとの割り付けを覚えて、同じ用紙を何度聞かれても割り付け直さず、プレビューがすでに作った割り付け（設定の用紙の分。`PrintPreview::layout_snapshot`）をパネルが開く用紙と同じなら使い、聞かれた用紙を縦横に回した用紙を、大きい文書（2 万行より大きい）なら別のスレッドで先に割り付ける。残りは、パネルで、先に割り付けていない用紙（A4 から B5 に替えるなど）を選ぶときの 1 回で、`knowsPageRange:` はその場でページ数を返す決まりなので、大きい文書では上の数字の分だけ待つ（AppKit に非同期で答える道がない。型 B）。パネルが開く用紙は、アプリが最後に印刷した用紙で、設定の用紙とは限らない（同じでないと、開くときに 1 回割り付ける）。実機で確かめる（Sandbox の手順は PR の本文） | B | S | なし |

### OS との結び付き

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| タスクトレイ（常駐、左クリックのメニュー、`-NOWIN`）、トレイのショートカットキー | `CControlTray.cpp` | 設定は灰色。`-NOWIN` は読むだけで何もしない | B | L | GPUI にトレイの API があるか（未確認） |
| 配布: 署名、公証、`.dmg`、Wayland のアイコン（`.desktop`）、Finder の「このアプリで開く」の確認 | － | #198、#215 は途中 | ユーザーの判断が要る: リリース | M | アカウント |
| Linux の HTML クリップボード | `CClipboard.cpp` | テキストとして HTML のソースが載る（#170）。GPUI 0.3.8 の Linux の書き込みが文字しか載せないため。X11 は `write_to_clipboard` が `item.text()` だけを `UTF8_STRING` で書き（`linux/x11/client.rs`。`text/html` の atom は `x11/clipboard.rs` でコメントアウトのまま）、Wayland は `TEXT_MIME_TYPES`（`text/plain` の 3 種）だけを `offer` する（`wayland/client.rs`）。`ClipboardEntry` にも HTML の種類がない。MATSU が自分で書くと、GPUI の所有者と取り合い、依存（x11rb・wayland-client）も要る。報告の下書きは `.claude/handoff/drafts/`（ユーザーが再現してから投稿） | B | S | GPUI |
| macOS のネイティブメニューのアイコン、メニューを開く直前のフック | － | 入れていない（#136、#124）。gpui-pre の限界 | B | S | GPUI |
| ファイルのドロップ: 右ボタンのメニュー、編集ウィンドウの上限 | `CEditView::OnMyDropFiles` | #123 | B | S | GPUI |
| ホイール 1 ノッチの行数のシステム設定 | － | Windows だけ反映（#49） | B | S | GPUI |
| ウィンドウの配置・常に手前（Linux） | `CEditWnd` | OS が許さない（#88） | B | － | OS |

### アクセシビリティ

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| エディタの本文の読み上げ（可視行の TextRun、キャレット、選択） | － | 入れた（`view/a11y_text.rs`）: 画面に出ている行を TextRun に、キャレットと選択を TextSelection にして、Div の `a11y_synthetic_children` で出す。文字の位置と幅、単語の始まりつき。Windows（UI Automation の TextPattern）と Mac（AX）での確認は調整役。画面の外の行（余白）とキャレットが画面外のときの選択は出さない。残り: キーボードのフォーカスが本文の要素に移らない（UI Automation の `HasKeyboardFocus` が偽。ウィンドウを切り替えて戻ると正しくなる。GPUI の側で、#290 で調べた。報告の下書きは `.claude/handoff/drafts/`。ユーザーが再現してから投稿） | C | S | GPUI |
| ポップアップメニューの入れ物と区切り線に名前がない、項目のチェック・灰色・位置 | － | gpui-component の限界（#177、#204）。報告の下書きは `.claude/handoff/drafts/`。ユーザーが再現してから投稿 | ユーザーの判断が要る: ほかの OSS への投稿 | － | gpui-component |
| 無効なボタン・入力欄の disabled | － | 表（一覧）の名前は入れた: 設定・履歴・タグジャンプ・ウィンドウ一覧などは前から、検索・Grep・DIFF・ファイル内容比較・外部コマンド・ワンライナーの履歴の一覧とダイアログに、役割と名前を付けた（`a11y.rs` の `list_box`・`dialog`）。灰色のチェックボックス・ラジオボタンと自前のボタンは `aria_disabled`（#528）で出す。ラジオボタンのグループ（`radio_group.rs`）の使えないボタンにも、グループの側で付ける（2026-11-25 に Sandbox で、DIFF の「他の編集中のファイル」が灰色なのに `IsEnabled` が真だったのを直した）。残り: gpui-component の `Button` と `Input` の `disabled` は見た目だけで、AccessKit の `disabled` を書かない（GPUI にも書く所がない）。MATSU の側からは付けられない。ドロップダウン（`drop_down.rs`、#586）の閉じた箱は、コンボボックスとして名前・値・開閉を出すが、箱の中のボタンも別に出る（GPUI にボタンの役割を隠す口が見つからない。2026-11-25 に Sandbox で、コンボボックス「色選択」の値と開閉が出ることを確かめた）。報告の下書きは `.claude/handoff/drafts/`。ユーザーが再現してから投稿 | B | S | gpui-component |

### そのほか

| 項目 | サクラの場所 | MATSU の今 | 判断 | 大きさ | 依存 |
|---|---|---|---|---|---|
| コマンドラインの `-GROUP=`（タブのグループ指定）、`-NOWIN`、`-DEBUGMODE`、`-PROFMGR` | `CCommandLine.cpp`、`CNormalProcess.cpp`、`CProcessFactory.cpp` | 読んで無視する（`command_line.rs`）。`-DEBUGMODE` はタイプだけ変える。理由: `-GROUP=` は、サクラが自分で起動する子プロセス（`CControlTray.cpp` 1113 行あたり、`CPropComPlugin.cpp` 575 行）にグループ番号（`GetFreeGroupId`）を渡す内部の取り決めで、利用者には見えない。MATSU のグループはウィンドウ（ADR-0015）で、番号がなく、1 つのプロセスが作るので、渡す相手がいない。`-NOWIN` は、トレイだけのコントロールプロセスを起こす（`IsStartingControlProcess`）。トレイがない MATSU で窓なしで起動すると、見えず終えられないプロセスになる。`-DEBUGMODE` は `OutputDebugString` を見せる窓（Windows のデバッグ出力）。`-PROFMGR` は、プロファイルマネージャー（下の「プロファイル」の行、ユーザーの判断が要る）に頼る | B | S | 同上 |
| コマンドラインの細部の残り: 長すぎるパスの箱（`STR_ERR_FILEPATH_TOO_LONG`）、起動を実行中の MATSU Editor に渡したときのプログラム名の数字 | `CCommandLine.cpp` | 空白入りで引用符のない名前の探索（作業フォルダーで、つなげて存在するもの）、使えない文字（`* ? " < > |`）の確認の箱（Windows のみ。ほかでは使える文字なので型 B）、プログラム名の数字による文字コード（`-CODE=` が勝つ）、`-TYPE` の既定の文字コードを読み込みに使うことは入った。長すぎるパスの箱は、MATSU のパスの長さに上限がない（`wcsncpy_s` の `_TRUNCATE` に当たる所がない）ので出す場面がなく、型 B。渡す側のプログラム名の数字は `Request`（`matsu-platform/src/instance/request.rs`）に載らない。載せるには、プロセス間の取り決めの新しい版が要る。ユーザーが、取り決めを変えず、効かせずに閉じると決めた（2026-11-19。`-CODE=` で同じことができる） | B | S | なし |
| ダイアログの表示言語（`szLanguageDll`）。一覧にフォーカスを移すアクセスキーは #547 で入った | `CPropComKeybind.cpp`、`CSelectLang.cpp` | 画面の言語は日本語だけ（共通設定「ウィンドウ」の言語の欄は灰色） | 言語: ユーザーの判断が要る（ADR） | M | なし |

## 調整役の宿題と今のラウンド

- 第 1 波〜第 13 波は終わった。次は、上の「次のラウンドに勧める順番」の表の上から。
- 本文へのキーボードのフォーカス（上の「アクセシビリティ」）は、GPUI への報告の下書きがある。ユーザーが再現してから投稿する。
- 巨大ファイルの「折り返す設定の表示」の不具合は、直した PR が見つからなかった。Sandbox で確かめる。

## サクラとの違いのうち、わざと合わせないもの（`known_differences.txt`）

直す対象ではないが、行が残る限り比較のテストが許している。許可リストは 10 件（2026-11-22。`crates/matsu/tests/sakura_oracle/known_differences.txt`）。表の上の 4 行（5 件）は「わざと合わせない」もの、下の 2 行（5 件）は、まだ入っていないために許している。

| 件 | 理由 |
|---|---|
| `caret-move-cursor-past-line-end` | サクラの不具合（型 A）。わざと合わせない |
| `savecode-sjis-crlf-to-cp1200` | サクラが UTF-16 の指定を無視する（型 A）。わざと合わせない |
| `regex-jp-nonascii-class`、`convert-code-jis-to-sjis` | サクラの不具合と見て合わせない（2026-10-21 のユーザーの判断）。原因は調べていない |
| `indent-tab-empty-lines` | インデント形式の自動判定（`DetectIndentationStyle`）は、サクラ v2.4.3 で既定になり、oracle の v2.4.2 にはない。v2.4.3 に合わせる（2026-11-02 のユーザーの判断） |
| `savecode-sjis-crlf-to-cp51932`、`savezen-utf8-to-cp51932`、`savezen-utf8-to-jisx0208euc` | MS-CP51932 / 20932 がない（「止まっているもの」）。合わせないのではなく、まだ入っていない |
| `saveloss-jp-to-cp1251`、`saveloss-jp-to-latin1` | ベストフィット表がない（「止まっているもの」）。同上 |

## 次の PR・ADR との関係

ユーザーが 2026-10-24 に頼んだ 5 つは、[ロードマップ](roadmap.md)の Phase 7 に並べた。2026-10-25 に、続きを Phase 8 に並べた。この台帳は、そのうちの「サクラの残りの機能」にあたる。

- 未保存の文書の自動保存: ADR-0022（決定。書く側と戻す側は入った。終了のときの動きが残る）
- データベース操作（SQL*Plus を含む）: ADR-0023（決定。実行、エラー行への移動、色分け、サンプルは入った）
- エディタ以外の機能をタブで: ADR-0024（決定。タブの抽象化、メニューの切り替え、プレビューのタブは入った。次の種類は使ってから決める）
- 言語のスクリプト: ADR-0025（決定。TypeScript、PowerShell、Perl、Emacs Lisp は入った。AppleScript が残る）
- 常駐するプラグイン: ADR-0026（2026-10-25 に決定。上の「プラグイン」の表を、ADR の PR の順に埋める）
