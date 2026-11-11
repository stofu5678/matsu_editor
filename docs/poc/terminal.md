# ターミナル（シェルのタブ）の下調べ（ADR-0028）

調査日: 2026-11-11。読んだだけで、ビルドや実行はしていない。Zed の `terminal` と `terminal_view`（GPL）は読んでいない。読んだものとライセンスは次のとおり。

| 読んだもの | ライセンス |
|---|---|
| `alacritty_terminal` / Alacritty のソースと CHANGELOG（github.com/alacritty/alacritty） | Apache-2.0（crates.io の license 欄） |
| `vte`（github.com/alacritty/vte の CHANGELOG） | Apache-2.0 OR MIT |
| `portable-pty`・WezTerm の `pty/`、`term/` の設定の定義（github.com/wezterm/wezterm） | MIT（LICENSE.md、Wez Furlong） |
| `vt100` の README、`libghostty-vt` の docs.rs | MIT / MIT OR Apache-2.0 |
| GPUI 本体（zed-industries/zed の `crates/gpui`。`window.rs`、`text_system.rs`、`text_system/line.rs`、`line_layout.rs`） | Apache-2.0（`crates/gpui/Cargo.toml`） |
| 生成 AI ツールと VS Code と Microsoft の文書 | 文書（読んだだけ） |
| このリポジトリ | MIT OR Apache-2.0 |

「推測」と書いたものは確かめていない。「未確認」は、読めなかった、または試していないもの。

## 1. `alacritty_terminal`

### 版・ライセンス・活動

- 最新 0.26.0（2026-04-06）。crates.io の license は Apache-2.0 のみ。作成 2020-07-31。総ダウンロード約 260 万、44 版
- 版の履歴: 0.24.2（2025-01-13）、0.25.0（2025-02-16）、0.25.1（2025-10-18）、0.26.0（2026-04-06）。2026 年の版は 0.26.0（と rc1、2026-03-21）の 1 つだけ。Alacritty 本体は v0.17.0（2026-04-06）
- `alacritty_terminal/` の最近のコミット（GitHub API、コミット日）: 2026-11-06 Web Assembly ビルド対応、2026-10-16 Kitty キーボードのスタックの上限修正、2026-08-31 行の列数の上限を外す、2026-08-26 ゼロ幅文字の無制限の増加を修正、2026-08-17、2026-06-16 PTY の非ブロッキング化の失敗を握りつぶさない、2026-03-05 シグナル終了時の後始末、2026-01-29 終了状態を `ExitStatus` に。つまり**月に 1 回前後は動いていて、バグ修正が中心。0.26.1（未リリース）の変更もたまっている**。開発版の番号は 0.18.0-dev（Alacritty 本体）
- CHANGELOG（`alacritty_terminal/CHANGELOG.md`）: 0.26.0 は `tty::Options::escape_args`（Windows のシェル引数のエスケープ）の追加、macOS で `~/.hushlogin` があれば `login -q`、終了通知が `ExitStatus` を持つ破壊的変更。0.25.0 は `Options::hold` が `drain_on_exit` に。**マイナー版ごとに破壊的変更がある**ので、`=0.26.0` のように固定して上げるのがよい（GPUI を `=` で固定しているのと同じ）
- 最小の Rust は 1.85.0（workspace の `rust-version`）

### 依存（0.26.0、crates.io API の dependencies）と `deny.toml` の許可リスト

通常の依存と、各クレートの最新版のライセンス（crates.io API）:

| 依存 | 条件 | ライセンス | 今の `Cargo.lock` |
|---|---|---|---|
| base64 ^0.22 | 常に | MIT OR Apache-2.0 | 0.22.1 あり |
| bitflags ^2.4.1 | 常に | MIT OR Apache-2.0 | 2.13.2 あり |
| home ^0.5.5 | 常に | MIT OR Apache-2.0 | なし（新規） |
| libc ^0.2 | 常に | MIT OR Apache-2.0 | あり |
| log ^0.4 | 常に | MIT OR Apache-2.0 | あり |
| parking_lot ^0.12 | 常に | MIT OR Apache-2.0 | 0.12.5 あり |
| polling ^3.8 | 常に | Apache-2.0 OR MIT | 3.11.0 あり |
| regex-automata ^0.4.3 | 常に | MIT OR Apache-2.0 | 0.4.18 あり |
| unicode-width ^0.2 | 常に | MIT OR Apache-2.0（MATSU でも使用中） | 0.2.3 あり |
| vte ^0.15 | 常に | Apache-2.0 OR MIT | なし（新規） |
| rustix ^1 | unix | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 1.1.5 あり |
| rustix-openpty ^0.2 | unix | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | なし（新規） |
| signal-hook ^0.4.3 | unix | MIT OR Apache-2.0 | なし（新規）。`signal-hook-registry` 1.4.8 は あり |
| miow ^0.6 | windows | MIT OR Apache-2.0 | なし（新規） |
| piper ^0.2.1 | windows | MIT OR Apache-2.0 | 0.2.5 あり |
| windows-sys ^0.59 | windows | MIT OR Apache-2.0 | 0.59 はなし（0.48 / 0.52 / 0.60.2 / 0.61.2 はある。別の版が増える） |
| serde ^1 | 任意（`serde` 機能） | MIT OR Apache-2.0 | 不要 |

- `vte` の依存: arrayvec（MIT OR Apache-2.0、あり）、memchr（Unlicense OR MIT、あり）、任意で bitflags、cursor-icon（MIT OR Apache-2.0 OR Zlib、**ロックになし**。`ansi` 機能で入る。推測）、log、serde
- `rustix-openpty` の依存: errno（MIT OR Apache-2.0、あり）、libc、rustix
- **どれも `OR` の片方が `deny.toml` の許可リスト（MIT / Apache-2.0 / Zlib ほか）に入っているので、ライセンスの検査は通る見込み**。GPL 系は含まれない。`alacritty_terminal` 本体の Apache-2.0 も許可済み。実際の `cargo deny check` は走らせていない（下調べでは何もビルドしていない）。実装の PR で確かめること
- 新しく増えるクレート: `vte`、`rustix-openpty`、`signal-hook`、`miow`、`home`、（`cursor-icon`）。`windows-sys` の 0.59 は、`multiple-versions = "warn"` なので警告だけ。`bans.wildcards = deny` には当たらない（版指定がある）。助言（advisories）は未確認
- 既存の `deny.toml` の `[licenses].allow` には変更が要らない見込み

