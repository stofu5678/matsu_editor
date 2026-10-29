# JSON-RPC の試作

[ADR-0007](../adr/0007-macros-and-plugins.md) の JSON-RPC の形を決めるための調査と、通信の仕様の案。2026-10-11 に、ユーザーが形を承認した。この形で試作（PoC）を作り（[poc/gpui-textview](../../poc/gpui-textview/README.md) の「マクロ（JSON-RPC）」）、結果を「試作の結果」に書いた（2026-10-14）。

## 目的

- ADR-0007 の「別プロセスと JSON-RPC で通信する」の具体的な形を決める
- その形で、サクラのマクロと同じことが書けるかを、PoC で確かめる

## 方式

### ユーザーの案

- エディタのマクロで、`require` のようなライブラリの呼び出しを書かせるのはおかしい
- スクリプトの言語では、次の 2 つを、利用者のスクリプトの先頭に差し込む
  - (a) JSON-RPC を呼んだり呼ばれたりするブート
  - (b) 基本 API
- プロセス ID などは標準入力から受け取る
- コンパイルする言語は、(a)(b) をライブラリとして自分で呼ぶ。標準入力の仕様は共有する
- OS に入っている WSH・AppleScript・PowerShell でも使えると、採用しやすい

### 結論

- `require` を書かせない方針に賛成（Codex も賛成）
- ただし、文字列として先頭に差し込むのではなく、言語ごとの起動の仕組みで先に読み込む。こうすると、エラーの行番号がずれず、利用者のファイルも書き換えない
- 形は次の 5 点（2026-10-11 にユーザーが承認）
  - 標準入出力を使う
  - `initialize` で情報を渡す
  - ASCII で 1 行 1 メッセージにする（改行は LF だけ）
  - 基本 API を前もって作る
  - マクロとプラグインを分ける

## 通信の仕様（案）

案 v0。試作（`poc/gpui-textview`）は、この仕様で作った。

### 1. 起動のしかた

利用者のスクリプトに `require` や `import` を書かせない。言語ごとの起動の仕組みで、ブート（JSON-RPC の読み書き）と基本 API を先に読み込む。利用者のファイルは書き換えない（エラーの行番号がずれないように）。

| 言語 | 起動 | 備考 |
|---|---|---|
| JavaScript（Node.js。`.mjs`・`.cjs`） | `node --require <boot.cjs> <user.mjs>` | `Editor` をグローバルに置く。`.js` は同梱の QuickJS（「同梱のスクリプトホスト」） |
| Python | `python <boot.py> <user.py>` | boot.py が `runpy.run_path(user, init_globals={"Editor": ...}, run_name="__main__")` で実行する。行番号はずれない |
| Emacs Lisp（第 2 弾） | `emacs -Q --batch -l boot.el --eval '(ume-run-user-file "<user.el>")'` | ブートが利用者のファイルを UTF-8 で 1 式ずつ読んで評価し、エラーの行番号を出す |
| Lua（第 2 弾） | `lua <boot.lua>`。利用者のスクリプトは、名前と中身を `initialize` の `params.script` で渡す | Windows の lua.exe はファイルを ANSI（日本語の Windows では CP932）の API で開くので、パスを引数で渡すと、CP932 にない文字のパスを開けない。ブートが `load(中身, "@" .. 名前)` で読むと、どのパスでも動き、行番号もずれない |
| WSH（第 2 弾、VBScript・JScript） | .wsf に `<script src>` を並べる | スクリプトのファイルは UTF-16LE（BOM 付き）でないと日本語を読めない |
| PowerShell 5.1（第 2 弾） | ブートで関数を定義し、利用者のスクリプトをフルパスでドットソースする | `-ExecutionPolicy Bypass` はグループポリシーを越えられない |

- 処理系の場所は設定で変えられる（試作では環境変数）
- コンパイルする言語は、ブートと基本 API をライブラリとして自分で呼ぶ。通信の仕様は同じ

### 2. 1 行の形

- 1 行に 1 メッセージ。行の終わりは LF（0x0A）だけ
  - エディタは CR を送らない。Linux の Emacs は CR でも行を切るので、CRLF だと次の読み取りが空になって失敗した
  - 受け取る側は、行末の CR を無視する
- 送る JSON は ASCII だけにする
  - ASCII 以外の文字は `\uXXXX` で書き、U+10000 以上はサロゲートペア（`\ud83d\ude00` など）にする
  - WSH の標準入出力は CP932、PowerShell 5.1 の標準入力は shift_jis なので、生の UTF-8 は化ける。`\uXXXX` なら正しく往復した
  - Python の `json.dumps` は既定で ASCII にする。Node.js の `JSON.stringify` はしないので、ブートで変換する（JS の文字列は UTF-16 なので、0x7F より大きいコード単位を 1 つずつ `\uXXXX` にすれば、サロゲートペアも正しくなる）
- マクロからエディタへの行は、先頭に RS（0x1E）を付ける
  - エディタは、行の中の最初の RS より前を利用者の出力、RS より後ろをメッセージとして扱う
  - 利用者が改行なしで出力した直後に要求が来ても崩れない
- RS のない行と、標準エラーの出力は、利用者の出力として出力ウィンドウに出す
- エディタからマクロへの行（標準入力）には RS を付けない。標準入力に書くのはエディタだけ
- 1 行の長さの上限は、試作では設けなかった。1 MB を超える行（片道 2 MB を超える 1 行）が、Node.js と Python で往復できた（「試作の結果」）。上限を設けるかは決めていない

### 3. メッセージ（JSON-RPC 2.0）

- 最初に、エディタが `initialize` を通知で送る（応答は要らない）

  ```json
  {"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":1,"processId":1234,"editor":{"name":"UME Editor","version":"0.0.0"},"apiVersion":1,"document":{"path":"C:\\work\\a.txt"}}}
  ```

  - `processId` はエディタのプロセス ID。`document.path` は、ファイルでなければ null
  - Lua では、`params.script` に利用者のスクリプトの `name`（エラーに出す名前）と `source`（中身）を入れる（1. を参照）
- マクロの要求: `{"jsonrpc":"2.0","id":1,"method":"InsText","params":["\u3042"]}`
  - 引数は、サクラのマクロ API と同じ順の配列
  - id は 1 から増やす整数
- エディタの応答: `{"jsonrpc":"2.0","id":1,"result":null}`。失敗なら `{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"..."}}`
- エラーのコードは JSON-RPC の標準を使う
  - -32700: 解析できない
  - -32600: 要求の形が違う
  - -32601: そのメソッドはない
  - -32602: 引数が違う
  - -32603: エディタの中のエラー
  - エディタ独自のコード（-32000 から -32099）は後で決める
- 応答が error のとき、ブートは例外を投げる（JavaScript は Error、Python は例外）。利用者は try で受けられる。受けなければ、マクロはエラーで終わり、言語のエラー表示（行番号つき）が標準エラーに出る
- マクロの終わりは、プロセスの終了で知る。エディタは標準入力を閉じる。終了コードが 0 でなければ、標準エラーと合わせて失敗を出す
- エディタは、いつでもマクロを止められる（プロセスを終わらせる）。試作では、タイムアウト（既定 30 秒、環境変数で変える）と「マクロの中断」のコマンドを用意する

### 4. マクロとプラグインを分ける

- この仕様はマクロ（1 回走って終わるもの）だけを扱う
- マクロからエディタへの同期の要求だけを使い、エディタからマクロへの要求はしない
- 理由: 応答を待っている間にエディタがイベントを処理すると、編集の途中に割り込んでしまう（Codex の指摘）
- 常駐してイベントを受けるプラグインは、別の形で決めた（ADR-0026。下の「プラグインの形」）

#### プラグインの形（ADR-0026、2026-10-26 に同梱の QuickJS で実装）

