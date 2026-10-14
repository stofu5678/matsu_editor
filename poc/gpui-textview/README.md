# GPUI テキスト表示の試作

[ADR-0004](../../docs/adr/0004-ui-framework.md) を確定するための、捨てる前提の試作。結果は [docs/poc/ui-framework-result.md](../../docs/poc/ui-framework-result.md)。

本体のワークスペースには含めていない。このディレクトリで `cargo run --release` する。Windows では Visual Studio の C++ ツール（ARM64 なら ARM64 用）が必要。Mac でも動く（下の「Mac」）。

- F7: フォントの切り替え、Ctrl+= / Ctrl+-: 文字の大きさ、F8: 100 万行のテキストを作る、Ctrl+O: 開く（Mac では Ctrl の代わりに ⌘）
- 選択とクリップボード（サクラの既定の割り当て）: Shift+矢印・Home・End・PageUp・PageDown、Ctrl+Shift+Home / End、マウスのドラッグ、Shift+クリックで選ぶ。Ctrl+A: すべて選択、Ctrl+X / Shift+Del: 切り取り、Ctrl+C / Ctrl+Ins: コピー、Ctrl+V / Shift+Ins: 貼り付け。選んでいないときのコピーと切り取りは、カーソルの行を改行ごと扱い、それを貼り付けるとカーソルの行の上に入る（サクラの「選択なしでコピー」と「ラインモード貼り付け」）。元に戻す（Ctrl+Z）はまだない
- 印刷（[ADR-0011](../../docs/adr/0011-printing.md)、[結果](../../docs/poc/printing.md)）: Ctrl+P で印刷、Ctrl+Shift+P で印刷プレビュー（Mac では Ctrl の代わりに ⌘。ファイル メニューにもある）。プレビューでは ← → / PageUp・PageDown でページ、Ctrl+= / Ctrl+- で拡大・縮小、Esc で戻る。Windows は印刷ダイアログと GDI で、Linux は印刷のポータル（xdg-desktop-portal。下の「Linux」の 4）に PDF を渡して印刷する。Linux でポータルが使えないときは、PDF を一時ディレクトリ（`/tmp` など）に `ume-print-<ランダムな文字>.pdf` として保存し、その場所をプレビューに出す
- `--print-pdf <入力> <出力.pdf>`: サクラの既定の印刷設定と A4 で、PDF を作って終了する（Windows と Linux。Linux で印刷する PDF と同じもの）。`--print-test <入力> <出力.pdf>`: 印刷ダイアログを出さずに印刷して終了する。Windows では GDI で「Microsoft Print to PDF」（環境変数 `POC_PRINTER` で変えられる）に出し、Linux では PDF を作る
- 環境変数 `POC_PRINT_FONT=<フォントのファイル>[:<番号>]`: PDF に使うフォント。既定では、Linux は fontconfig で印刷のフォント（Noto Serif CJK JP）を探し、Windows は MS Mincho の `msmincho.ttc` を使う
- `cargo run --release --bin minimal`: メモリを比べるための最小のウィンドウ
- 環境変数 `POC_NO_MENU` / `POC_NO_ROOT` / `POC_NO_TEXT` / `POC_UI_FONT=<フォント名>`: メモリを測るための切り替え
- 環境変数 `POC_MEMSTAT=<秒>`: 起動してその秒数が経ったら、メモリの内訳を `memstat-<プロセスID>.txt` に書き出す（Windows のみ。Rust のヒープ、Win32 のヒープ、領域の種類ごとの量、大きい確保の一覧）
- 環境変数 `RUST_LOG=info`: GPUI のログ（選ばれた GPU など）を標準エラーに出す
- 環境変数 `POC_TEXT_MODE=grayscale`: 文字をグレースケールで描く（既定は Windows の設定に従い、ClearType ならサブピクセル）

## GPU ドライバーでメモリが増える問題の回避

Qualcomm Adreno（Windows ARM64）では、GPUI が文字をテクスチャに書き込むたびにメモリが約 4 MB 増える（[結果](../../docs/poc/ui-framework-result.md) を参照）。`patches/gpui-pre-windows-atlas-upload.diff` で回避できる。試すには次のようにする。