### 子プロセスの起動

- Unix（`tty/unix.rs`）: `openpty` で PTY の組を作り、`std::process::Command` で起動する。`pre_exec` で `setsid()`、スレーブを制御端末にする `ioctl(TIOCSCTTY)`、標準入出力をスレーブに。マスター側には `IUTF8` を立てる。終了は SIGCHLD を pipe 経由で受け（`signal-hook`）、`try_wait()` で確かめる。`Drop` で子に SIGHUP を送って `wait()`。サイズ変更は `ioctl(TIOCSWINSZ)`（失敗すると `die!` でプロセスごと落ちる。これは気になる点。推測だが、実際に失敗する状況は少ない）
- 規定のシェル: 環境変数 `USER`/`HOME`/`SHELL`、なければ `getpwuid_r`。`Options.shell`（`Shell::new(program, args)`）で上書きできる
- macOS: シェルを `/usr/bin/login -flp <user>`（`.hushlogin` があれば `-qflp`）の下で起動する（ログインシェル）。**#364 の `login_path`（Finder 起動時の PATH）は、このログインシェルが自分で PATH を作るので、端末の中では要らない可能性が高い**。ただし `login` の起動はユーザーの `.zprofile` を読むので、端末タブの起動が重い設定では遅くなる（推測）
- 環境変数: `ALACRITTY_WINDOW_ID`、`USER`、`HOME`、`WINDOWID`、`Options.env` を設定。`XDG_ACTIVATION_TOKEN`、`DESKTOP_STARTUP_ID` を外す。`tty::setup_env()`（`TERM` = `alacritty`（terminfo があれば）または `xterm-256color`、`COLORTERM=truecolor`）は `unsafe { env::set_var }` で**自プロセスの環境を書き換える**関数。呼ぶのは Alacritty 本体で、`tty::new` が呼ぶかは未確認（`unix.rs` の取得結果には `TERM`/`COLORTERM` を設定する行がなかった）。**MATSU は `login_path` のコメントにある通り「`set_var` は他のスレッドと安全でない」方針なので、`setup_env` は呼ばず、`TERM=xterm-256color` と `COLORTERM=truecolor` を `Options.env` で渡すのがよい**
- Windows（`tty/windows/conpty.rs`）: ConPTY。`CreatePseudoConsole`・`ResizePseudoConsole`・`ClosePseudoConsole` を、まず **`LoadLibraryW("conpty.dll")`** で探し、なければ OS の（静的リンクの）API を使う。winpty には落ちない。匿名パイプ 2 本、`STARTF_USESTDHANDLES` を null で渡して**親のハンドルを子に継がせない**。環境変数は `Options.env` を足してから親の環境を足す（大文字小文字を無視して重複を除く）。作業フォルダは `Options.working_directory`
- 最小の Windows: コードに記述なし。Microsoft の文書（`CreatePseudoConsole`）では「Windows 10 October 2018 Update（1809）のデスクトップアプリ」「Windows Server 2019」。Windows 11 ARM64 は問題なし
- `conpty.dll` を exe の隣に置けば（Windows Terminal の OpenConsole 由来。MIT。推測、ライセンスは未確認）新しい ConPTY が使える。WezTerm の `pty` も同じ作り（`kernel32` から取り、exe の隣の `conpty.dll` があればそれを使う）

### イベントループとスレッド（`event_loop.rs`）

- `EventLoop::spawn` が **"PTY reader" という名前の 1 スレッド**を起こす。PTY の読み書き、`vte` による構文解析、`Term` の更新はここ。UI スレッドは `Notifier`（`mpsc::Sender<Msg>` と `Poller` の通知）で `Msg::Input(bytes)` / `Msg::Resize` / `Msg::Shutdown` を送る
- 読み取りバッファは 1 MiB（`READ_BUFFER_SIZE`）、1 回のロックで解析するのは最大 `u16::MAX` バイト。`Term` は `FairMutex<Term<U>>`。読み取りスレッドは `try_lock_unfair()` で取れなければバッファに貯め続け、いっぱいになってから `lock_unfair()`。**UI スレッドが描画のために `Term` をロックしている間は、読み取りが止まらずに貯まる設計**
- 同期更新（CSI ?2026）: `parser.sync_timeout()` が期限を返し、ループのタイムアウトに使われる。期限が来たら強制的に更新を終えて `Wakeup` を送る。ただし `Term` 側では `NamedPrivateMode::SyncUpdate` は何もしない扱いで、モードの問い合わせ（DECRQM）には「リセット」と答える。つまり**同期更新の「バッファして一度に反映」は構文解析器側（`vte::ansi::Processor`）が行うが、ツールが DECRQM で対応を確かめる場合は「未対応」と見える（推測。要確認）**
- ホストへの通知は `EventListener::send_event`。`Event`: `Wakeup`（再描画）、`Title`、`ResetTitle`、`Bell`、`ClipboardStore`（OSC 52 の書き込み）、`ClipboardLoad`（OSC 52 の読み取り。ホストが応答を PTY に書く）、`ColorRequest`、`PtyWrite`、`TextAreaSizeRequest`、`CursorBlinkingChange`、`MouseCursorDirty`、`Exit`、`ChildExit(ExitStatus)`
- 終了: `Msg::Shutdown`、子の終了（`drain_on_exit` で残りの出力を読み切る）。Linux の読み取りの EIO は子の終了待ちのため無視。`Drop` で子へ SIGHUP

### ホストに任されるもの