マクロの形に、エディタからプラグインへの呼び出し `plugin/invoke` を 1 種類だけ足す。「応答を待っている間にエディタがイベントを処理しない」という上の決めは変わらない: プラグインが走っている間、エディタはその呼び出しの要求にだけ答え、同じプラグインに次の呼び出しはしない（1 つのホストは 1 度に 1 つの呼び出し）。

1. 起動: `matsu-script --engine quickjs`。`initialize` の `params.kind` が `"plugin"`、`params.plugin` が `{"id": プラグインの ID, "dir": フォルダー}`。`params.script` はない
2. 呼び出し（エディタ → プラグイン）: `{"jsonrpc":"2.0","id":N,"method":"plugin/invoke","params":{"jack":"Command","name":ハンドラのパス,"source":ハンドラの中身,"args":ジャックの引数}}`。ファイルはエディタが読んで中身を送る（マクロの `params.script` と同じ理由）
3. プラグインはハンドラを走らせる。その間の `Editor.*` と `Plugin.*` は、マクロと同じ形の要求としてエディタに送る（`Plugin.*` は `GetPluginDir` `GetDef` `GetOption` `SetOption` `GetString` `GetCommandNo`。エディタが定義ファイルとオプションのファイルから答える）。ハンドラの中では `Plugin.jack` と `Plugin.args` が見える
4. 応答（プラグイン → エディタ）: `{"jsonrpc":"2.0","id":N,"result":{"ok":true}}`、ハンドラが例外を投げたときは `"error":{"code":-32000,"message":行番号つきのエラー}`。RS で始める（ほかの出力と区別するため）
5. 終わり: `{"jsonrpc":"2.0","method":"plugin/shutdown"}` か、標準入力を閉じる。常駐しない（`UseCache=0`）プラグインは、1 回の呼び出しごとにプロセスを起動して、この `plugin/shutdown` で終わらせる
6. ハンドラは、プロセスの中の**ハンドラのファイルごとの別のランタイム**で動かす。テキストは関数 `(function () { …テキスト… }).call(globalThis)` の本体として評価する（1 行目はそのまま。エラーの行番号は変わらない）。最上位の `let` `const` `class` `var` と関数はその呼び出しのものなので、同じファイルをもう一度呼べる。次の呼び出しに残したいものは `globalThis` に置く（同じファイルの次の呼び出しにだけ見える。別のファイルからは見えない）。既定は sloppy モード（サクラの JScript のプラグインは宣言していない名前に代入する）。テキストの先頭が `"use strict"` なら、関数本体の指示として効く
7. ハンドラのパスは、プラグインのフォルダーの中のファイルに限る（`..`・絶対パス・外を指すリンクは断る。同意の SHA-256 がフォルダーの中のファイルだけだから）
8. 時間の制限と、失敗の数え方は `matsu_plugin::plugin_host`（`LIMITS` の表と `MAX_FAILURES`）


### 5. エディタの側の作り

- 子プロセスの入出力は、UI とは別のスレッドで行う。UI は止めない
- 要求は UI のスレッドで 1 つずつ実行して、応答を返す（マクロは同期なので、要求は 1 度に 1 つしか来ない）
- Windows では、コンソールの窓が出ないように子プロセスを起動する（`CREATE_NO_WINDOW`）
- 利用者の出力の文字コード
  - Node.js は UTF-8
  - Python は、環境変数 `PYTHONUTF8=1` を付けて起動し、UTF-8 にする（Windows でパイプにつながった Python の標準出力は、既定では CP932 になるため）。Python 3.15 からは、付けなくても UTF-8 になる（「試作の結果」）。3.14 以前のために付ける
  - WSH は CP932（第 2 弾で扱う）

### 6. 基本 API

- 名前と引数は、サクラのマクロ API に合わせる（[ADR-0007](../adr/0007-macros-and-plugins.md)）。出典は「出典」を参照
- API の定義（名前、引数の型、戻り値の型）を 1 か所に書き、そこから言語ごとの関数を前もって作る（生成する）
  - JavaScript の Proxy のような、どんな名前でも通す呼び出しにはしない（エディタの補完と、引数の確かめのため）
- 型の対応（null、false、空の配列）は、定義の型から言語ごとに決める
  - Lisp を試したとき、ライブラリの既定のままだと、JSON の false が Emacs Lisp・Common Lisp・Gauche で「真」と判定された
  - null は Common Lisp・Gauche・Racket で「真」と判定された
- 試作で作る API（サクラのヘルプの意味に合わせる）
  - `GetLineCount`、`GetLineStr`、`InsText`、`AddTail`、`GoFileTop`、`GoFileEnd`、`Up`、`Down`、`SelectAll`、`GetSelectedString`、`IsTextSelected`、`MoveCursor`、`Jump`、`GetFilename`
  - 例: `GetLineStr(0)` はカーソルの行、`GetLineStr(n)` は n 行目で、全行数より大きければ空文字列
  - 例: `GetLineCount(0)` の引数は 0 以外ならエラー
  - 例: `MoveCursor(行, 桁, オプション)` の行と桁は 1 から数え、オプションは 0 が未選択、1 が通常選択、2 が矩形選択
  - 改行を含むか、桁の単位（文字か UTF-16 か）などの細部は、サクラのソース（`sakura_core/macro/CMacro.cpp` など）で確かめた（「試作の結果」の「サクラのソースで分かったこと」）

#### サクラのマクロの全コマンドを呼べる（2026-10-16）

サクラのマクロの表（`sakura_core/macro/CSMacroMgr.cpp` の `m_MacroFuncInfoCommandArr` と `m_MacroFuncInfoArr`。`ume_commands::sakura::macro_functions`）の名前は、すべて `Editor.<名前>(...)` で呼べる。

- 呼べる名前の一覧は 1 か所にある。`ume_plugin::catalog::entries()` で、手書きの `api::FUNCS`（14 個。引数の名前と説明がある）と、表のそのほかの行（引数は `arg1`、`arg2`、…）を合わせる。名前は表の綴りのまま（大文字と小文字を区別する）。コードの生成（`api.gen.cjs`、`api_gen.py`、`api_gen.rb`、`api_gen.php`）と、同梱の処理系（Lua、QuickJS）は、この一覧から作る
- 呼び出しの行き先は `ume_plugin::bridge` が決める。コマンド（結果のない行）は、キーマクロと同じく、関数コードでレジストリのコマンドを実行する（キー・メニュー・キーマクロと同じ動き）。1 回の呼び出しが取り消し（Undo）の 1 単位。コマンドがあれば、`FUNCS` の手書きより優先する（今は `Up`、`Down`、`GoFileTop`、`GoFileEnd`、`Char` などがコマンド。`InsText`、`AddTail`、`MoveCursor`、`Jump` はコマンドができるまで手書きの側）。結果のある関数（`GetLineCount`、`GetLineStr`、…）は `FUNCS` が答える
- 引数は表の型（数か文字列）で確かめる。最後のいくつかは省ける（サクラは省いた引数にヌルを渡す。足りない分をコマンドが許すかはコマンドが決める）。型が違う、多すぎるときは `-32602`
- まだコマンドのない名前（サクラの表にはあるが、このエディタが持っていない）は `-32001`（not implemented）で、関数の名前（`F_FILESAVE` など）が入る。窓やクリップボードがないときに動かせないコマンドは `-32002`。サクラの表にない名前は `-32601`。サクラにはこのような場合はない
- 同じ関数コードを持つ古い綴り（`ViewMode` と `ReadOnly`）は、どちらも呼べる
- `1PageUp` のように数字で始まる名前は、メソッド名にできない言語がある。生成したコードにはメソッドを作らず、名前で呼ぶ道（JavaScript は `Editor['1PageUp']()`、Python は `getattr(Editor, '1PageUp')()`、Ruby は `Editor.send(:'1PageUp')`、PHP は `Editor::__callStatic`、Lua と QuickJS の同梱の処理系は表のキー）を使う
- 定義と生成物がずれると、テスト（`generated_files_are_up_to_date`）が落ちる。作り直しは `UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files`