1. `~/.cargo/registry/src/*/gpui-pre-windows-0.3.8` を任意の場所に写し、そこで `patch -p1 < .../patches/gpui-pre-windows-atlas-upload.diff`
2. `Cargo.toml` の末尾に `[patch.crates-io]` と `gpui-pre-windows = { path = "<写した場所>" }` を足してビルドする（コミットはしない）

## Mac

Windows と同じコードで動く（2026-10-10 から）。Mac に合わせて次のようにした。

- Windows の Ctrl は ⌘ になる（⌘O、⌘= / ⌘-、⌘Q）。⌘+ でも大きくなる。JIS キーボードでは、= は ⇧-、+ は ⇧; で押す。文字の大きさは、Windows と同じ ⌃= / ⌃- でも変えられる
- Mac のキーボードの F7・F8 は、fn を押しながら押す。fn なしでは、明るさや音量、再生などのキーになる
- Mac の作法のキー: ⌘H（隠す）、⌥⌘H（ほかを隠す）、⌘M（しまう）、⌘← / ⌘→（行頭 / 行末）、⌘↑ / ⌘↓（先頭 / 末尾）
- メニューは画面上部のメニューバーに出し、先頭に「終了」などのあるアプリのメニューを置いた。ウィンドウの中のメニューバーと、Alt キーでのメニュー操作は使わない（Mac では Option で文字を入力するため）
- F7 で切り替えるフォントは、BIZ UDゴシック、Osaka、Menlo、ヒラギノ角ゴシック。GPUI はファミリー名でフォントを選ぶので、Osaka は等幅でない方になる
- `Cargo.toml` で、Mac のときだけ GPUI の `font-kit`（ないと文字が 1 つも描かれない）と `runtime_shaders`（シェーダーを起動時にコンパイルするので、Xcode の Metal Toolchain が要らない）を有効にしている。ビルドには Xcode か Command Line Tools が要る
- `POC_MEMSTAT` は Rust のヒープだけを書き出す。メモリは、アクティビティモニタの「メモリ」か、自己確認モードの出力で見る

## Linux

Ubuntu 24.04（WSLg）と Ubuntu 26.04.1（Hyper-V の仮想マシンの GNOME の Wayland）で確かめた。WSL2（WSLg）での準備は、下の「WSL2（WSLg）で動かす」に順にまとめた。

- F7 で切り替えるフォントは、Noto Sans Mono CJK JP、Noto Sans CJK JP、DejaVu Sans Mono、Noto Serif CJK JP
- タイトルバーを描かない Wayland（WSLg、GNOME など）では、タイトルとボタンを試作が描く（gpui-component の `TitleBar`）。ウィンドウの端での大きさの変更は、GNOME ではできた（gpui-component の `Root` が付ける枠 `window_border` が受け持つ）が、WSLg ではできなかった
- ファイルを開くダイアログ（Ctrl+O）と印刷ダイアログ（Ctrl+P）には xdg-desktop-portal が要る。無いときは、ファイルを開くダイアログは下の行に理由を出し、印刷は PDF を保存する
- 日本語入力: GNOME の Wayland では、GPUI が text-input-v3 で IME（IBus と Mozc）とつなぐ（[結果](../../docs/poc/ui-framework-result.md)の「Hyper-V の Ubuntu（Wayland）での確認」）。WSLg は IME（fcitx5）が Wayland に入力方式としてつながることを許さないので、X11 で起動して、XIM で IME とつなぐ（下の 5）

### WSL2（WSLg）で動かす

2026-10-10 に、Windows 11（x64）の WSL2 の Ubuntu 24.04 で、この順に準備して動いた。

#### 1. WSL を新しくする

Windows の PowerShell で次を実行する。`wsl --shutdown` で WSL が止まるので、WSL の中で動いているもの（ターミナルや Claude Code）は先に閉じる。

```powershell
wsl --update
wsl --shutdown
```

WSL 2.6.1.0（WSLg 1.0.66）では、ウィンドウがタスクバーに出るだけで、中身が何も描かれなかった。`/mnt/wslg/weston.log` に `rdp_allocate_shared_memory: Failed to open "/mnt/shared_memory/{...}" with error: Input/output error` が出続けていた。WSL 3.0.1.0（WSLg 1.0.79）に更新すると直った。

#### 2. Rust（asdf）