| 項目 | `alacritty_terminal` | ホスト（MATSU）が書く |
|---|---|---|
| 状態・エスケープシーケンスの解釈・グリッド | あり（`Term`、`Cell` は 24 バイト以内。`flags`、`fg`/`bg`、ゼロ幅文字は最大 9 個、下線色、ハイパーリンクは `Option<Arc<CellExtra>>`） | |
| スクロールバック | `Config.scrolling_history`（既定 10000 行、上限なし）。実行中の変更は`Term::set_options`（未確認） | 長さの既定と上限の決定 |
| リサイズ | `Term::resize`（列が変わると選択を消す）と `Msg::Resize(WindowSize)` | 画面の大きさ → 行・列の計算、両方への通知 |
| 選択 | `Term.selection`、`Selection`（Simple/Block/Lines/Semantic）、`selection_to_string` | マウス操作・描画・クリップボード |
| ダメージ追跡 | `Term::damage()`（行ごとの左右）。選択とカーソルは含まれない | 使うかどうか |
| 描画 | なし。`renderable_content()` 相当でセルを渡す（未確認の API 名） | すべて |
| キー入力 → バイト列 | なし。モード（`TermMode`）は参照できる | すべて（矢印、ファンクション、修飾、Kitty 形式） |
| マウスの報告 | モード（`MOUSE_REPORT_CLICK`/`MOUSE_MOTION`/`MOUSE_DRAG`/`SGR_MOUSE`/`UTF8_MOUSE`/`ALTERNATE_SCROLL`）は持つ | 実際の報告の符号化 |
| 括弧付き貼り付け | `BRACKETED_PASTE` を持つ | 貼り付け時に `ESC[200~ … ESC[201~` を付ける |
| IME | なし | すべて |
| OSC 52 | `Config.osc52`（既定 `OnlyCopy`。`Disabled`/`OnlyPaste`/`CopyPaste`） | イベントの扱い |
| ハイパーリンク（OSC 8） | `set_hyperlink`、セルに保持（Alacritty 0.11.0 で追加） | リンクの描画、開く操作 |
| Kitty キーボード | `Config.kitty_keyboard`（**既定 false**）。`DISAMBIGUATE_ESC_CODES`、`REPORT_EVENT_TYPES`、`REPORT_ALTERNATE_KEYS`、`REPORT_ALL_KEYS_AS_ESC`、`REPORT_ASSOCIATED_TEXT` のモード（0.13.0 で追加） | 符号化 |

## 2. 代替の候補

| 候補 | 版（日付） | ライセンス | 何をするか | ConPTY・Windows ARM64 | 備考 |
|---|---|---|---|---|---|
| `alacritty_terminal` | 0.26.0（2026-04-06） | Apache-2.0 | 状態＋構文解析＋PTY＋イベントループ | ConPTY あり。ARM64 は推測（下） | 上の通り |
| `portable-pty` | 0.9.0（2025-02-11）。crates.io は 1 年以上更新なし | MIT | **PTY だけ**（状態や構文解析はない）。`openpty` → `spawn_command` | ConPTY（`kernel32`、exe の隣の `conpty.dll` があればそれを使う。フラグ `INHERIT_CURSOR`/`RESIZE_QUIRK`/`WIN32_INPUT_MODE`）。0.9.0 は `winapi` 0.3 と `winreg`、`nix` 0.28、`serial2`、`shell-words` に依存。WezTerm の main は 2026-08-25 に `windows-sys` へ移行済みで、crates.io には未反映 | 状態の管理は別に要る（`vt100` か自作）。読み取りのスレッドも自分で書く。シェルの起動は `CommandBuilder`（環境変数の指定、`cwd`）で足りる |
| `vt100` | 0.18.1（2026-10-19）。0.17.0 は 2026-10-18 | MIT | 構文解析＋画面の状態。PTY はなし。`contents_formatted()`、`contents_diff()` | PTY を持たないので OS に依らない。依存は `vte` 0.15、`unicode-width` 0.2、`itoa` | README に、スクロールバック、代替画面、マウス、ハイパーリンクの記述はなし（未確認）。tmux/screen 型のプログラム向け |
| `termwiz` | 0.23.3（2025-03-20） | MIT | WezTerm の端末のライブラリ（文字セル、色、入力の符号化、`terminfo`）。端末の状態は `wezterm-term`（**crates.io にない**。git 依存が要る → `deny.toml` の `unknown-git = "deny"` に当たる） | PTY は `portable-pty`。依存は多い（`pest`、`phf`、`sha2`、`wezterm-*` など約 40 個、`nix` 0.29、`signal-hook` 0.3、`winapi`） | **曖昧幅の設定がある**（下、3 章）。ただし `wezterm-term` が公開されていないので、使うならソースの取り込み（MIT なので可能）になる |
| `libghostty-vt`（Ghostty の端末部分の Rust バインディング） | 0.2.0（日付は未確認） | MIT OR Apache-2.0 | 端末の状態（`Terminal`、`RenderState`、`KeyEncoder`）。PTY はなし | docs.rs が示すプラットフォームは Linux x86_64 のみ（Windows は未確認）。ハンドルは `!Send + !Sync`。0.2.0 の文書に「今後破壊的変更がある」。sys クレートは Rust 1.93 以上、edition 2024（`rust-toolchain.toml` との整合は未確認）。ネイティブのビルドが要るはず（推測） | 新しく、版が若い。ネイティブコードのビルドは CI の 5 環境で重い。今回は候補にしない |
| `wezterm-term` 単体 | 公開されていない | MIT | | | 上の `termwiz` の行 |
| Zed の `terminal` | | **GPL** | | | 読まない。使わない |

- **Windows ARM64**: どの候補も ARM64 向けの記述は見つからなかった。`alacritty_terminal` は Windows 向けに `windows-sys` 0.59（Microsoft の生成クレート）、`miow`、`piper` だけを使い、`cfg(windows)` の分は純 Rust と FFI なので、`aarch64-pc-windows-msvc` でビルドできる可能性は高い（推測）。Alacritty 本体の配布物は Windows x64 だけの様子（未確認）。Zed が Windows ARM64 版で `alacritty_terminal` を使っているという記憶があるが、確かめていない（Zed のコードは読まない決まり）。**実機（Windows ARM64、ADR-0009）で ConPTY を起動して確かめるのが確実**
- `portable-pty` + `vt100` の組み合わせは、PTY と状態が別々で、スレッドもイベントも自作になる。`alacritty_terminal` は「状態・PTY・イベントループ」が揃うので、作る量が少ない。代わりに曖昧幅などが選べない（3 章）
- `alacritty_terminal` の PTY だけ、または状態だけを使うこともできる（`Term` に `vte::ansi::Processor` でバイトを渡す形。`EventLoop` を使わず、`portable-pty` で起動する）。**この組み合わせの確認は未確認**