#### 結果のある関数（2026-10-16）

サクラの `m_MacroFuncInfoArr` の関数のうち、このエディタが答えられるものを実装した。手書きの `api::FUNCS`（テキストとカーソルだけで答えられる 14 個）に加えて、レイアウト・クリップボード・窓の状態が要るものは `ume_plugin::functions` にある（`EditorFunc`。引数の名前、型、説明は `FUNCS` と同じに持つので、生成するコードも同梱の処理系もそのまま拾う）。出典はどれも `sakura_core/macro/CMacro.cpp` の `HandleFunction` で、関数ごとのコメントに書いた。

- 窓から受け取るもの（`ume_plugin::host::HostEnv`）: ファイルの文字コード、表示域の行数・桁数・先頭行、ステータスバーの有無、折り返し桁、キーマクロの記録中、実行中のマクロのパス、文書タイプ。呼び出しの前に窓が作り、窓がない実行（`--run-macro`、テスト）は `HostEnv::detached()` で、窓が要る関数は `-32002` を返す。呼び出しが変えたいもの（タブ幅、折り返し桁）は `requests` に入れて返し、窓が呼び出しの後で適用する
- クッキー（`GetCookie` など）と描画スイッチは `HostState` に入れ、窓が持つ。マクロが終わっても消えない。クッキーの範囲は `"window"` と `"document"`。サクラは両方とも `CEditDoc` に持ち、1 つの窓に文書は 1 つなので、このエディタでも寿命は同じ（別々の入れ物）。名前の検査はサクラのまま（`_` 以上の文字をすべて通す）
- 待つ呼び出し（メッセージボックス、ファイル・フォルダーのダイアログ、`Sleep`）は、その場では答えられない（窓が動き続けないと、ダイアログが出ない）。窓が受け取って、ダイアログが閉じる（時間が来る）のを待ち、そのときに応答を返す。待つ間はマクロのタイムアウトの時計を止める（`Session::hold`、`release`）。ダイアログに出る名前の対応は `functions/window.rs` の先頭に書いた
- 選択範囲の位置（`GetSelectLineFrom` など）はレイアウトの行と桁（1 から数える。折り返した行は別の行、桁は表示の桁）で、選択がなければ 0。桁・インデックスを引数や結果にする関数のうち、論理位置（`LineColumnToIndex` の結果、`LogicToLayoutLineNum` の引数など）は UTF-16 のコード単位（`MoveCursor` と同じ）
- `GetDefaultCharLength` は 1。サクラはレイアウトの単位（フォントの幅 + 字間のピクセル数）で答えるが、このエディタは桁（セル）でレイアウトするので、半角 1 文字が 1
- `ExpandParameter` は、`$$ $A $F $f $g $/ $N $b $e $E $n $C $x $y $p $P $V $S $M $h $<profile>` と、条件 `${R? ... $}`（`R w M U N I`）に対応した。`$d $t $D $T $B $Q $I` は対応しない（サクラが知らない文字を書いたままにするのと同じく、そのまま残る）。`$N` と `$E` は簡易表示がないので、フルパス
- クリップボードは文字列だけ。`SetClipboard` の列選択（ビット 0）と行選択（ビット 1）の印は、両方同時には付けられない（列選択が勝つ）。形式を名前で指定する関数は `CF_TEXT`、`CF_OEMTEXT`、`CF_UNICODETEXT`（または 1、7、13）とモード -2 と 3 だけ。ほかは `-32001`
- `InputBox` と `CreateMenu` も待つ呼び出し。`InputBox` は 1 行入力のダイアログ（`CDlgInput1`。メッセージ、既定の文字列、最大の長さ。0 以下は 260。長さは UTF-16 のコード単位で数える）を窓の中に出し、OK で文字列、キャンセルで空文字列を返す。`CreateMenu` はメニュー文字列（`,` 区切り。先頭の `[CDRSEB]` の旗、`-` は区切り線）を `functions/menu.rs` で木にして、カスタムメニューと同じポップアップに出し、選ばれた項目の番号（区切り線とサブメニューは数えない）を返す。キャンセルは 0
- `ChangeInsertSpace` は `bInsSpace` を読み書きする（窓ごと。引数は `VARIANT::iVal` と同じく下位 16 ビットで見る）。Tab キーはこの旗を見て、次のタブ位置まで空白を入れる
- 実装しなかったもの: `GetLineAttribute` は種類 0（変更行）、1（変更の連番）、2（ブックマーク）だけで、差分（3）とアウトライン（4）は無いので `-32001`
- `SetDrawSwitch` は旗を覚えて、前の値を返すだけ。窓の描画は止めない（マクロは UI スレッドの呼び出しの間に動くので、呼び出しの途中で描画は走らない）

## 言語ごとに試した結果

Claude が 2026-10-11 に試した事実。試した場所は、Windows 11 ARM64 の PC と、その上の WSL2（Ubuntu 24.04、aarch64）。Windows の処理系は、WSL から WSL の相互運用のパイプ越しに起動した。試作のエディタ（Rust）からの起動ではない。

### JavaScript・Python・WSH・PowerShell

| 言語 | 基本 API とブートを用意する方法 | エラーの行番号 | 日本語の受け渡し | 起動の時間 |
|---|---|---|---|---|
| Node.js | `node --require boot.cjs user.js` で `Editor` をグローバルに置く | ずれない | 問題なし | 測っていない |
| Python | ブートから `runpy.run_path` で実行する | ずれない | 問題なし | 測っていない |
| WSH（JScript） | .wsf にブートと利用者のスクリプトを並べる | 番号はずれない（ファイル名は .wsf の名前で出る） | 標準入出力が CP932 で、UTF-8 は化ける。`\uXXXX` なら正しく往復した | 約 40 ms |
| PowerShell 5.1 | ブートで関数を定義し、利用者のスクリプトをドットソースする | ずれない（`user.ps1:5` と出る） | 標準入力が shift_jis。`\uXXXX` なら正しく往復した | 約 150 ms |
| AppleScript | Mac がないので試していない | | | |

- Node.js は Linux と Windows、Python は Linux、WSH と PowerShell は Windows ARM64 で試した
- Node.js の `fs.readSync(0)` で、1.5 秒遅れて届いた応答も正しく待てた（Windows でも。EAGAIN は起きなかった）。WSL 経由のパイプでの結果。試作のエディタ（Rust）から起動しても待てた（「試作の結果」）
- WSH は、スクリプトのファイルが UTF-16LE（BOM 付き）でないと日本語を読めない。UTF-8 は BOM があってもなくてもエラー。エディタが UTF-16 に変えて渡せば済む
- WSH の `WScript.Echo` は、ほかの関数に差し替えられない
- PowerShell 5.1 の `ConvertTo-Json` は、`-Depth`（既定は 2）と `-Compress` が要る。ASCII 以外の文字のエスケープは自分で書く必要がある。ドットソースはフルパスでないと失敗した。この PC に PowerShell 7 は入っていない
- このときの Windows の `python.exe` は、ストアへ案内するだけのもの（WindowsApps）で、Python は入っていなかった。2026-10-14 に Python を入れて、試作で確かめた（「試作の結果」）

### Lisp

Emacs の利用者を考えて調べた。6 つとも、次がすべて通った。

- initialize で送った「あいう😀」（`\uXXXX`、サロゲートペアを含む）をそのまま送り返せる
- 1.5 秒遅れた応答を待てる
- 受け取った「日本語の行😀」に「!」を足して返せる
- スクリプトに直接書いた「漢字かな😀」（UTF-8 のファイル）を正しく送れる
- 通信の行が ASCII だけ