Rust の版は、リポジトリの `rust-toolchain.toml` と `.tool-versions` で 1.99.0 に固定している。[asdf](https://asdf-vm.com/) で入れる場合は次のとおり。rustup で入れてもよい。

```sh
asdf plugin add rust https://github.com/code-lever/asdf-rust.git
ASDF_RUST_PROFILE=minimal asdf install rust 1.99.0
```

rustfmt と clippy は、`rust-toolchain.toml` の `components` から入る。

#### 3. ビルドに要るパッケージ

```sh
sudo apt install build-essential libfontconfig-dev libfreetype-dev libxcb1-dev \
  libxkbcommon-dev libxkbcommon-x11-dev fonts-noto-cjk
```

x64 では、これでビルドでき、日本語も出た（`fonts-noto-cjk` がないと日本語が出ない）。ARM64 の WSL では、ほかに `clang cmake libasound2-dev libwayland-dev libx11-xcb-dev libzstd-dev` も入れて確かめている。

#### 4. ファイルを開くダイアログと印刷ダイアログ

```sh
sudo apt install xdg-desktop-portal xdg-desktop-portal-gtk
```

これで、印刷ダイアログの「ファイルに出力」で PDF に印刷できる。紙に印刷するには、Linux の印刷の仕組み（CUPS）を入れて、プリンターを登録する。WSL の Linux には Windows のプリンターが見えず、WSL（NAT）からはネットワークのプリンターも自動では見つからないので、アドレスで登録する。次は、ドライバーなしの印刷（IPP Everywhere）に対応したネットワークのプリンターの場合（2026-10-11 に EPSON EW-M873T で確かめた）。

```sh
sudo apt install cups
sudo lpadmin -p <名前> -E -v ipps://<プリンターのアドレス>:631/ipp/print -m everywhere
```

#### 5. 日本語入力（fcitx5 と Mozc）

入れて、入力方式に Mozc を足す。US 配列のキーボードなら、`jp` を `us` にする。

```sh
sudo apt install fcitx5 fcitx5-mozc
mkdir -p ~/.config/fcitx5
cat > ~/.config/fcitx5/profile <<'PROFILE'
[Groups/0]
Name=Default
Default Layout=jp
DefaultIM=mozc

[Groups/0/Items/0]
Name=keyboard-jp
Layout=

[Groups/0/Items/1]
Name=mozc
Layout=

[GroupOrder]
0=Default
PROFILE
```

fcitx5 は、Wayland の機能を外して起動する。WSL を起動するたびに要る。動いているかは `pgrep -a fcitx5` で見る。

```sh
WAYLAND_DISPLAY= fcitx5 -d --disable=wayland,waylandim
```

そのまま `fcitx5 -d` で起動すると、WSLg が IME 用の Wayland の接続を断り（`zwp_input_method_v1@10: error 0: permission to bind input_method denied`）、fcitx5 はそれをエラーとして終了してしまう。`-d` で裏で起動するので、このエラーは画面に出ない。

#### 6. 起動

```sh
cd poc/gpui-textview
cargo build --release
XMODIFIERS=@im=fcitx WAYLAND_DISPLAY= ./target/release/gpui-textview-poc
```

- `WAYLAND_DISPLAY=` で X11 を使わせ、`XMODIFIERS=@im=fcitx` で XIM の相手を fcitx5 にする。`RUST_LOG=info` を付けると、`Try connect fcitx` と `XConnected server` の行で、つながったことが分かる
- 日本語入力は「半角/全角」キーで切り替える
- 日本語入力が要らなければ、`./target/release/gpui-textview-poc` だけで Wayland で動く

## マクロ（JSON-RPC）

別プロセスの JavaScript（Node.js）か Python をマクロとして走らせ、マクロがエディタのコマンドを JSON-RPC で同期的に呼ぶ試作（Phase 0 の「JSON-RPC を試作する」）。通信の仕様は [docs/poc/json-rpc.md](../../docs/poc/json-rpc.md) にあり、この試作はその案 v0 に沿う。試作の結果も、そこにまとめた。

### 動かし方

- ツール メニュー > 「名前を指定してマクロ実行...」（サクラのメニューの文言）で `.js` か `.py` を選ぶ。実行中は、ウィンドウの下に出力パネルが開き、マクロの出力、標準エラー、終了の状態（正常終了、終了コード、時間切れ、中断）が出る。パネルの「閉じる」で隠せる。UI は止まらない（プロセスの入出力は別のスレッドで行い、マクロの呼び出しだけを UI のスレッドで 1 つずつ実行する）
- ツール メニュー > 「マクロの中断」: 実行中のマクロのプロセスを終わらせる（サクラのメニューにはない名前で、この試作で付けた）
- `macros/sample.js` と `macros/sample.py`: 日本語と絵文字を使うサンプル
- ウィンドウなしの実行: `gpui-textview-poc --run-macro <マクロ> <入力のテキスト> <結果のテキスト>`。マクロの出力は標準出力と標準エラーに出し、結果の文書を書き、マクロが正常に終わったときだけ終了コード 0 で終わる。文書は、ウィンドウなしの実行では `TextModel`（src/macro_host/model.rs）、ウィンドウでは `Editor` が持つ
- 環境変数 `POC_NODE`（既定は `node`）、`POC_PYTHON`（既定は Linux・Mac で `python3`、Windows で `python`）: 処理系の場所。起動できないときは、その旨と環境変数の名前を出力パネルに出す。Windows の `python` がストアのスタブ（WindowsApps）だと終了コード 9009 で終わるので、その旨も添える。Windows で Python Install Manager の `python`（アプリ実行エイリアス）を使うと、起動用のプロセスを通るので、起動が約 100 ms 遅い。`POC_PYTHON` に本体（例: `%LOCALAPPDATA%\Python\pythoncore-3.15-arm64\python.exe`）を指定すると避けられる
- 環境変数 `POC_MACRO_TIMEOUT=<秒>`（既定 30、小数可）: これを超えたらマクロのプロセスを終わらせる。エディタが応答する時間も含む
- マクロの起動: `node --require <boot.cjs> <マクロ>` と `python -u <boot.py> <マクロ>`（Python は環境変数 `PYTHONUTF8=1` つき）。Windows では `CREATE_NO_WINDOW` で起動する。ブートのファイルは実行ファイルに埋め込んであり、実行のたびに、一時ディレクトリ（`ume-macro-<ランダムな文字>`。Unix ではモード 0700。Windows ではユーザーごとの一時フォルダーに作るだけで、アクセス権は設定していない）に書き出し、終わったら消す。マクロの中に require や import を書く必要はない。`Editor`（と、エラー用の `EditorError`）がグローバルに置かれる

### コードの場所

- `src/macro_host/`: GPUI に依存しないマクロのホスト。`protocol.rs`（1 行の枠組み、ASCII へのエスケープ、メッセージ）、`api.rs`（基本 API の定義表 `FUNCS` と、サクラの意味での実装）、`session.rs`（子プロセス、スレッド、タイムアウト、中断）、`boot.rs` と `boot/`（ブートのライブラリ）、`model.rs`（テスト用のテキスト）、`tests.rs`（本物の Node.js と Python を走らせる統合テスト）
- `src/macro_ui.rs`: ウィンドウとのつなぎ（メニューの命令、マクロの呼び出しを `Editor` に対して実行する部分、出力パネル）
- API の定義（名前、引数の型、戻り値の型、説明）は `api.rs` の `FUNCS` の 1 か所。そこから `boot/api.gen.cjs`（JavaScript）と `boot/api_gen.py`（Python）を生成してコミットしてある。定義を変えて生成物が古くなると、`cargo test` の `generated_files_are_up_to_date` が落ちる。作り直すには `UME_UPDATE_GENERATED=1 cargo test generated_files` を実行する。JS の Proxy のような、どんな名前でも通す呼び出しにはしていない

### 基本 API と意味

名前と引数はサクラのマクロ API（`sakura_core/macro/CSMacroMgr.cpp`、ヘルプは `help/macro/source/reference/`）と同じ。意味は `CMacro.cpp` とそこから呼ばれるコマンドのソースで確かめた（ソースを読んだだけで、サクラ本体は動かしていない）。要求は `{"jsonrpc":"2.0","id":1,"method":"GetLineStr","params":[1]}` の形で、引数は配列。

| 関数 | 引数 | 戻り値 | 意味 |
|---|---|---|---|
| `GetLineCount(0)` | 予約（0 以外はエラー） | 整数 | 論理行（改行で区切った行）の数。最後の改行の後ろの空の行は数えない。空の文書は 0 |
| `GetLineStr(n)` | 行番号（1 から、0 はカーソルの行。負はエラー） | 文字列 | **改行を含む**行（`GetDocLineStrWithEOL`）。最後の行は、文書が改行で終わらなければ改行がない。行数より大きい番号は空文字列 |
| `InsText(s)` | 文字列 | なし | カーソルに挿入する。選択があれば置き換える。カーソルは挿入した文字列の後ろへ行く |
| `AddTail(s)` | 文字列 | なし | 選択を解いて文書の末尾へ移り、そこに挿入する |
| `GoFileTop()` / `GoFileEnd()` | なし | なし | 先頭 / 末尾へ移り、選択を解く |
| `Up()` / `Down()` | なし | なし | 1 行上 / 下へ移り、選択を解く |
| `SelectAll()` | なし | なし | すべて選択する |
| `GetSelectedString(0)` | 予約（無視される） | 文字列 | 選択した文字列（改行を含む）。選択がなければ空文字列 |
| `IsTextSelected()` | なし | 整数 | 0 未選択、1 選択中（サクラは矩形選択で 2 を返すが、試作に矩形選択はない） |
| `MoveCursor(行, 桁, オプション)` | 整数 3 つ | なし | 行と桁は **1 から**数える。オプション 0 は選択なし、1 は元の位置から選択する。2（矩形選択）は未対応でエラー。行か桁が 1 未満なら何もしない（サクラはビープ音）。最後の行より後ろは文書の末尾へ、行の長さより大きい桁は行末へ行く |
| `Jump(行, フラグ)` | 整数（フラグは省略できる） | なし | 行頭へ移る。行は 1 から。1 未満は先頭行、行数より大きいときは最後の行。フラグの bit 0 は改行単位（省略時は 1）。bit 1（PL/SQL）と折り返し単位（bit 0 が 0）は未対応でエラー |
| `GetFilename()` | なし | 文字列 | ファイルのフルパス。ファイルでなければ空文字列 |

サクラのソースとヘルプで分かったこと:

- `GetLineStr` は改行を含む。ヘルプには書かれておらず、`CMacro.cpp` の F_GETLINESTR が `GetDocLineStrWithEOL` を呼ぶことで分かった。`0` はカーソルのある行、`n` は n 行目、全行数より大きい値は空文字列（ヘルプ）
- 行番号は 1 から、0 は「カーソルの行」（`GetLineStr` のみ）。`MoveCursor` の行と桁もヘルプで「1 開始」
- 桁の単位は論理位置の文字数（C++ の `wchar_t`、Windows では UTF-16 のコード単位）。絵文字などの U+10000 以上の文字は 2 と数える。試作もこの単位にした。サクラの `MoveCursor` は、論理位置をレイアウト位置に直して移るので、桁の「何文字目か」と画面の桁（全角 2、タブの幅）は別。サクラの実機で確かめていない
- `GetLineCount` の引数と `GetSelectedString` の引数は予約。前者は 0 以外ならエラー、後者は無視される（ヘルプ）
- `Jump` のフラグ省略時は 1（改行単位）（`CMacro.cpp` の F_JUMP）
- サクラの `MoveCursor` で、列が行末より大きいときは行末に止めるのか、フリーカーソルのように保つのかは、ソースだけでは確かめきれなかった。試作は行末に止める（要確認）

サクラとの違い（試作の制約）:

- 文書は `\n` で行に分けていて、改行の種類（CR LF など）を覚えていない。`GetLineStr` と `GetSelectedString` の改行は `\n` になる
- 折り返しがないので、レイアウト単位の行番号はない
- 引数の型を確かめる（整数と文字列のみ。整数でない数や真偽値は -32602）

### 確かめたこと

2026-10-11 の確認（Windows の Python は 2026-10-14）。

- 単体テスト（`cargo test`）: 1 行の枠組み、RS での分割、行末の CR、ASCII へのエスケープ（サロゲートペアを含む）、要求の解析（-32700・-32600・-32602）、-32601、API の意味、生成物が古くなっていないこと、環境変数のタイムアウトの読み取り
- 本物の Node.js と Python を走らせる統合テスト（処理系がなければ「SKIP」を出して通る。`cargo test -- --nocapture` で見える）: 日本語と絵文字の往復 / エディタが 1.5 秒遅れて応答する / 1 MB を超える行（片道 2 MB 超の 1 行）の往復 / 改行なしの出力の直後に来る要求 / 構文エラーと実行時エラー（標準エラーに利用者のファイルの行番号が出る）/ エラー応答が例外になる（JS も Python も `EditorError`。`code` にエラーコードが入る）/ 無限ループと応答が来ない待ちがタイムアウトで止まる / すぐ終わるマクロ（EOF）/ 終了コード / Python が `PYTHONUTF8=1` で日本語を UTF-8 で出す / 標準エラー / 中断 / 処理系がないときのメッセージ / 不正な行（Python が生の行を送る）
- Linux（WSL2、aarch64、Node.js v25、Python 3.13）: `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`cargo build --release` が通った。`--features self-check` の自己確認に「マクロ」の段を足し、実際のウィンドウ（WSLg）で `sample.js`・`sample.py` の実行（文書の中身、出力パネルの内容、正常終了）と、終わらないマクロの中断を確かめた
- Windows ARM64（Node.js v24。2026-10-11 は Python が入っていなかった）: `cargo test --release` が通った。Python のテストは飛ばした。`--features self-check` の自己確認（実際のウィンドウ）も通り、`sample.js` と、終わらないマクロの中断を確かめた。`sample.py` は、`python` がストアのスタブ（終了コード 9009）だったので飛ばし、その旨の案内が出力パネルに出た
- Windows ARM64 の Python（2026-10-14）: Python Install Manager（26.4）で Python 3.15.0 の ARM64 版を入れた。`cargo test --release` の 56 件が通り、Python のテストも飛ばさずに通った。`--run-macro` の `sample.py` も正しかった。ユーザーが、メニューから `sample.js`・`sample.py` を実行し、文書と出力パネルが正しいこと、`Jump(1)` で 1 行目に戻ることを確かめた。終わらない Python のマクロを「マクロの中断」で止められ、`python.exe` が残らないこと、コンソールの窓が出ないことも確かめた
- 画面を見ての操作（メニュー、ファイルダイアログ、出力パネル）は、Windows でユーザーが確かめた（2026-10-14）。Linux と Mac では、人は見ていない
- 参考: Linux で `--run-macro` に `sample.js` と `sample.py` を渡したときの、起動から終了までの時間は、どちらも約 35 ミリ秒（1 回ずつ測っただけ）。Windows ARM64 では、`sample.js` が 103 ミリ秒、`sample.py` が 174 ミリ秒（Install Manager の `python` を通す）と 72 ミリ秒（本体を直接）だった（9 回の中央値、2026-10-14）

### 制約と、まだやっていないこと

- Undo がない（この試作のエディタに元に戻す機能がない）ので、マクロの編集を 1 回の Undo にまとめる件は確かめていない
- プラグインとイベント（エディタからマクロへの要求）はない。マクロが 1 回走って終わる形だけ
- 矩形選択、PL/SQL、折り返し単位の行番号、文書の版・複数の文書は扱わない。API はこの表の 14 個だけ
- マクロの実行中も、キーボードなどでテキストを編集できる。マクロの呼び出しの間に編集が割り込むことがある（マクロの間は編集を止める仕組みは作っていない）
- 中断とタイムアウトは、マクロのプロセスだけを終わらせる。マクロが起動した別のプロセスは残ることがある。出力パイプが開いたままの孫プロセスがあっても、終了から 1 秒でマクロの終了を表示する。なお、Windows の Python Install Manager の `python` は、起動用のプロセスが本体を子として起動するが、起動用を終わらせると本体も終わった（2026-10-14）
- ブートの一時フォルダー（`ume-macro-*`）は、マクロの終了を知らせた後で消す（消せなくてもエラーにしない）。知らせた直後にプロセスが終わると、消す途中で打ち切られて残る。Windows でテストを 1 つずつ別のプロセスで回したときに残った（2026-10-14）。ウィンドウでの実行では残らなかった
- マクロが大量に出力すると、出力の 1 行ごとに画面を更新する。UI が追いつかない分はメモリにたまる（行数の上限があるのは出力パネルの表示だけ）
- 改行なしの出力は、改行が来るかマクロが終わるまで出力パネルに出ない（RS で区切るのは、要求の直前に出た分だけ）
- Node.js のエラー応答の未処理の例外は、Node が最初に出す抜粋に `boot.cjs`（一時ディレクトリ）の行が出る。利用者のファイルの行は、続くスタックトレースに出る
- マクロのファイルの文字コードは、Node.js と Python の既定（UTF-8）。WSH などは第 2 弾
- macOS は、この PR では動かしていない（コードは標準ライブラリと共通の部品だけ）

## 自己確認モード

画面を見ずに、キーと表示を確かめるモード。キーを押し、ウィンドウの描画結果を PNG に保存して終了する。macOS の画面収録やアクセシビリティの許可は要らない。Mac、Windows x64、Linux x64（WSLg）で確かめた。Linux では GPUI がウィンドウを画像にできないので、画像の保存だけ飛ばす。

```sh
cargo run --features self-check -- --self-check /tmp/textview-check
```

次の順に試し、GPUI がキーをどう読んだか（例: `secondary-= = cmd-= → editor::FontBigger: 13 → 14 pt`）とメモリを出して、最後に `self-check: ok` を出す。

1. 起動直後を `opened.png` に保存する
2. F7 でフォントを一巡し、それぞれを `font-0.png`〜`font-3.png` に保存する
3. 文字の大きさのキー（Windows では Ctrl+= など。Mac では ⌘= などと ⌃= など）
4. IME と同じ呼び出しで「にほんご」を「日本語」に変換し（`composing.png`）、確定する
5. Shift+Home・Shift+←・Shift+↑ で選び（複数行の選択を `selection.png` に保存）、Ctrl+C・Ctrl+X・Ctrl+V・Ctrl+A と、選んでいないときの行のコピー・切り取り・貼り付けを試す。システムのクリップボードを使うので、中身は書き換わる。Wayland では、GPUI が本物のキー入力なしにはクリップボードを持てないので、この確認は飛ばす（`WAYLAND_DISPLAY=` を付けて X11 で走らせると確かめられる）
6. 印刷: 2 ページの文書に入れ替え、Ctrl+Shift+P でプレビューを開く（`preview-100.png`）。縮小して次のページへ移り（`preview-2.png`）、Ctrl+V が隠れた文書に貼り付けないことを確かめ、前のページに戻り（`preview.png`）、行番号と 2 段組にする（`preview-columns.png`）。Mac では、印刷パネルと同じ経路で PDF に出力し（A4 縦の `print.pdf` と A4 横の `print-landscape.pdf`。1 ページ目を `print-1.png` と `print-landscape-1.png` に）、Ctrl+P で印刷パネルを出して Esc で取り消す。Windows と Linux では、同じ文書を PDF に出力する（Linux で印刷するものと同じ `print.pdf`。Windows では、GDI で「Microsoft Print to PDF」にも出す `print-gdi.pdf`）。印刷ダイアログは出さない（GPUI の中に送るキーは、OS の印刷ダイアログに届かないので、取り消せない）。最後に Esc でプレビューを閉じる
7. マクロ: `macros/sample.js` と `macros/sample.py` を実行して、文書の中身と出力パネルの内容を確かめ（Node.js か Python がなければ、その実行は飛ばす）、終わらないマクロを「マクロの中断」で止める
8. F8 で 100 万行を作り（`large.png`）、Mac では ⌘↓ で末尾へ移る（`large-end.png`）

Mac では、キーボードの種類（JIS など）を付けた macOS のキーイベントでキーを押すので、キーボード配列の読み方も確かめられる。それ以外では GPUI の `dispatch_keystroke` で押す。5 の選択のキーと、6 のプレビューの中のキーは、Mac でも `dispatch_keystroke` で押す。本物の日本語入力の確認の代わりにはならない。Mac で画面がロックされていると、ウィンドウがキー入力を受け取れないので失敗する。ほかのアプリを使っていて、試作のウィンドウが前に出られないときも同じ（キーイベントは試作の中にだけ送るので、ほかのアプリには届かない）。