## 3. 日本語の扱い

### 幅

- `alacritty_terminal`: 文字の幅は `UnicodeWidthChar::width()`（`unicode-width` 0.2、**CJK でない版**）。つまり**東アジアの曖昧幅（Ambiguous）は 1 桁**。選ぶ設定はない（`Config` の項目にない、CHANGELOG に曖昧幅の記述なし）。全角は `WIDE_CHAR` + 次のセルに `WIDE_CHAR_SPACER`、行末で折り返すときの `LEADING_WIDE_CHAR_SPACER`。結合文字などのゼロ幅文字は、直前のセルの `zerowidth`（最大 9 個。2026-08-26 に上限が付いた）に足される。**文字ごとの幅の判定で、書記素クラスター（絵文字の ZWJ 列、異体字選択子 VS16 など）は 1 つの塊として扱わない**（推測。`input()` が 1 文字ずつ幅を見るため。未確認）
- 曖昧幅を変えるには、`alacritty_terminal` を取り込んで（Apache-2.0。改変したと書けば可）`input()` の幅の呼び出しを `width_cjk()` に替えるか、`WidthRule`（下）を呼ぶ形に変える必要がある。**外からは選べない**
- `vt100` 0.18: `unicode-width` 0.2 を使う（CJK 版かは未確認）。設定の有無は README にない
- `termwiz`/`wezterm-term`: `TerminalConfiguration::unicode_version()` が `UnicodeVersion` を返し、その `ambiguous_are_wide`（既定 false）で選べる。`unicode_version` の既定は 9（意図的に古い）、`cell_widths`（個別の幅の上書き）の項目もある。`unicode_column_width(s, version)` は書記素ごとに幅を数える。**ホストが曖昧幅を選べるのは、調べた中でここだけ**
- MATSU のエディタ: `crates/matsu-layout/src/width.rs` の `WidthRule`。`ambiguous_wide: bool`（既定 **true** = 曖昧は 2）。ほかに `JIS_LATIN1`（`§ ¨ ° ± ´ ¶ × ÷` を 2 桁）、ギリシャ・キリルの扱い、BMP 外は 2 桁、無効バイトは 2 桁、ゼロ幅・結合文字は 1 桁。`matsu-layout` / `matsu-coloring` / `matsu-ui` が `unicode-width` 0.2 に依存。`WidthRule::columns` が 1 文字の桁数を返す。**ユーザー向けの設定（matsu.ini など）は見つからなかった。`WidthRule::default()` を各所が使う**
- **エディタと端末は同じ規則にできない（合わせない方がよい）**: サクラ互換のエディタは曖昧幅を 2 にするが、生成 AI のコマンドライン・ツール（Node.js 製が多い）の文字幅の数え方は、私の知識では曖昧幅 = 1（`string-width` の既定。**今回は確かめていない**）。端末が曖昧幅 2 だと、`○ ● ■ ─ │` などの罫線や記号でツールの画面がずれる（推測）。**端末の既定は 1、設定で 2 に切り替えられるようにし、エディタの設定には追従させない、が安全**。罫線文字（U+2500 台）を全角で描くフォントが日本語環境にあるので、描画側でセル幅に合わせる（グリフを枠に押し込む）必要がある

### IME

- MATSU のエディタ: `crates/matsu-ui/src/view/input.rs` が GPUI の `EntityInputHandler`（`marked_text_range`、`replace_and_mark_text_in_range`、`bounds_for_range`、`character_index_for_point` など）を実装。変換中の文字は**文書の中に入れて（marked）**下線を付けて描く。UTF-16 の位置と行内の桁を変換する。候補ウィンドウの位置は `bounds_for_range` が返す矩形（`point_of`）で決まる
- 端末では、変換中の文字は PTY に送らず**ホストが保持**して、カーソルの位置のセルの上に重ねて描き（変換中は下線、桁幅は上の幅の規則）、確定したときだけ PTY にバイト列（UTF-8）で書く、が普通の作り。`alacritty_terminal` は IME を知らない。`EntityInputHandler` を端末のビューにも実装する（`bounds_for_range` は「カーソルのセルの矩形」を返す）。Alacritty 本体が IME を実装した記録: 0.7.0「Windows の IME 対応」、0.11.0「インライン入力メソッド」、0.13.0「変換中文字列の全角が切れる」修正、0.14.0「変換中の表示が下の文字に溶ける」修正。**同じ種類の落とし穴がある**ことの参考
- GPUI 側で、端末のビューが `EntityInputHandler` を `window.handle_input`（`window.rs` の `pub fn handle_input`）で登録する。`ElementInputHandler`（`element.rs` が使用中）と同じ作り

## 4. 生成 AI のコマンドライン・ツールが端末に求めるもの

文書の出典: Claude Code `https://code.claude.com/docs/en/terminal-config` と `…/fullscreen.md`、Codex CLI `https://learn.chatgpt.com/docs/config-file/config-reference`（旧 developers.openai.com）、Gemini CLI `docs/cli/settings.md`（google-gemini/gemini-cli）。