| 処理系 | 言語 | 最新版 | Windows ARM64 | 起動＋1 往復（Linux / Windows） | エラーの行番号 | ライセンス |
|---|---|---|---|---|---|---|
| Emacs（`emacs -Q --batch`） | Emacs Lisp | 31.1（2026-08） | 公式は x64 だけ（エミュレーションで動いた）。MSYS2 に ARM64 版 | 28 ms / 127 ms | `-l` で読むと出ない。ブートで 1 式ずつ読めば出る | GPL-3.0 |
| SBCL | Common Lisp | 2.6.9（2026-09） | ARM64 版あり（2.6.2 から） | 18 ms / 78 ms | 出る | パブリックドメイン（一部 BSD 系） |
| Racket | Scheme 系 | 9.3（2026-08） | ARM64 版あり | 約 130 ms / 220 ms | `-f` で読むと出ない | MIT / Apache-2.0 |
| Babashka | Clojure | 1.13.225（2026-09） | x64 だけ（エミュレーションで動いた） | 14 ms / 52 ms | 出る | EPL-1.0 |
| Gauche | Scheme | 0.9.15（2024-04） | x64 だけ（エミュレーションで動いた） | 70 ms / 199 ms | 出る | BSD |
| Hy | Python 上の Lisp | 1.3.1（2026-07） | Python 次第 | 43 ms / 試していない | 出る | MIT |

- Linux の Emacs は 29.3、SBCL は 2.2.9（Ubuntu のパッケージ）で試した
- 時間は 7 回の中央値で、この PC での目安。Windows の値は、WSL から起動した分を含む（cmd.exe を起動して終えるだけで 26 ms）
- 起動のしかた（試した形。上の仕様案の起動とは別）
  - Emacs: `emacs -Q --batch -l boot.el -l user.el`
  - SBCL: `sbcl --noinform --non-interactive --no-sysinit --no-userinit --load boot.lisp --load user.lisp`
  - Gauche: `gosh -l boot.scm user.scm`
  - Babashka: `bb --init boot.clj user.clj`
  - Racket: `racket -l racket/base -t boot.rkt -f user.rkt`
  - Hy: Python のブートから `import hy` して `runpy.run_path`
- Emacs
  - batch モードの `read-from-minibuffer` は、読む前に標準出力を fflush するので、要求を書いて応答を待つ形で詰まらない（Emacs 31 の `src/minibuf.c` の `read_minibuf_noninteractive`）。`\n` でも `\r` でも行を終える
  - JSON の関数は Emacs 30 から本体に入っている（29 以前は JSON を使えないビルドがある）。ASCII 以外をそのまま出すので、`\uXXXX` への変換をブートに書いた
  - `-l user.el` だとエラーに行番号が出ず、バックトレースは標準出力、最後の 1 行だけ標準エラーに出た（Linux の 29.3、Windows の 31.1 とも）。Codex は「マニュアルでは標準エラー」と言ったが、分けて取り直しても標準出力だった。Emacs 31 は `lexical-binding` の指定がないと警告を出す
  - ブートで利用者のファイルを UTF-8 で 1 式ずつ読んで評価すると、`user_err.el:3: Symbol's function definition is void: ...` と出て、警告も消えた。行番号は、エラーが起きた最上位の式の開始行
  - 日本語の Windows では、Emacs の既定の文字コードは CP932。今回は UTF-8 と正しく判定されたが、ブートで UTF-8 に固定する
  - Emacs のバッファーの関数（`insert` など）は UME Editor の文書には効かない。`ume-insert` のような関数を用意する
- 利用者の設定を読まずに起動する: Emacs は `-Q`、Gauche は `-q`。Babashka は、スクリプトの隣かカレントディレクトリにある `bb.edn` を読む。環境変数 `BABASHKA_PRELOADS` にも注意（Codex の指摘）
- 改行: Linux の Emacs に CRLF を送ると、次の読み取りが空になって失敗した（Windows の Emacs と SBCL は大丈夫だった）
- 型: ライブラリの既定のままだと、JSON の false が Emacs Lisp・Common Lisp・Gauche で「真」、null が Common Lisp・Gauche・Racket で「真」と判定された
- 調べた側の勧め: Emacs の利用者には Emacs Lisp（`emacs -Q --batch`）、一般の Lisp には SBCL、3 番手は Racket（すべての対象に公式の版がある）。Babashka・Gauche・Hy は要望が出てから
- 後回しにするもの
  - Steel: Rust の Scheme。Helix のプラグインの案はまだ下書き。1.0 前で、Windows ARM64 版の配布なし
  - nbb: Node の上の Clojure。試していない
  - ECL: 配布はソースだけ
  - Guile: Windows でそのまま動く版がまだない
  - Chez Scheme: Windows は Intel 向けだけ、JSON などの部品なし
  - Clozure CL: 最後の正式版が 2024 年
  - CLISP: 2010 年から正式版なし
- 2026-10-11 にユーザーが決めた: Emacs Lisp を第 2 弾の言語に加える（[ADR-0007](../adr/0007-macros-and-plugins.md)）

### Lua

2026-10-14 に、ユーザーの依頼で、第 2 弾の Lua を Lisp と同じ 5 つの確認で試した。Lua 5.5.1・5.4.9（lua.org のソースから作った）と LuaJIT 2.1（試したのは 2026-10-13 のコミット）の 3 つとも、すべて通った。LuaJIT は Linux だけで試した。ほかに、2.4 MB の 1 行（「あ😀」を 20 万回並べた文字列）の往復と、スクリプトの `print` の日本語も確かめた。

| 処理系 | 最新版 | Windows ARM64 | 起動＋1 往復（Linux / Windows） | エラーの行番号 | ライセンス |
|---|---|---|---|---|---|
| Lua | 5.5.1（2026-07）。5.4 系は 5.4.9（2026-08） | lua.org の配布はソースだけ。Visual Studio の ARM64 用のコンパイラーで作れた。winget の DEVCOM.Lua（5.4.6）は x64 だけで、エミュレーションで動いた | 1.4 ms / 18〜20 ms | 出る | MIT |
| LuaJIT | 2.1（ローリング リリース） | winget の DEVCOM.LuaJIT は x64 だけ。試していない | 1.3 ms / 試していない | 出る | MIT |

- 時間は 7 回の中央値。Windows の値は、PowerShell から .NET の `Process` で起動して、Windows の中で測った。ほとんどがプロセスの起動の分で、スクリプトなしで lua.exe を起動して終えるだけで 21.3 ms だった。上の Lisp の表と同じく WSL から起動すると約 51 ms（cmd.exe を起動して終えるだけで 33 ms）。x64 版の lua.exe をエミュレーションで動かすと 25.5 ms
- 起動のしかた（試した形）: `lua boot.lua user.lua`。または `lua boot.lua` だけにして、`initialize` の `params.script` で中身を渡す
- エラーは `user_err.lua:3: attempt to call a nil value (global 'undefined_function')` と出た（LuaJIT は `attempt to call global 'undefined_function' (a nil value)`）。ブートは `xpcall` と `debug.traceback` で受けて、標準エラーに出す
- ブートは、Lua 5.1（LuaJIT）から 5.5 まで同じものが動くように、ビット演算と utf8 ライブラリを使わずに書いた。標準ライブラリに JSON がないので、ASCII へのエスケープ（サロゲートペアを含む）と読み取りをブートに書いた（全体で約 200 行）
- 型: JSON の false と null は、Lua でもそのまま偽になる（Lisp で起きた問題は起きない）。null は nil になるので、配列の途中の null は穴になる。困る場面が出たら、印の値を用意する
- Windows の lua.exe
  - 標準出力がテキストモードなので、`\n` が CRLF になって届いた。受け取る側が行末の CR を無視すれば済む（2. のとおり）
  - ファイルを ANSI の API（`fopen`）で開くので、利用者のスクリプトのパスを引数で渡すと、CP932 にない文字（絵文字など）を含むパスは `??` になって開けなかった。CP932 で書けるパスは開けたが、エラーに出るファイル名が CP932 になり、UTF-8 として読むと化けた
  - エディタがスクリプトの名前と中身を `initialize` で渡し、ブートが `load(中身, "@" .. 名前)` で読むと、絵文字を含む名前でも動き、エラーも `C:\…\マクロ\絵文字😀.lua:3: attempt to call a nil value ...` と UTF-8 で出た
