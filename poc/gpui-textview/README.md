# GPUI テキスト表示の試作

[ADR-0004](../../docs/adr/0004-ui-framework.md) を確定するための、捨てる前提の試作。結果は [docs/poc/ui-framework-result.md](../../docs/poc/ui-framework-result.md)。

本体のワークスペースには含めていない。このディレクトリで `cargo run --release` する。Windows では Visual Studio の C++ ツール（ARM64 なら ARM64 用）が必要。Mac でも動く（下の「Mac」）。

- F7: フォントの切り替え、Ctrl+= / Ctrl+-: 文字の大きさ、F8: 100 万行のテキストを作る、Ctrl+O: 開く（Mac では Ctrl の代わりに ⌘）
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
5. F8 で 100 万行を作り（`large.png`）、Mac では ⌘↓ で末尾へ移る（`large-end.png`）

Mac では、キーボードの種類（JIS など）を付けた macOS のキーイベントでキーを押すので、キーボード配列の読み方も確かめられる。それ以外では GPUI の `dispatch_keystroke` で押す。本物の日本語入力の確認の代わりにはならない。