| 機能 | Claude Code | Codex CLI | Gemini CLI | 端末に必要か |
|---|---|---|---|---|
| 代替画面 | 「フルスクリーン」描画（`/tui fullscreen`）。既定で選ばれる条件は文書の表（2026-05-06 以降の新規は既定でフルスクリーン、など）。`CLAUDE_CODE_DISABLE_ALTERNATE_SCREEN=1` で従来型 | `tui.alternate_screen` = `auto`/`always`/`never`（`auto` は Zellij で使わない） | `ui.useAlternateBuffer`（既定 false） | あった方がよい。ないとフルスクリーン描画の画面が崩れる。**ある**（`ALT_SCREEN`） |
| マウスの報告 | フルスクリーンでマウスを捕まえる（クリック、ドラッグでの選択、ホイール）。`CLAUDE_CODE_DISABLE_MOUSE=1` で無効 | 記述を確かめられず（未確認） | 記述なし | あった方がよい。モードは**ある**。符号化はホストが書く |
| 同期出力（CSI ?2026） | 起動時に端末へ問い合わせ、対応していれば使う。tmux 3.6 までは未対応でちらつくと書いてある。`CLAUDE_CODE_FORCE_SYNC_OUTPUT=1` で強制 | 未確認 | 未確認 | あるとちらつかない。構文解析器は処理するが、DECRQM の応答に注意（1 章） |
| 括弧付き貼り付け | 3 行を超える／800 文字を超える貼り付けを `[Pasted text #N]` に畳む。貼り付けの中の指示は、入力した文面が頼むときだけ従うよう Claude に伝える | `tui.disable_paste_burst` の設定がある（貼り付けを連打として扱う仕組みの無効化。括弧付き貼り付けが使えない端末向けの作りと推測） | 未確認 | **ほぼ必須**。ないと貼り付けの改行が Enter になる |
| Shift+Enter | Kitty キーボード対応の端末で設定なしに動く（Alacritty 0.16 以降、foot など）。そのほかは Ctrl+J または `\` + Enter。VS Code と Zed は `/terminal-setup` が要る | `tui.keymap.*` で割り当てを変えられる | 未確認 | Kitty キーボードがあると便利。なくても Ctrl+J で足りる（Claude Code の文書） |
| 色 | テーマに `#rrggbb`、`ansi256(n)`、`ansi:<名前>` | 未確認 | 未確認 | 24 ビット色 + 256 色 + 16 色。端末側に `COLORTERM=truecolor` を出す |
| OSC 8 リンク | `Cmd`/`Ctrl` + クリックで URL とパスを開く（Claude Code 自身が処理）。VS Code などでは端末のリンク処理に任せる | 未確認 | 未確認 | 必須ではない |
| 通知 | Ghostty/Kitty/iTerm2 を検出するとデスクトップ通知（それぞれの OSC）。ほかは何もしない、または `terminal_bell`。OSC 7501「Program Status Protocol」に対応した端末なら状態を出す | `tui.notification_method` = `auto`/`osc9`/`bel` | 未確認 | ベル（`Event::Bell`）は受けて鳴らす。OSC 9 などは任意 |
| クリップボード | 選択を OS のクリップボードのツール、または **OSC 52** で書く（SSH 時）。iTerm2 は既定で OSC 52 を拒否 | 未確認 | 未確認 | OSC 52 の書き込みを受けるかは安全の判断（6 章） |
| リサイズ | 必要（画面の大きさに合わせて描く） | 同左 | 同左 | **必須**。`SIGWINCH`/ConPTY のリサイズ |
| Windows 固有 | **ConPTY 経由のホストは、差分描画の位置指定の書き込みをまとめて間違うことがあり、断片が残る**。`CLAUDE_CODE_ALT_SCREEN_FULL_REPAINT=1` で全部を描き直す。Windows では Backspace が `^H` で来ると Ctrl+Backspace と読むので `CLAUDE_CODE_BS_AS_CTRL_BACKSPACE=0` を使う | | | ConPTY の癖（5 章の注意） |
| `TERM_PROGRAM` | Ghostty などの検出に使う（`mintty` の特別扱いもある） | | | 端末は `TERM_PROGRAM` を設定しない、または独自の値にする（検出の誤りを避ける）。**MATSU 専用の値を設定してよいかは決め事** |

- 結論: **必須に近いのは、代替画面、256 色と 24 ビット色、リサイズ、括弧付き貼り付け、同期出力の構文解析**。マウスは任意（なくても使える。キーボード操作がある）。Kitty キーボードは後回しにできる（Ctrl+J がある）。OSC 8 は任意
- `alacritty_terminal` は上の「ある」をほぼ満たす。足りない・要確認なのは、DECRQM での同期出力の応答、Kitty キーボードの符号化（ホスト側）、OSC 9 などの通知（`Term` は未知の OSC を捨てるか、イベントにするかは未確認）、`TERM`（`xterm-256color` が安全。`alacritty` の terminfo は他の OS に入っていないので使えない）

## 5. GPUI でセルの格子を描く

### GPUI が持つもの（Apache-2.0 の `crates/gpui`、MATSU が使うのは `gpui-pre` 0.3.8 で、版の違いがありうる。未確認）

- `window.text_system().shape_line(text, font_size, &runs, force_width)` → `ShapedLine`。`TextRun`（長さ、フォント、色、背景色、下線、取り消し線）の列で、1 行を色分けして整形する。改行は含められない。`force_width: Option<Pixels>` は、文字ごとの送り幅を固定する引数（意味の詳細は未確認）。`ShapedLine::paint(origin, line_height, align, bounds, window, cx)` と `paint_background`、`width()`、`x_for_index`、`split_at`
- 1 グリフずつの描画 `Window::paint_glyph(origin, font_id, glyph_id, font_size, color)`、`paint_emoji`、`paint_quad(PaintQuad)`（背景色の矩形）、`paint_underline`、`paint_strikethrough`、`paint_path`、`paint_layer`
- **整形結果のキャッシュ**: `LineLayoutCache`（`line_layout.rs`）。前フレームと今フレームの 2 世代で、キーは「文字列・フォント・大きさ・runs」。同じ行を毎フレーム整形し直しても、中身が同じなら再利用される。`reuse_layouts` もある
- `TextSystem::advance(font_id, size, ch)`、`em_width`、`em_advance`、`ascent` でセル幅を測れる
- カスタム要素: `Element` を実装（`request_layout` → `prepaint` → `paint`）。MATSU は `EditorElement`（`crates/matsu-ui/src/view/element.rs`）で実績あり

### MATSU のエディタの描き方（`crates/matsu-ui/src/view/element.rs`、約 16000 行の `view/` のうち）