- 入手のしかた: Ubuntu 26.04 には lua5.5（5.5.0）・lua5.4（5.4.8）・lua5.1・luajit のパッケージがある。Mac は Homebrew（lua 5.5.1、luajit。ARM64 版あり）。Windows は上の表のとおり
- Neovim の `vim.*` のような API は、UME Editor の文書には効かない。基本 API は、ほかの言語と同じく、生成した関数で用意する
- 2026-10-14 にユーザーが決めた: Lua は第 2 弾のままにする。処理系を同梱する案は「決まっていないこと」に残した

### PowerShell（`.ps1`。2026-10-24 に実装。ADR-0025）

- 処理系: `pwsh`（PowerShell 7）を先に探す。Windows で `pwsh` が PATH になければ、OS に入っている Windows PowerShell 5.1（`powershell`）で動かす。別の方に変えるときは `MacroConfig::interpreter` で指す（設定画面の欄は、ほかの言語と同じく後で）
- 起動: `-NoLogo -NoProfile -NonInteractive -Command <短い文>`。短い文が、環境変数 `MATSU_BOOT_DIR` のフォルダーにある `boot.ps1` の中身を読んで、スクリプトブロックにして実行する。ファイルを `-File` で渡さないのは、実行ポリシー（Windows のクライアントの既定は Restricted）がスクリプトの「ファイル」だけを止めるため。文字列から作ったスクリプトブロックは止められない（グループポリシーが `-Command` 自体を止める環境は対象外）
- 利用者のスクリプトは、ファイルのパスではなく `initialize` の `params.script`（`name` と `source`）で渡す（同梱の処理系と同じ）。5.1 は BOM のないファイルを ANSI で読むので、文字コードの問題も避けられる。ブートは `[scriptblock]::Create` で作り、グローバルのスコープで実行する（コンソールと同じく、利用者の変数と関数はグローバルに残る）
- `$Editor` は、API の名前ごとの ScriptMethod を持つオブジェクト（`$Editor.InsText('x')`）。サクラの表のほかの名前や、数字で始まる名前は `$Editor.Call('1PageUp')`。`$Editor.Info` が `initialize` の内容。生成物は `api_gen.ps1`
- スクリプトの場所: 文字列から作るので `$PSScriptRoot` と `$PSCommandPath` は空（設定できない）。代わりに `$MacroPath`（スクリプトのパス）と `$MacroDir`（そのフォルダー）を用意した
- JSON: `ConvertFrom-Json` は、日付に見える文字列を DateTime にする版があり、`ConvertTo-Json` は 5.1 と 7 で違うので、ブートに小さな読み取りと書き出しを書いた。書き出しは ASCII だけ。整数の値の `double` は整数で送る。コマンドレット（`New-Object`、`Join-Path`、`Add-Member`）は、最初の呼び出しにモジュールの読み込みで 100〜300 ms かかるので、起動の経路では使わない
- エラー: エディタの `error` は ErrorRecord（例外の `Data['Code']` に番号）として投げる。ScriptMethod が `ScriptMethodRuntimeException` で包むので、利用者は `Get-EditorErrorCode $_`（番号か `$null`）で調べる。捕まえなかったエラーは `名前:行: メッセージ` で標準エラーに書き、終了コード 1。行は、利用者のスクリプトの先頭を 1 とする（エディタを呼んだ行、構文エラーは構文解析器が示す行）
- 起動の時間: この PC（WSL2 の Linux ARM64、PowerShell 7.6）で、`-Command 1` が約 0.3〜0.7 秒、ブートを含めて 1〜1.5 秒。Node.js は約 0.1 秒。利用者には、マクロを動かしてから最初の動きまで 1 秒ほどかかるように見える。Windows と 5.1 の値は未測定（過去の記録: 5.1 の起動は約 150 ms、Windows ARM64）
- 出力: 標準出力と標準エラーの文字コードは UTF-8（BOM なし）に設定する

### Perl（`.pl`。2026-10-24 に実装。ADR-0025）

- 起動: `perl -`。プログラム（`api_gen.pl` と `boot.pl`）を標準入力に書き、`__END__` で終える。Perl は `__END__` までをプログラムとして読み、残りは `DATA` ハンドルで読める。その残りが `initialize` の行と、呼び出しへの応答。ファイルを一つも作らない。Windows の perl はファイルを ANSI の API で開くので、一時フォルダーのパスに ANSI にない文字（英語版 Windows の日本語・中国語のユーザー名）があると開けないが、これなら起きない。`PERL5OPT` は外す（モジュールやオプションを差し込めるため）
- JSON: `JSON::PP`（Perl 5.14 から標準）。`ascii` で書き出す。5.14 以降が対象（Module::CoreList で、JSON::PP の最初の版は 5.013009、つまり 5.14.0 に入る）。この PC は 5.38
- 利用者のスクリプトは `initialize` の `params.script` で渡し、`eval` で動かす。文字列は文字（`use utf8` と同じ）で、先頭に `#line 1 "名前"` を付けるので、エラーは `名前 line N.` と出る。ブートの `strict` と `warnings` は引き継がない（`perl script.pl` と同じ）
- 出力: `STDOUT` と `STDERR` は `:raw:utf8`（Windows が改行を CR LF にしない、コードページに依らず UTF-8）
- `Editor->InsText('x')` のクラスメソッド。関数の形（`Editor::InsText('x')`）は使えない。サクラの表のほかの名前は `AUTOLOAD`、数字で始まる名前は `Editor->Call('1PageUp')`。`Editor->info` が `initialize` の内容（ハッシュ参照）
- 数と文字列: Perl は区別が曖昧なので、数の値を持つ（印字して文字列の面も持つものを含み、文字列が数に読める）ものを数、そうでなければ文字列として送る。`"$x"` は常に文字列。NaN、無限大、参照は送る前に断る
- エラー: エディタの `error` は `EditorError` オブジェクト（`code`、`message`、`file`、`line`）を `die` で投げる。文字列にすると `EditorError CODE: メッセージ at 名前 line N.`。N はエディタを呼んだ利用者の行
- 起動の時間: `perl` 自体は約 5 ms。プログラムは約 2300 行（生成した API）

### Emacs Lisp（`.el`。2026-10-24 に実装。ADR-0025）

- 起動: `emacs -Q --batch --load boot.el`（`-Q` で利用者の設定を読まない）。ブートが `initialize` の `params.script` を受け取り、`api_gen.el` を読み込んで、スクリプトを実行する。終了はブートが `kill-emacs` で行う
- 対象は Emacs 30 以降（JSON の関数が本体に入っている）。29 以前でも、JSON に対応してビルドされていれば動く（ブートは `json-parse-string` と `json-serialize` の有無を調べ、なければ「Emacs 30 以降が要ります」と出して終わる）。Ubuntu 24.04 の `emacs-nox` は 29.3
- スクリプトは 1 式ずつ読んで評価する（`read` と `eval`）。`--load` で読むと行番号が出ないため。エラーは `名前:行: メッセージ` で、行は失敗した最上位の式が始まった行。構文エラー（括弧の過不足など）は、読めなかった式の始まりの行。レキシカルバインディングで評価する
- 文字コード: `locale-coding-system` などを UTF-8 に固定する（日本語の Windows の既定は CP932）。通信の行は ASCII のみ（`json-serialize` が返す UTF-8 を文字に戻して、ASCII を超える文字を `\uXXXX` に書き換える）。エディタへは LF だけを送る（Linux の Emacs は CR を行の終わりに読む）
- 関数: Sakura の名前をハイフン区切りの小文字にして `editor-` を付ける（`InsText` は `editor-ins-text`、`GetLineStr` は `editor-get-line-str`）。どの名前でも `(editor-call "InsText" "x")`。数字で始まる名前は `editor-call` だけ。引数は文字列・整数・浮動小数点数（整数の値は整数で送る。NaN と無限大は断る）・`nil`（null）・`t`（true）・`:false`。`editor-info` が `initialize` の内容（alist）
- エラー: エディタの `error` は `editor-error` を `signal` する（データは番号とメッセージ）。`(condition-case err (...) (editor-error (cadr err)))` で番号を調べる
- 出力: `princ`・`print` は標準出力、`message` は標準エラー
- 本物の Emacs をローカルで動かせなかったので、動作の確認は CI（Ubuntu 24.04 の Emacs 29.3）による

