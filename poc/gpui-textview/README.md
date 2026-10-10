# GPUI テキスト表示の試作

[ADR-0004](../../docs/adr/0004-ui-framework.md) を確定するための、捨てる前提の試作。結果は [docs/poc/ui-framework-result.md](../../docs/poc/ui-framework-result.md)。

本体のワークスペースには含めていない。このディレクトリで `cargo run --release` する。Windows では Visual Studio の C++ ツール（ARM64 なら ARM64 用）が必要。Mac でも動く（下の「Mac」）。

- F7: フォントの切り替え、Ctrl+= / Ctrl+-: 文字の大きさ、F8: 100 万行のテキストを作る、Ctrl+O: 開く（Mac では Ctrl の代わりに ⌘）
- 選択とクリップボード（サクラの既定の割り当て）: Shift+矢印・Home・End・PageUp・PageDown、Ctrl+Shift+Home / End、マウスのドラッグ、Shift+クリックで選ぶ。Ctrl+A: すべて選択、Ctrl+X / Shift+Del: 切り取り、Ctrl+C / Ctrl+Ins: コピー、Ctrl+V / Shift+Ins: 貼り付け。選んでいないときのコピーと切り取りは、カーソルの行を改行ごと扱い、それを貼り付けるとカーソルの行の上に入る（サクラの「選択なしでコピー」と「ラインモード貼り付け」）。元に戻す（Ctrl+Z）はまだない
- 印刷（[ADR-0011](../../docs/adr/0011-printing.md)、[結果](../../docs/poc/printing.md)）: Ctrl+P で印刷、Ctrl+Shift+P で印刷プレビュー（Mac では Ctrl の代わりに ⌘。ファイル メニューにもある）。プレビューでは ← → / PageUp・PageDown でページ、Ctrl+= / Ctrl+- で拡大・縮小、Esc で戻る。プリンターに出せるのは Mac だけで、Windows と Linux はプレビューまで
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

Ubuntu 24.04 で確かめた。WSL2（WSLg）での準備は、下の「WSL2（WSLg）で動かす」に順にまとめた。

- F7 で切り替えるフォントは、Noto Sans Mono CJK JP、Noto Sans CJK JP、DejaVu Sans Mono、Noto Serif CJK JP
- タイトルバーを描かない Wayland（WSLg など）では、タイトルとボタンを試作が描く（gpui-component の `TitleBar`）。ウィンドウの端での大きさの変更はできない
- ファイルを開くダイアログ（Ctrl+O）には xdg-desktop-portal が要る。無いときは、下の行に理由を出す
- 日本語入力は、X11 で起動して、XIM で IME（fcitx5）とつなぐ。Wayland では GPUI は text-input-v3 で IME とやり取りするが、WSLg は IME（fcitx5）が Wayland に入力方式としてつながることを許さないので、この経路は使えない（下の 5）

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

#### 4. ファイルを開くダイアログ

```sh
sudo apt install xdg-desktop-portal xdg-desktop-portal-gtk
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
6. 印刷: 2 ページの文書に入れ替え、Ctrl+Shift+P でプレビューを開く（`preview-100.png`）。縮小して次のページへ移り（`preview-2.png`）、前のページに戻り（`preview.png`）、行番号と 2 段組にする（`preview-columns.png`）。Mac では、印刷パネルと同じ経路で PDF に出力し（A4 縦の `print.pdf` と A4 横の `print-landscape.pdf`。1 ページ目を `print-1.png` と `print-landscape-1.png` に）、Ctrl+P で印刷パネルを出して Esc で取り消す。最後に Esc でプレビューを閉じる
7. F8 で 100 万行を作り（`large.png`）、Mac では ⌘↓ で末尾へ移る（`large-end.png`）

Mac では、キーボードの種類（JIS など）を付けた macOS のキーイベントでキーを押すので、キーボード配列の読み方も確かめられる。それ以外では GPUI の `dispatch_keystroke` で押す。5 の選択のキーと、6 のプレビューの中のキーは、Mac でも `dispatch_keystroke` で押す。本物の日本語入力の確認の代わりにはならない。Mac で画面がロックされていると、ウィンドウがキー入力を受け取れないので失敗する。