- 画面に見えている行だけを整形する（`shape`、`shape_styled`、`shape_number` のクロージャ。`window.text_system().shape_line(...)`）
- 1 桁の幅は、`"0000000000"` を整形して 10 で割った値（`half`）、`cell = column_width(half, settings.column_space)`。**位置はすべて桁数から画素に直す**ので、フォントが違ってもキャレットと選択が格子に乗る（ファイル先頭のコメント）
- 文字間隔があるとき（`spaced`）は 1 文字ずつ置く（`Char(Box<ShapedLine>)`）。色の塊（`TextStyle`、`styled_run`）ごとに `ShapedLine` を作る
- 背景・選択・キャレットは `paint_quad`（`fill`）。IME の下線、ルーラー、スクロールバーも自前

**端末が再利用できるもの**: `half` の測り方、`column_width`、`TextRun` の作り方、`EditorElement` の `request_layout`/`prepaint`/`paint` の骨格、`geometry.rs` の `Metrics`。**再利用できないもの**: `EditorView`（文書、折り返し、選択が文書の位置に結び付いている）。端末は別の `TerminalView` / `TerminalElement` を作る。

### 描画の費用の見積もり（測っていない。推測）

- 200 × 60 = 12 000 セル。行ごとに「同じ色・同じ属性の連続」を 1 つの `TextRun` にまとめれば、1 行あたり数個から数十個の run、全体で 60 回の `shape_line`
- キャッシュが効けば、変わっていない行の整形は省ける（キー一致）。変わった行だけ整形し直し。描画（グリフの敷き詰め）はフレームごとに全部やり直すが、GPU に渡す形（スプライト）で、12 000 セル程度は GPUI のエディタ本文（数千文字）の数倍で、**普通のフレーム（16 ms）には収まる見込み**。ただし測っていない。大量の出力（`cat` で流れる場合）では 1 フレームに全面が変わり、整形が 60 回走る。これは**Windows ARM64 の実機で測ること**。出力が速いときは、再描画を 1 フレームに 1 回に絞る（`Wakeup` を間引く）
- 全角文字・記号は 1 文字ずつ（または全角の塊ごとに）セルの格子に置く必要がある。フォントのフォールバックで送り幅がずれるため。そこが一番遅くなりうる。日本語を多く含む画面は、ASCII だけの画面より遅い（推測）
- セルの背景色は、同じ色が続く範囲ごとに `paint_quad` にまとめる

## 6. 安全の決めどころ

MATSU の現状（確認済み）:

- 外部コマンド実行（`F_EXECMD`）は、**マクロから確認なしで走る**（サクラ互換。`crates/matsu-ui/src/ui_commands/macros/execmd.rs`）。マクロは `matsu-script`（Lua・QuickJS・MicroPython・mruby ほか）と `matsu-plugin`（外部の言語のプラグイン）から呼べる
- プラグインの安全は ADR-0026 の「4. セキュリティ」で、既定オフ、プラグインごとに確認画面と有効化、変更の検出、API の制限なし（「防ぎ方は同意であり閉じ込めではない」）。このうち、セキュリティは ADR-0018 でユーザーが決める領域
- `matsu-platform/src/shell` は、OS の端末アプリを開く機能（PowerShell/cmd、Linux は最初に見つかった端末、macOS は `open` 系）。MATSU 自身の端末ではない
- `login_path` は「Finder・Dock から起動したとき」だけシェルに PATH を聞く（10 秒で打ち切り）。環境そのものは `std::env::set_var` で変えず、起動するプログラムに `Command::env("PATH", ..)` で渡す方針

決めること（選択肢と他製品の扱い）:

1. **何が端末に文字を送れるか**
   - 利用者のキー入力と貼り付け（当然）
   - マクロ・プラグインが「端末に文字を送る」API を持つか。持つなら、**確認なしに送れると、プラグイン 1 つで任意のコマンドを実行できる**。ただし今も `ExecCommand` で同じことができる（サクラ互換）。端末は**対話的なシェル**なので、送った文字が履歴に残り、実行中のプロセス（AI ツール）にも入る。選択肢: (a) API を作らない、(b) 作るが「送るだけ（改行を付けない）」「実行は利用者が Enter を押す」、(c) ADR-0026 の同意（プラグインごとの有効化）の中に入れる
   - 計画中の「選択範囲をシェルのワンライナーで処理」「選択範囲を生成 AI に送って次の行に答えを入れる」: 端末タブを使わず `exec`（パイプ）で足りる可能性が高い（端末は要らない。推測）。端末タブに送るなら、どのタブに送るか（複数あるとき）の指定が要る
   - **貼り付け**: 括弧付き貼り付けを使う。シェルが対応していないとき、貼り付けの改行が即実行される。**複数行の貼り付けの警告**を出すか（VS Code にはその設定がある。設定名は未確認）。貼り付けの文字列に `ESC[201~` が含まれると括弧付き貼り付けを抜けられる（端末の古典的な注入）ので、**貼り付けから ESC などの制御文字を除く**必要がある（一般的な知識。今回は他の製品の文書では確かめていない）
2. **ユーザーの操作なしに何かが走るか**
   - タブを開いたとき、ワークスペースやファイルの種類によって自動で端末を開く、特定のコマンドを自動で走らせる、は**作らない**ことを推奨したい（決めるのはユーザー）。VS Code の「制限モード」（Workspace Trust）は、**信頼していないフォルダーでは端末を開くこと自体を既定でブロック**し、タスクの実行・一覧表示、デバッグも止める（`terminal.integrated.allowInUntrustedWorkspace` で許可できる）。MATSU には「フォルダーを信頼する」という概念がまだない。ADR-0026 のプラグインの同意が一番近い
   - シェルの起動で `.bashrc` / `.zshrc` / PowerShell の `$PROFILE` が走る。作業フォルダの `.env` などを読むシェル設定があれば、フォルダを開いただけで走りうる（VS Code の文書にも、シェルが `.env` を読む場合の注意がある）
   - シェル統合（OSC 633 / OSC 133 / OSC 1337）: VS Code は起動時に引数や環境変数を使って各シェルに統合スクリプトを差し込む（bash/fish/pwsh/zsh、Windows は Git Bash と pwsh）。コマンドの位置・終了コード・作業フォルダが分かるようになる。偽造対策に、コマンド行を伝える `E` の列に**ナンス**を付けて確かめる。**今回の MATSU では、最初は作らない**。作るなら OSC 7（作業フォルダ）と OSC 133（プロンプト位置）だけが小さい候補（推測）