### AppleScript（`.applescript`。macOS のみ。2026-10-25 に実装。ADR-0025）

- 起動: `osascript <一時フォルダーのプログラム>`。エディタが、前置き（`boot/boot.applescript`）、利用者のスクリプトを入れた `script UserMacro ... end script`、後置き（`on run`）を 1 つのプログラムにして、非公開の一時フォルダーに書く（`applescript.rs`）。利用者のスクリプトは、その script オブジェクトの本体になる（`use` の行は書けない。`on run` を自分で書いてもよい）
- 中継（ADR-0025 の「実装のときに確かめる」の項目）: AppleScript は標準入出力を直接は扱えないので、AppleScriptObjC（`use framework "Foundation"`）で、`NSFileHandle`（標準入力から `availableData` を読んで行にためる。標準出力へはすぐ書く）と `NSJSONSerialization`（JSON の読み書き）を使う。`osascript` 自身が中継になるので、別の中継プログラムは要らない。ほかの言語と同じ JSON-RPC の行（RS で始まる 1 行 1 要求）
- JSON: 書き出しは UTF-8 のまま（`\uXXXX` にしない。macOS の標準出力は UTF-8 で、受け取る側も UTF-8 を読める）。整数の値の実数（`4 / 2`）は整数として送られる（`NSNumber`）
- 関数: 名前ごとのハンドラーは作れない（`copy`・`delete`・`print` などが AppleScript の語のため）。`editorCall("InsText", {"x"})` の形で、サクラの名前と引数の並びで呼ぶ。`editorPrint(テキスト)` で出力パネルへ 1 行書く（`log` は標準エラー）。`editorInfo("editor.name")` が `initialize` の内容（キーパス）。`tell` の中では `my editorCall(...)`
- エラー: エディタの `error` は AppleScript のエラー（番号がエディタの番号、文字は `EditorError 番号: メッセージ`）。`try ... on error errorText number errorNumber` で受ける。捕まえなかったエラーは `osascript` が `ファイル:開始:終了: execution error: ...`（`開始-終了` の形も読む） と出すので、エディタが文字の範囲を `名前:行` に直す（UTF-16 の文字数で数える。前置きの中の範囲は行なしで `名前` だけ）。構文エラーも同じ
- 戻り値: `osascript` は `run` が返した値を標準出力に出すので、後置きは長い印の文字列（`applescript::END_MARK`）を返し、エディタがその行を捨てる
- 文字コード: プログラムのファイルは UTF-8。日本語・絵文字の文字列は往復のテストで確かめる（macOS の CI で実行）
- macOS 以外では、マクロは始まらず「AppleScript のマクロは macOS でだけ動かせます」と出る

### Codex の指摘（1 回目）

JSON-RPC の方式について、Codex が挙げた点。

- 1 回走って終わるマクロと、常駐するプラグインを分ける。イベントの処理中に応答を待つと、編集の状態に割り込む。エディタは UI のスレッドを止めてはいけない
- ASCII へのエスケープは自分で書く（`JSON.stringify`、json2、PowerShell 5.1 はしない）。サロゲートペア、CRLF、分割して届く読み取り、最大の長さを仕様に書く
- RS の目印は、改行のない出力や子プロセスの出力で崩れうる。専用の通信路が一番堅い
- API には、型・戻り値・エラー・版を含む定義と、機能のやり取りが要る。文書の ID、位置の単位、文書の版、まとめての編集、Undo の区切りも要る
- `cscript //U` で、リダイレクトした入出力が UTF-16 になる
- PowerShell は、コンソールのない GUI から起動した場合を試す
- AppleScriptObjC の `availableData` は分割して返るので、ためる必要がある
- Node.js の issue #35997 に、EOF と EAGAIN の問題の報告がある
- VBScript は Windows 11 24H2 で「オプション機能」になり（既定で有効）、2027 年ごろに既定で無効になる予定。削除の時期は未定
- `-ExecutionPolicy Bypass` は、グループポリシーを越えられない

## 同梱のスクリプトホスト（ume-script）

2026-10-16 にユーザーが決めた: Lua・QuickJS（JavaScript）・MicroPython・mruby の処理系を同梱する。ただし本体のプロセスには入れず、`ume` の隣に置く 1 つの別の実行ファイル `ume-script` に入れる。`ume-script` は、外部の処理系と同じ JSON-RPC のマクロの仕様（上の 1.〜3.）を話す。止まったり無限ループしたりしても、巻き込まれるのはそのプロセスだけで、エディタは、ほかのマクロと同じく時間切れで止める。どの言語も同じしくみで動く。

- 実装は `crates/ume-script`。起動のしかたは `ume-script --engine lua`。`--engine` で処理系を選ぶ。今あるのは Lua（5.4。`mlua` に同梱の C のソースをビルドして使う）と QuickJS（`--engine quickjs`。`rquickjs` が持つ QuickJS-NG の C のソースをビルドして使う）で、ほかの処理系は `Engine` トレイトを実装して `engine::by_name` に足す
- 拡張子との対応は、`.lua` が Lua、`.js` が QuickJS（どちらも同梱）。Node.js で動かすのは `.mjs` と `.cjs`。エディタ（`ume-plugin` の `Language::Bundled(Engine::Lua / Engine::QuickJs)`）が、`ume` の実行ファイルの隣の `ume-script` を起動する。`MacroConfig::interpreter` で別の場所を指せる（テストが使う）。隣になければ `PATH` から探す
- 利用者のスクリプトは、ファイルのパスを引数で渡さず、エディタが読んで `initialize` の `params.script`（`name` と `source`）で渡す（「Lua」の節の理由と同じ）。UTF-8 でなければエディタが読み込みの時点で断る。先頭の BOM は取る
- ホストは `initialize` を読み、スクリプトを実行して、`Editor.InsText(...)` などの呼び出しを要求として送り、応答を待つ。やることは `boot.cjs`・`boot.py` と同じ
  - `Editor` と、その関数、`EditorError` はグローバルにある。関数は `ume_plugin::catalog::entries()`（JavaScript・Python・Ruby・PHP の生成元と同じ一覧）から、Rust の関数として束ねる。1 つずつ書いてはいない
  - 応答がエラーなら、`EditorError`（`code`・`message`・呼んだ場所 `where`）を投げる。`pcall` で受けられ、受けなければ `名前:行: EditorError -32602: ...` で終わる
  - 引数の最後の `nil` は省く。整数の値の浮動小数点数（`4 / 2`）は整数として送る。表や関数など JSON にできない値は、その場で `-32602` の `EditorError` にする
  - `print` は Rust で標準出力に書いて、すぐ流す。`io.write` と `io.stderr` は、バッファーを切って（`setvbuf("no")`）、要求との順番を保つ
  - スクリプトのエラーは、行番号つきで標準エラーに書き、終了コード 1 で終わる。最初の行が `#` で始まる（シバン）ときは、行番号を保ったままコメントにする