3. **セッションの記録**
   - 端末の出力をファイルに残すか。残すと、パスワードやトークンが平文で残る（`read -s` は画面に出ないが、`cat .env` などは出る）。**既定では記録しない、を推奨したい**（ユーザーの判断）
   - 保存するもの: スクロールバック（メモリ上、既定 10000 行は多め。VS Code の保存は `terminal.integrated.persistentSessionScrollback`、無効化は `enablePersistentSessions`）。**MATSU は未保存の文書を保存する仕組み（ADR-0022）があるが、端末の内容はそこに入れない**（子プロセスは再起動で復活できない）
   - シェルの履歴: シェル自身の履歴ファイル（`~/.zsh_history` など）が書く。MATSU は触らない。AI に送った文字列を MATSU が履歴として持たない
   - 利用者のデータ（保存・バックアップ・削除、`matsu.ini` の形式と置き場所）は ADR-0018 でユーザーが決める領域。端末の設定の置き場所はそこに該当
4. **環境変数**
   - 親（MATSU）の環境をそのまま渡す（`alacritty_terminal` の既定）か、一部を除くか。VS Code は `terminal.integrated.inheritEnv`（macOS・Linux で無効化できる）、`terminal.integrated.env` を持つ
   - MATSU が渡す値: `TERM=xterm-256color`、`COLORTERM=truecolor`、（`LANG` / `LC_ALL` が UTF-8 でないと日本語が崩れる。macOS を Finder から起動したときは `LANG` が空のことがある。推測）、macOS は `login` が `PATH` を作る。MATSU 固有の変数（`MATSU_*`。`login_path` には `MATSU_RESOLVING_ENVIRONMENT`）を子に渡さない方が安全。`XDG_ACTIVATION_TOKEN` と `DESKTOP_STARTUP_ID` は `alacritty_terminal` が除く
   - AI ツールが使う API キー（`ANTHROPIC_API_KEY` など）を、MATSU が**渡す・保存する**ことはしない（環境変数やシェルの設定から読まれるだけ）
5. **端末からの出力（エスケープシーケンス）の扱い**
   - **OSC 52（クリップボード）**: 書き込み（`OnlyCopy`）は既定。**読み取り（`ClipboardLoad`）を許すと、端末で動くプログラムがクリップボードを読める**。既定は書き込みだけ、または無効にするのが安全（`alacritty_terminal` の既定は `OnlyCopy`）。書き込みも、クリップボードを勝手に書き換えるので、通知を出すか（Claude Code は OSC 52 を SSH 時に使う）
   - **タイトル（`Event::Title`）**: タブの題に出す。長さを制限し、制御文字を除く（タブバーの描画が崩れる、だまし絵になる）
   - **応答の送り返し（`PtyWrite`、`ColorRequest`、`TextAreaSizeRequest`）**: 端末が応答を PTY に書くので、問い合わせの列を含む出力（`cat` した悪意あるファイル）で、シェルの入力にバイト列が入る古典的な問題（「エスケープシーケンスによる注入」）。`alacritty_terminal` は応答を `PtyWrite` で返す。Alacritty 本体の挙動をそのまま使うのが無難（推測）
   - OSC 8 のリンク: 表示と URL が違うことがある。開くときに URL を見せる、`file:` や任意のスキームは開かない、`http`/`https` だけ、など。Claude Code は UNC パスを資格情報の漏えいの恐れからリンクにしない（文書に記述）
6. **子プロセスの終了**: タブを閉じたときに、実行中のプロセスがあれば確認を出すか（ADR-0024 の決定 9「閉じる確認を種類ごとに決める」）。終了の扱いは `SIGHUP`（Unix）、`ClosePseudoConsole`（Windows。**出力を読み切るまでブロックしてデッドロックしうる**と `conpty.rs` のコメントにある）。MATSU の `exec::kill_tree`（プロセスグループ）を、端末の子に使うか
7. **ほかの OSS 製品の例**: VS Code の文書で確かめたのは、制限モード（上）、シェル統合のナンス、`inheritEnv`、永続セッションの設定。**ウィンドウを閉じたあとも子プロセスを生かす（tmux 型）かは決めない（作らない）ことを推奨**（推測。ユーザーの判断）

## 7. リポジトリで合わせるもの

- **タブの枠組み**（`crates/matsu-ui/src/tabs/content.rs`）: `TabContent`（今は `Editor(PaneSet)` と `Preview(PreviewTab)`）と、各種類が実装する `TabPanel`（`caption`、`tooltip`、`icon`、`accessible_name`、`focus_handle`、`menu_kind`、`status`、`close_check`）、`MenuKind`（`Editor`、`Preview`、テスト用の `Probe`。`MenuKind::ALL` に足す）。**端末は `TabContent::Terminal(..)` と `MenuKind::Terminal` を足す形**。`CloseCheck` は `None`/`UnsavedChanges` だけなので、「実行中のプロセスがある」の種類（例 `RunningProcess`）を足す必要がある。注意: ファイルの先頭コメントに「閉じる流れ（`file_ops::close`）は今も文書に直接問い合わせている」とある。プレビュー（`tabs/preview_tab.rs`、`preview/`）が最初の実例で、**窓が「見えない `EditorView`」（`hidden`）を持たせて、どの場所からも「ビューがある」ことにする作り**になっている。端末も同じ作りで済むかは未確認（端末には文書がない）
- 端末のタブを開く関数は、MATSU 独自の `F_` 関数として `ui_commands/` に 1 つ 1 ファイル（CLAUDE.md の「1 コマンド 1 ファイル」）。プレビューは `ui_commands/window/preview_tab.rs`
- **設定**（`matsu.ini`、サクラの形式。ADR-0010）: 新しい設定は「サクラにない節」に書く（ADR-0024 の決定 7）。端末のシェル、スクロールバックの行数、曖昧幅、OSC 52 の扱い、環境変数の除外の置き場。データベースは `databases.ini` を別に持つ前例（ADR-0023）。置き場所は利用者のデータの領域でユーザーの判断
- **日本語の幅**: 3 章の通り、`WidthRule` と端末の規則は別にして、曖昧幅の既定が違う
- **macOS の PATH（#364、`matsu-platform/src/login_path/`）**: `tags/mod.rs`、`exec/flow.rs`、`macros/mod.rs`、`shell/unix.rs` が `login_path::get()`/`wait()` を使う。端末は `login` 経由で自分のシェルを動かすなら PATH を渡す必要はない。端末を `login` なしで起動する場合は `login_path::get()` を `Options.env` の `PATH` に入れる。**`login -flp` が Finder 起動でも期待通りか、macOS（Tart のサンドボックス）で確かめること**。ADR-0027 の Mac の CI ランナーは、端末が子を起動するテストを動かせる
- Windows の端末: `matsu-platform/src/shell/` の `Shell`（`CommandPrompt`/`PowerShell`）が、既にシェルの種類を持つ。端末タブの既定のシェルはこれに合わせるか。`pwsh` の探し方（`find_in_path`）も使える
- ADR-0009: Windows ARM64 の実機で動かして確かめる。ConPTY の実機確認は、Windows サンドボックスで確かめられる
- 依存の追加（`alacritty_terminal`）は、CLAUDE.md により「ライセンスと依存」の領域で、`deny.toml` の許可リストに足すものがなくても、ユーザーに確認する。ADR（ADR-0024 の決定 5）に、すでに「別の ADR で決める」と書いてある
- クレートの置き場: 端末の状態と PTY は UI に依存しないので `crates/matsu-terminal`（新規）に置いて、`matsu-ui` が使う形が、依存の向き（UI → コア）に合う。ただし `alacritty_terminal` の `Term` は文書のバッファではなく、`matsu-core` に入れる理由はない（推測）。OS 固有の処理（PTY）は専用のモジュールに閉じ込める（CLAUDE.md）
- 文書は範囲指定でアクセスする規則: 端末の内容を「選択範囲として文書に挿入」するときは、全文の文字列にせず、範囲で渡す（スクロールバックは最大 10000 行 × 列）

## 候補の比較表（短く）

| | `alacritty_terminal` 0.26.0 | `portable-pty` 0.9.0 + `vt100` 0.18.1 | `termwiz`/`wezterm-term` | `libghostty-vt` 0.2.0 |
|---|---|---|---|---|
| ライセンス | Apache-2.0 | MIT + MIT | MIT（`wezterm-term` は crates.io にない） | MIT OR Apache-2.0 |
| PTY / ConPTY | あり（ConPTY は `conpty.dll` 優先） | `portable-pty` が持つ（`vt100` は無し） | `portable-pty` | なし |
| 端末の状態 | あり（スクロールバック、選択、ダメージ、ハイパーリンク、Kitty キーボードのモード） | `vt100`（スクロールバック等は未確認） | あり（`wezterm-term`） | あり（`RenderState`、`KeyEncoder`） |
| イベントループ | あり（1 スレッド） | なし（自作） | なし | なし |
| 曖昧幅の選択 | **なし**（取り込んで改変） | 未確認 | **あり**（`ambiguous_are_wide`） | 未確認 |
| 活動（2026） | 月 1 回前後のコミット、2026-04 に版 | `portable-pty` は 2025-02 から更新なし（WezTerm の main は 2026-09 まで動く）、`vt100` は 2026-10 に 2 版 | `termwiz` は 2025-03 が最後 | 新しい（0.1〜0.2） |
| 新しい依存の数 | 新規 5〜6（`vte`、`rustix-openpty`、`signal-hook`、`miow`、`home`、`cursor-icon`） | 多い（`winapi` 0.3、`nix` 0.28、`serial2`、`shell-words` ほか） | 約 40 | ネイティブのビルドが要る（推測） |
| Windows ARM64 | 推測で可。実機で確認 | `winapi` 0.3 は arm64 を持つはず（推測） | 未確認 | 未確認（Linux x86_64 だけ記載） |
| 作る量 | 小さめ | 大きい | 大きい（git 依存か取り込み） | 大きい |

## 答えられなかったこと（未確認・決められないこと）

1. `alacritty_terminal` を **Windows ARM64** でビルド・実行できるか（ConPTY の起動、サイズ変更、日本語）。実機で確かめるしかない
2. `alacritty_terminal` の `renderable_content()` などの描画用 API の名前と形（`Term::damage` と `Cell`、`Flags` は確認済み）。`Term::set_options` の有無
3. 同期出力（CSI ?2026）の DECRQM の応答が「リセット」になること（`Term` の `SyncUpdate` が空の処理）を、Claude Code・Codex・Gemini が実際にどう扱うか。Claude Code は端末に問い合わせて対応を確かめると書いてあるが、問い合わせの方法（DECRQM か、別の方法か）は文書にない
4. 曖昧幅: 生成 AI ツールが曖昧幅をどう数えるか（`string-width` の既定が 1、は私の知識）。実機で Claude Code / Codex を動かして罫線の揺れを確かめる
5. `alacritty_terminal` の `input()` が絵文字の ZWJ 列・VS16 をどう扱うか（1 文字ずつ見ると推測）
6. GPUI の `shape_line` の `force_width` の意味、`gpui-pre` 0.3.8 と Zed の main（読んだ版）の差
7. 描画の費用（12 000 セル）の実測。ARM64 の実機で、日本語を多く含む画面と `cat` の大量出力を測る
8. macOS での `login -flp` の起動の遅さと、Finder 起動での PATH・`LANG`
9. Codex CLI・Gemini CLI の端末機能（マウス、括弧付き貼り付け、同期出力）の記述。文書の範囲（Codex の設定の表の後半 6 万字）を読み切れていない
10. `libghostty-vt` の Windows 対応、ビルドの要件（Zig など）、ライセンスの全体
11. ユーザーが決めること（ADR-0018）: 依存の追加（`alacritty_terminal`、新しい 5〜6 クレート）、セキュリティ（マクロ・プラグインから端末に送る API の有無、OSC 52 の読み書き、自動実行をしないこと、環境変数）、利用者のデータ（記録しない、スクロールバックを保存しない、設定の置き場所）、`conpty.dll` / OpenConsole の同梱（配布物とライセンス）