- QuickJS（`.js`）は、利用者のスクリプトをモジュールではなく普通のスクリプト（classic script）として実行する。`Editor`・`EditorError`（`Error` を継承し、`code` を持つ）・`console` はグローバルにある。`console.log` は標準出力、`console.error` と `console.warn` は標準エラーに、Rust で書いてすぐ流す。`require` と `module` はない
  - 引数の最後の `undefined` は省く。整数の値の浮動小数点数は整数として送る。関数・シンボル・BigInt・孤立したサロゲートを含む文字列は、その場で `-32602` の `EditorError` にする
  - 捕まえなかったエラーは、`スクリプト名:行:桁` つきのスタックを標準エラーに書き、終了コード 1 で終わる。スクリプトのあとに動く Promise の処理は、終わるまで待つ。誰も受けなかった拒否も、同じようにエラーで終わる
  - 再帰の深さは機械のスタックの手前（512 KB）で止まり、`RangeError` になる（Windows のメインスレッドは 1 MB）
- mruby（`.mruby`。4.0。`crates/ume-mruby-sys` が C のソースと生成済みのファイルを抱えてビルドする。ビルドに Ruby は要らない）も同じ作りで、`Editor` の関数は C の関数として名前ごとに束ねる。`EditorError` は `code` と `message` を持ち、呼んだ行は例外の `backtrace` に出る（`where` はない）。エラーは `名前:行: メッセージ (クラス名)` で標準エラーに書く。入っている mrbgem と理由は `crates/ume-mruby-sys/vendor/mruby/README.md` にある。`.rb` は引き続き利用者の CRuby で動く
- MicroPython（`.upy`、`--engine micropython`）: MicroPython 1.29.0（MIT）を、公式の embed ポート（`ports/embed`）で作った C のソースにして `crates/ume-micropython-sys/vendor/micropython/` に置き、`cc` でビルドして `ume-script` に入れる（再生成の手順は、そこの README）。`.mpy` は MicroPython のバイトコードの拡張子なので使わない。`.py` は利用者の CPython のまま
  - `Editor`（`Editor.info` を含む）と `EditorError(Exception)`（`code`・`message`）は、`boot.py` と同じ。C の関数 `_ume_call` を 1 つだけ経由して、`FUNCS` の名前から束ねる。引数の最後の `None` は省く。整数の値の浮動小数点数は整数として送る
  - 例外は、MicroPython の形で標準エラーに出す（`File "名前", line 3, in <module>`）。終了コードは 1
  - 使えるのは組み込みのモジュールだけ（`sys`・`json`・`math`・`struct`・`collections`・`array`・`binascii`・`heapq`・`re`・`io`・`errno`・`gc`）。ファイルは開けない。`re` は MicroPython の小さな `re`（エンジンは re1.5。BSD-3-Clause で、ライセンス文は上流の re1.5 から取って `vendor/micropython/lib/re1.5/LICENSE` に置いた）。CPython の `re` と違い、先読み・後読み・名前付きグループ・フラグ・`{m,n}`・`\b`・`findall` などがなく、`.` は改行にも合い、`.` と繰り返しは 1 文字でなく 1 バイトに働く（日本語の文字の途中で終わる一致は `UnicodeError`。文字そのもの・`|`・範囲は使える）。詳しくは `crates/ume-script/src/micropython.rs` の先頭の説明。ヒープは 64 MiB 固定（OS から確保して使った分だけ触る）で、超えると `MemoryError`
- 配布物には、`ume` の隣に `ume-script` も入れる（リリースのパッケージの作りで扱う）

## 試作の結果

`poc/gpui-textview` で 2026-10-11 に作り、2026-10-14 に Windows の Python でも確かめた。コード、API の表、制約は [poc/gpui-textview](../../poc/gpui-textview/README.md) の「マクロ（JSON-RPC）」にある。

### 作ったもの

- 試作のエディタ（GPUI）の「ツール」メニューに、「名前を指定してマクロ実行...」（サクラのメニューの文言）と「マクロの中断」を足した。マクロの出力、標準エラー、終了の状態は、画面の下の出力パネルに出る
- 通信の仕様（案 v0）は、変えずに使えた。Python は `-u`（出力をためない）を付けて起動した
- 子プロセスの入出力は別のスレッドで行い、要求は UI のスレッドで 1 つずつ実行する。マクロの実行中も UI は止まらない
- 基本 API は、サクラの関数 14 個（6. の一覧）。定義を Rust の 1 か所（名前、引数の型、戻り値の型）に書き、JavaScript と Python の関数を生成してコミットした。定義と生成物がずれると、テストが落ちる
- ウィンドウなしでも走らせられる（`--run-macro <マクロ> <入力> <出力>`）

### 確かめたこと

- 本物の Node.js と Python を走らせる統合テストが、Linux（WSL2 の aarch64。Node.js v25、Python 3.13）と Windows ARM64（Node.js v24、Python 3.15.0）で通った。中身は次のとおり
  - 日本語と絵文字の往復
  - 1.5 秒遅れた応答を待てる
  - 1 MB を超える行（片道 2 MB を超える 1 行）の往復
  - 改行なしで出力した直後の要求（RS で分けられる）
  - 構文エラーと実行時エラーで、利用者のファイルの行番号が出る
  - エラーの応答が例外になる（JavaScript も Python も `EditorError`）
  - 無限ループと、応答が来ない待ちが、タイムアウトで止まる
  - すぐ終わるマクロ、終了コード、標準エラー、中断、処理系がないときの案内
  - 不正な行に -32700・-32600・-32601・-32602 を返す
- 実際のウィンドウ
  - Linux（WSLg）と Windows ARM64 で、自己確認モード（試作が自分で操作して確かめる）を通した。サンプルのマクロ（`sample.js`。Linux では `sample.py` も）、マクロが編集した直後の上下の移動、終わらないマクロの中断を確かめた（2026-10-11、Claude）
  - Windows ARM64 で、ユーザーがメニューから `sample.js` と `sample.py` を実行し、文書と出力パネルが正しいことを確かめた。終わらない Python のマクロを「マクロの中断」で止められ、`python.exe` が残らないこと、コンソールの窓が出ないことも確かめた（2026-10-14）
- macOS では動かしていない

### Windows の Python

2026-10-14 に、python.org が勧める Python Install Manager（26.4。winget の `Python.PythonInstallManager`）を入れ、それで Python 3.15.0 の ARM64 版を入れた。

- `python`・`python3`・`py` は、アプリ実行エイリアス（WindowsApps）で動く。PATH を変えなくても、試作の既定（`python`）で起動できた
- エイリアスの `python` は Install Manager の起動用のプロセスで、それが Python の本体（`%LOCALAPPDATA%\Python\pythoncore-3.15-arm64\python.exe`）を子として起動する。起動用は x64 版で、この PC（ARM64）ではエミュレーションで動く
- 起動用のプロセスを終わらせると、本体も終わった。試作の中断とタイムアウトは、この形でも効く
- ただし、起動のたびに約 100 ms 余計にかかる。PowerShell から Windows の中で測った、9 回の中央値

  | 測ったもの | `python`（エイリアス） | 本体を直接 |
  |---|---|---|
  | `python -c pass` | 117 ms | 19 ms |
  | `--run-macro sample.py` | 174 ms | 72 ms |

  参考: `--run-macro sample.js`（Node.js v24）は 103 ms
- Python 3.15 は、`PYTHONUTF8=1` を付けなくても UTF-8 モードで動いた（PEP 686）。`-X utf8=0` を付けると、パイプにつながった標準出力は CP932 になった

### サクラのソースで分かったこと

API の意味を、サクラのソース（`sakura_core/macro/CMacro.cpp` と、そこから呼ばれるコマンド）とヘルプで確かめた。サクラ本体は動かしていない。

- `GetLineStr` が返す行は、改行を含む。ヘルプには書かれておらず、`CMacro.cpp` の F_GETLINESTR が `GetDocLineStrWithEOL` を呼ぶことで分かった。0 はカーソルの行、全行数より大きい番号は空文字列（ヘルプ）
- 行番号と桁は 1 から数える（ヘルプ）
- 桁の単位は、論理位置の文字数（C++ の `wchar_t`。Windows では UTF-16 のコード単位）。U+10000 以上の文字（絵文字など）は 2 と数える。試作もこの単位にした
- `GetLineCount` は、最後の改行の後ろを行として数えない（`"a\n"` は 1 行）。引数は予約で、0 以外はエラー。`GetSelectedString` の引数も予約で、無視される（ヘルプ）
- `Jump` のフラグを省略すると 1（改行単位）になる（`CMacro.cpp` の F_JUMP）
- `MoveCursor` で行末より大きい桁を指定したとき、行末に止めるのか、フリーカーソルのように保つのかは、ソースだけでは確かめきれなかった（「決まっていないこと」）

### エディタの側の作りで分かったこと

- コマンドに渡すもの（`CommandContext`）を、14 個の関数で要ったものから詰めた。[ADR-0013](../adr/0013-module-structure.md) の「4. `CommandContext` に何を渡すか」に書いた
- マクロは描画の間に何度も呼び出すので、最後に描画したときの割り付けは、マクロ自身の編集の後では古い。試作の上下の移動は、割り付けを使わず、文字単位で動くようにした
- Windows で、ブートを書き出した一時フォルダー（`ume-macro-*`）が残ることがある（2026-10-14 に見つけた）
  - 試作は、マクロの終了を知らせた後で一時フォルダーを消す（消せなくてもエラーにしない）。知らせた直後にプロセスが終わると、消す途中で打ち切られる
  - テストを 1 つずつ別のプロセスで 3 回ずつ回すと、Python を使うテストで合わせて 35 個残った。テストの最後に 0.3 秒待つようにすると 3 個に減り、その 3 個は待ちを通らないテストのものだった
  - ウィンドウでの実行と、ウィンドウなしの実行（28 回）では残らなかった
  - Python を入れた直後の最初のテストの実行では 26 個残った。こちらは、プロセスがほかのテストのために動き続けていたので、この理由では説明できない（原因は分かっていない）

### 確かめていないこと

- Undo のまとまり（試作のエディタに Undo がない）
- プラグインとイベント（エディタからマクロへの要求）
- 子プロセスの出力で、RS の目印が崩れないか
- 範囲の読み書きの速さ
- macOS、WSH、PowerShell、AppleScript
- マクロの実行中の利用者の編集。試作は止めていないので、マクロの呼び出しの間に、キーボードでの編集が割り込むことがある

## 決まっていないこと

- 1 行の長さの上限。試作では、1 MB を超える行（片道 2 MB を超える 1 行）が通った。上限を設けるか、設けるならいくつにするか
- エディタ独自のエラーコード（-32000 から -32099）
- 位置の単位。サクラは UTF-16 のコード単位で、試作もそうした（「サクラのソースで分かったこと」）。本実装でもそうするか
- プラグインの形（イベントを受ける常駐の形。マクロとは分けて決める）
- 型の対応（null、false、空の配列を、言語ごとにどう対応させるか）
- RS の目印の堅さ。改行なしの出力の直後の要求は、試作で崩れなかった。子プロセスの出力は試していない。専用の通信路にするかも含めて
- Undo のまとまり（1 つのマクロを 1 回の Undo にするか）。試作のエディタに Undo がないので、確かめていない
- 機能のやり取りと版（Codex の指摘。`initialize` の `protocolVersion`・`apiVersion` の扱いも含む）
- 組み込み向けの小さな処理系（Lua、mruby、MicroPython、QuickJS など）をエディタに同梱するか。2026-10-14 にユーザーが、同梱の候補にすると決めた。呼び出しは、ほかの言語と同じく JSON-RPC の別プロセスにする（本体のプロセスに動的ライブラリとして読み込む案は見送った。[ADR-0007](../adr/0007-macros-and-plugins.md) の「見送った案」）。同梱すれば、利用者が何も入れなくてもマクロを書ける言語ができる。たとえば lua.exe は約 0.5 MB で MIT（上の「Lua」）。気になる点: ADR-0007 の「処理系は同梱しない」の例外になる。同梱した版と利用者が入れた版のどちらを使うか、同梱した版の更新（脆弱性の修正など）を引き受けることになる
- Windows の Python の起動のしかた。Install Manager の `python` は、起動用のプロセスを通るので、毎回約 100 ms 遅い（「試作の結果」）。本体の場所を一度調べて覚えておくか、利用者が設定で本体を指すか
- マクロの実行中に、利用者の編集を止めるか（試作は止めていない）
- ブートを書き出す一時フォルダーの片付け。試作は終了を知らせた後で消すので、残ることがある（「試作の結果」）。終了を知らせる前に消す、起動時に古いものを消す、ブートはエディタの起動ごとに 1 回だけ書く、などを本実装で決める
- 試作のままにして、本実装で決めること（2026-10-14、ユーザー）
  - 「マクロの中断」のメニューの名前と位置（サクラのメニューにない名前で、試作で付けた）
  - `MoveCursor` で行末より大きい桁を指定したとき、行末に止めるか、フリーカーソルのように保つか（試作は行末に止める）。サクラの実機で確かめる
  - 引数の型の厳しさ（試作は、整数でない数や真偽値を -32602 にした。サクラはもっと緩いかもしれない）
  - Windows で、ブートを書き出す一時フォルダーのアクセス権（試作は、ユーザーごとの一時フォルダーに作るだけ。Unix ではモード 0700 にした）

## PoC の順番

1. （済み）試作のエディタ（Rust）から Node.js と Python を起動し、同期の往復を確かめる。遅れた応答、分割して届くデータ、EOF、UI が止まらないこと、タイムアウト
2. （一部）RS の目印が崩れる条件。改行のない出力は確かめた。子プロセスの出力は、まだ
3. WSH と PowerShell 5.1（文字コード、JSON の深さ）
4. AppleScript（Mac のセッションで）
5. （一部）エラーの位置は確かめた。範囲の読み書きの速さと、Undo のまとまりは、まだ

1 と、2・5 の一部は、`poc/gpui-textview` で行った（「試作の結果」）。3 と 4 は、まだ行っていない。

## 出典

サクラエディタのマクロ API（上流 https://github.com/sakura-editor/sakura ）

- <https://github.com/sakura-editor/sakura/blob/master/sakura_core/macro/CSMacroMgr.cpp>
- <https://github.com/sakura-editor/sakura/blob/master/sakura_core/macro/CMacro.cpp>
- <https://github.com/sakura-editor/sakura/tree/master/help/macro/source/reference>

JSON-RPC と Node.js

- JSON-RPC 2.0: <https://www.jsonrpc.org/specification>
- Node.js の issue #35997（EOF と EAGAIN）: <https://github.com/nodejs/node/issues/35997>

Windows の Python

- Python Install Manager: <https://docs.python.org/3/using/windows.html>
- Python Install Manager 26.4: <https://github.com/python/pymanager/releases/tag/26.4>
- PEP 686（UTF-8 モードを既定にする）: <https://peps.python.org/pep-0686/>

Lisp の調査

- <https://www.sbcl.org/all-news.html>
- <https://www.sbcl.org/platform-table.html>
- <https://ftp.gnu.org/gnu/emacs/windows/emacs-31/>
- <https://packages.msys2.org/base/mingw-w64-emacs>
- <https://download.racket-lang.org/releases/9.3/>
- <https://github.com/shirok/Gauche/releases>
- <https://github.com/babashka/babashka/releases>
- <https://github.com/hylang/hy/releases>
- <https://github.com/mattwparas/steel>
- <https://github.com/helix-editor/helix/pull/8675>
- <https://www.gnu.org/software/guile/news/gnu-guile-3011-released.html>
- <https://github.com/cisco/ChezScheme/releases>
- <https://github.com/Clozure/ccl/releases>
- <https://ecl.common-lisp.dev/>
- <https://github.com/babashka/nbb>
- <https://github.com/emacs-mirror/emacs/blob/emacs-31/src/minibuf.c>
