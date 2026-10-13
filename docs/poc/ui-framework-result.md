# UI フレームワークの試作の結果（GPUI）

この結果で [ADR-0004](../adr/0004-ui-framework.md) を決定にした（2026-10-11）。Linux の確認は、ユーザーが Phase 0 の条件から外した。そのうち Wayland での日本語入力とデスクトップとの統合は、2026-10-14 に Hyper-V の Ubuntu で確かめた（下の「Hyper-V の Ubuntu（Wayland）での確認」）。GPU での描画は残っている。確かめ方は ADR-0004 の「残っている確認」。

[試作の手順](ui-framework.md) に沿った結果。試作は次の 2 つで、どちらもブランチ `poc/gpui` で作った。GPUI はどちらも crates.io の `gpui-pre` 0.3.8。

- `poc/gpui-textview/`: Windows ARM64 で作った。2026-10-10 に Linux（WSLg）で起動し、Mac でも動くようにした。同じ日に Windows x64 と Linux x64 でも確かめた（下の「Windows ARM64 での試作」「poc/gpui-textview を Mac で動かした結果」「poc/gpui-textview を Windows x64・Linux x64 で動かした結果」）。試作はこれにまとめる
- `poc/gpui/`: Mac で別に作ったもの（下の「Mac での試作」）。キーの割り当ても違う（Windows 版は F7 でフォントの切り替え、Ctrl+= / Ctrl+- で文字の大きさ。Mac 版は、サクラの割り当ての Ctrl を ⌘ に読み替えたもの）。Mac で要った設定や作りは `poc/gpui-textview` に移し、2026-10-10 にコードを消した（git の履歴に残っている）

## Windows ARM64 での試作（`poc/gpui-textview`）

### 試した環境と版

- Windows 11 ARM64（10.0.26200）、`aarch64-pc-windows-msvc`、Rust 1.99.0
- GPUI: crates.io の `gpui-pre` 0.3.8（Zed の 279fe07 のスナップショット。毎週更新される）
- gpui-component 0.7.1（`gpui-pre` =0.3.8 に固定されている）

### 作ったもの

- 自前で描くテキスト表示（`EditorElement`）。表示している行だけを整形して描く。行番号、カーソル、IME の変換中の下線
- 文字の入力と削除、改行、矢印キー・Home・End・PageUp・PageDown・Ctrl+Home・Ctrl+End、マウスでのカーソル移動、ホイールでのスクロール
- サクラの構成のメニューバー（ファイル / 編集 / 変換 / 検索 / ツール / 設定 / ウィンドウ / ヘルプ）。Alt キーだけで押すとメニューバーに入り、左右キーで選び、下キーか Enter で開く。Alt+F などで直接開く
- 縦のスクロールバー（つまみのドラッグ、つまみ以外のクリックでページ送り）
- ファイルを開く（ダイアログ、ドラッグ & ドロップ）。文字コードは UTF-8 だけ
- 100 万行のテキストを作る機能（F8）、フォントの切り替え（F7）と大きさの変更（Ctrl+= / Ctrl+-）
- 下の行に、カーソル位置・行数・フォント・起動から最初の描画までの時間を出す

### AI が確かめた結果

| # | 項目 | 結果 |
|---|---|---|
| 3 | 起動時間 | 最初の描画まで 181〜206 ms（Release ビルド）。基準の 0.3 秒以内 |
| 4 | スクロール | 100 万行を 89 ms で作り、末尾への移動やページ送りで正しく描けた。滑らかさの体感はユーザーが確かめる |
| 5 | メモリ | 下の表。GPU ドライバーの問題で多く見えていた。回避策を当てると、起動直後で約 54 MB |
| 8 | AI での開発のしやすさ | 下の「開発の記録」 |

#### メモリ（起動して 6 秒後、Windows のタスクマネージャーと同じ値）

| 状態 | GPUI のまま | 回避策を当てた GPUI |
|---|---|---|
| GPUI だけの最小のウィンドウ（`src/bin/minimal.rs`） | 約 84 MB | 約 48 MB |
| 同上に、システムの UI フォントで日本語を 1 行足したもの | 約 147 MB | 約 48 MB |
| 試作（起動直後） | 約 307 MB | 約 54 MB |
| 試作（100 万行を作り、スクロールとフォントの切り替えをした後） | 約 536 MB | 約 284 MB |

値はワーキングセット。回避策は下の「原因」の項を参照。100 万行の後の値のうち約 224 MB は、試作が 1 行ずつ `String` で持っている分（Rust のヒープ）で、本番のバッファ（ADR-0005）とは関係ない。

##### 原因（2026-10-09 に調べた）

GPU ドライバー（Qualcomm Adreno X1-85、31.0.160.0）の振る舞いだった。GPUI の作りやフォントの読み込みではない。

- 試作に、メモリの内訳を出す機能を足した（`POC_MEMSTAT`、`src/memstat.rs`）。試作（起動直後、約 307 MB）の Rust のヒープは 2.7 MB、Win32 のヒープは 7.8 MB しかない。残りのほとんどは、約 4 MB ずつ確保された領域が 61 個（約 244 MB）だった
- GPUI は、文字の形をテクスチャ（1024×1024、4 バイト／画素で 4 MB）にまとめて GPU に置く。最初は 1 枚しか作らず、文字を 1 つ足すたびに `UpdateSubresource` で書き込む。GPUI に一時的にログを足して調べると、この書き込み 1 回ごとにプロセスのメモリがちょうど 4.0 MB 増え、解放されなかった（使用中のテクスチャへの書き込みのたびに、ドライバーがテクスチャを丸ごと複製しているとみられる）
- 文字をグレースケールで描くと（テクスチャは 1 枚 1 MB）、増え方が小さくなる（最小のウィンドウ＋日本語で 76 MB、試作で 204 MB）。サブピクセル（ClearType）で描くときは 4 MB のテクスチャを使うので、増え方が大きい。以前「システムの UI フォントで日本語を描くと約 65 MB 増える。代わりのフォントの読み込みのため」と書いたのは誤りで、日本語の文字を書き込む回数が増えたためだった
- `UpdateSubresource1` の `D3D11_COPY_NO_OVERWRITE` も、書き込みのたびの `Flush` も効かなかった。小さな一時テクスチャ（`D3D11_USAGE_STAGING`）に書いてから `CopySubresourceRegion` で GPU 上でコピーすると、増えなくなった。この変更を `poc/gpui-textview/patches/gpui-pre-windows-atlas-upload.diff` に残した（GPUI の 1 つのファイルだけの変更）。画面の表示は変わらない
- Zed にも同じ機種での報告がある（[zed-industries/zed#56349](https://github.com/zed-industries/zed/issues/56349)、2026-05、GPUI 0.2.2）。その報告では書き込みをまとめて回数を減らして回避している。「再現の確認待ち」のまま放置の印が付いている。最新の Zed（main）の GPUI でも、書き込みは `UpdateSubresource` のまま
- 他の GPU（x64 の Windows の Intel・AMD・NVIDIA、Mac、Linux）で起きるかはまだ分からない。x64 の Windows で確かめる（2026-10-10 追記: Mac と、x64 の Windows の NVIDIA GeForce GTX 1650 では起きなかった。下の「poc/gpui-textview を Windows x64・Linux x64 で動かした結果」）

##### 評価

- 回避策を当てれば、起動直後で約 50 MB に収まる。サクラエディタ（数十 MB 程度）と同じくらい
- GPUI の本体に直してもらうまでは、本番では GPUI の Windows 部分（Apache-2.0）に回避策を当てて使う（`[patch.crates-io]` と、手を入れた写し）。GPUI の版を上げるたびに当て直す手間がかかる

#### 開発の記録

- API は、crates.io から取得したソースと examples（`input.rs` など）を読んで調べた。資料がなくて詰まることはなかった
- 最初に書いたコードは、Windows で 1 回目からビルドが通った（警告 1 件だけ）
- Windows では `cx.set_menus` はメニューを記録するだけで、画面には何も出ない（macOS だけがネイティブのメニューバーを出す）。Windows と Linux では、gpui-component の `AppMenuBar` で自分で描く必要がある
- gpui-component の `AppMenuBar` には、キーボードでメニューを開く手段がない（開く処理が非公開）。Windows の作法（Alt キーでメニューバーに入る、Alt+文字で開く）に合わせるため、`AppMenuBar` を写して手を加えた（`src/menu_bar.rs`。Apache-2.0 の表示を残している）。GPUI の Windows 版は Alt キーを押して離したことを `ModifiersChanged` で知らせ、Windows のシステムメニューには渡さないので、アプリ側で実装できる
- gpui-component の `PopupMenu` は、上下キー・Enter・Esc に対応している。ただし、キーボードで開いたときに先頭の項目が選ばれた状態にならず、メニュー項目の中の文字（「開く(O)」の O など）でも選べない。本番では自前で用意する必要がある
- スクロールバーは、テキスト表示を自前で描いているので自分で描いた。スクロール位置を `f32` のピクセル値で持つと、行数が 1,000 万行を超えるあたりで 1 ピクセル単位の精度がなくなる。本番では「先頭の行番号＋行内のずれ」で持つ
- examples が使う `gpui_platform` は、crates.io では `gpui-pre-platform` という名前になっている
- `gpui-pre-platform` は、既定では Wayland と X11 の機能が無効になっている。そのままでは Linux で画面を持たない動作（headless）になり、エラーも出ずにウィンドウが出ない。`features = ["wayland", "x11"]` を指定する

### Linux（WSLg）での起動（2026-10-10）

同じ PC の WSL2（Ubuntu 24.04 aarch64、WSLg 1.0.66）で、`poc/gpui-textview` をビルドして起動した。

- ビルドに必要なパッケージ: `clang cmake libasound2-dev libfontconfig-dev libfreetype-dev libwayland-dev libx11-xcb-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libzstd-dev`。日本語を出すには `fonts-noto-cjk` も要る
- Wayland で起動し、メニュー・行番号・日本語・スクロールバー・下の行が Windows と同じように出た
- WSLg では GPU を使えず、ソフトウェア処理（Vulkan の llvmpipe）で描いている。Mesa の d3d12 ドライバーは、glxinfo が応答しなくなって使えなかった。描画の速さは、実機の Linux で測る
- BIZ UDGothic は入っていないので、日本語は代わりのフォント（Noto Sans CJK）で出る。Noto Sans CJK は地域ごとに字形が違う（日本・中国・香港など）。本番では日本向けのフォントを明示して選ぶ
- 日本語入力（fcitx5 などの IME）はまだ確かめていない（2026-10-10 追記: x64 の WSLg で確かめた。下の「Linux（WSLg）での日本語入力」）

### ユーザーの確認（2026-10-09）

1〜4 を試してもらった。日本語入力と文字の見え方は問題なし。気になった点として次の 2 つが挙がり、どちらも上のとおり対応した。

- Alt キーを使ったメニュー操作ができない
- スクロールしたときに、右にスクロールバーが出ない

### ユーザーに確かめてもらうこと

- [x] 1. 日本語入力（Microsoft IME、Google 日本語入力）: 変換中の文字が入力位置に出るか、候補ウィンドウが入力位置の近くに出るか、確定や取り消しで文字が重複・欠落しないか
- [x] 2. 文字の見え方（BIZ UDGothic / MS Gothic / Consolas / Yu Gothic UI を F7 で切り替え、12〜14pt）
- [x] 4. 100 万行のスクロールの滑らかさ（ホイール、PageUp・PageDown の押し続け）
- [x] 6. メニューとショートカット: メニューバーの操作感、Alt キーでメニューに入れるか、ファイルのドラッグ & ドロップ（Alt キーでの操作は指摘を受けて足した。直した版の操作感は、使いながら確かめる）
- [ ] 7. Windows x64 の実機でビルドと起動、Mac と Linux でのビルドと起動（Linux は WSLg で起動できた。上を参照）。あわせて、`POC_MEMSTAT=6` を付けて起動し、メモリが 4 MB ずつ増える問題が起きるかを見る（Mac の分は 2026-10-10 に確かめた。下の「poc/gpui-textview を Mac で動かした結果」。Windows x64 と Linux x64 の分も、ビルドと自己確認モードまでは 2026-10-10 に AI が確かめた。下の「poc/gpui-textview を Windows x64・Linux x64 で動かした結果」。残りは、人が動かしての確認）

## poc/gpui-textview を Mac で動かした結果（2026-10-10）

Windows 版と同じコードを、Mac（Apple M1、macOS 26.1、JIS キーボード）でビルドして動かした（確認項目 7 の Mac の分）。Mac 版（`poc/gpui`）で分かったことを移した。

### 変えたこと

- `Cargo.toml`: Mac のときだけ、GPUI の `font-kit`（ないと文字が 1 つも描かれない）と `runtime_shaders`（Metal Toolchain なしでビルドできる）を有効にした。どちらも、ほかの依存からは有効になっていなかった
- メニュー: 画面上部のメニューバーに出し、先頭にアプリのメニュー（隠す、終了など）を置いた。「ファイル(F)」の (F) は Mac では付けない。ウィンドウメニューに「しまう」と「拡大／縮小」を入れた。ウィンドウの中のメニューバーと Alt キーの処理は、Mac では使わない（Option で文字を入力するため）
- キー: Ctrl は ⌘ になる（GPUI の `secondary`）。⌘+ と、Mac でも Windows と同じ ⌃= / ⌃+ / ⌃- を足した。Mac の作法の ⌘H・⌥⌘H・⌘M・⌘←→↑↓ を足した
- フォント: Mac にあるもの（BIZ UDゴシック、Osaka、Menlo、ヒラギノ角ゴシック）にした
- 自己確認モード（`--features self-check`）を足した。キーを macOS のキーイベントとして押し、表示を PNG に保存する（[README](../../poc/gpui-textview/README.md)）

Windows の動きは変えていない（Mac のときだけの分岐にした）。変えたあとのコードは、2026-10-10 に Windows x64 でビルドして確かめた（下の「poc/gpui-textview を Windows x64・Linux x64 で動かした結果」）。Windows ARM64 ではまだビルドしていない（2026-10-11 追記: 印刷の試作で、Windows ARM64 でもビルドして動かした。[印刷の試作の結果](printing.md)）。

### 結果

| # | 項目 | 結果 |
|---|---|---|
| 1, 2 | 日本語入力、文字の見え方 | ユーザーが確かめる。IME と同じ呼び出しでの変換中の下線と確定、4 つのフォントでの表示は、自己確認モードの画像で確かめた |
| 3 | 起動時間 | Release ビルドで、最初の描画まで 203〜233 ms（6 回）。基準の 0.3 秒以内。シェーダーは起動時にコンパイルしている |
| 4 | スクロール | 100 万行を作り、⌘↓ で移った末尾を正しく描けた。滑らかさの体感はユーザーが確かめる |
| 5 | メモリ | physical footprint（アクティビティモニタの「メモリ」）で、起動直後 36 MB、4 つのフォントを一巡した後 40 MB、100 万行を作って末尾に移った後 282 MB（うち 224 MB は 100 万行を持つ Rust のヒープ）。Windows で見つかった、文字を書き込むたびに 4 MB 増える問題は起きていない。自己確認モードを付けた Release ビルドで測った。同じ日の夕方に測り直すと、58 MB・72 MB・313 MB だった。Linux 対応を入れる前のコードでも同じだったので、コードではなく環境の違い（このときのメインのディスプレイは 5K の外部ディスプレイ）とみられる |
| 6 | メニューとショートカット | F7・F8・⌘= / ⌘+ / ⌘-・⌃= / ⌃+ / ⌃-・⌘↓ を、JIS キーボードと同じ形の macOS のキーイベントで確かめた。メニューの操作感はユーザーが確かめる |

## poc/gpui-textview を Windows x64・Linux x64 で動かした結果（2026-10-10）

Mac 対応を入れたあとのコードを、x64 の PC 1 台で、Windows とその上の WSL2 でビルドして動かした（確認項目 7 の x64 の分）。

### 環境

| | Windows x64 | Linux x64 |
|---|---|---|
| OS | Windows 11 Pro Insider Preview（10.0.26220） | WSL2 の Ubuntu 24.04.5。WSL 3.0.1.0、WSLg 1.0.79（下の「WSLg でウィンドウが描かれなかった件」で 2.6.1.0・1.0.66 から更新した） |
| CPU | AMD Ryzen 7 3700X | 同じ |
| GPU | NVIDIA GeForce GTX 1650（ドライバー 32.0.16.1088）。GPUI は Direct3D 11.1 で使う | WSLg では GPU を使えず、Vulkan のソフトウェア処理（llvmpipe） |
| Rust | 1.99.0（rustup）、Visual Studio 2019 の C++ ツール | 1.99.0（asdf） |

### 変えたこと

- 自己確認モード: Linux ではウィンドウを画像にする処理（`render_to_image`）が GPUI 0.3.8 にないので、画像の保存だけ飛ばし、ほかの確認は続けるようにした
- Linux のフォント: F7 で切り替える 4 つは Windows のフォントで、Linux にはないので、4 つとも同じ代わりのフォントで描かれていた（切り替わっていないように見えた）。Linux では、fonts-noto-cjk と Ubuntu にあるもの（Noto Sans Mono CJK JP、Noto Sans CJK JP、DejaVu Sans Mono、Noto Serif CJK JP）にした。日本向けのフォントを名前で選ぶので、字形も日本のものになる
- Linux のタイトルバー: WSLg の Wayland（Weston）は、タイトルバーをサーバー側で描かない。GPUI は試作に描かせる（client-side decorations）が、試作は描いていなかったので、タイトルもボタンも出ず、ウィンドウを動かせなかった。この場合だけ、gpui-component の `TitleBar` でタイトル、最小化・最大化・閉じる、ドラッグでの移動を描くようにした。ボタンのアイコンのため、Linux だけ `gpui-kit-assets`（Apache-2.0、アイコンは Lucide の ISC）を足した。ウィンドウの端での大きさの変更は作っていない
- ファイルを開く: Linux のダイアログは xdg-desktop-portal を通して出す。無いときに何も起きなかったので、理由を下の行に出すようにした

Windows の動きは変えていない（フォントの一覧を Windows と Linux で分けただけ）。

### 結果

どちらでも、`cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`（`--features self-check` あり・なし）、`cargo test --locked`（テストは 0 件）、Release ビルドが通った。自己確認モードも最後まで通った（`self-check: ok`）。

| # | 項目 | Windows x64 | Linux x64（WSLg） |
|---|---|---|---|
| 1, 2 | 日本語入力、文字の見え方 | IME と同じ呼び出しでの変換中の下線と確定、4 つのフォント、メニューバーを、自己確認モードの画像で確かめた。本物の IME はユーザーが確かめる | 画像を保存できないので、ユーザーが画面で確かめた（下の「ユーザーの確認」）。日本語入力は fcitx5 と Mozc で確かめた（下の「Linux（WSLg）での日本語入力」） |
| 3 | 起動時間 | Release ビルドで、最初の描画まで 440〜542 ms（6 回）。基準の 0.3 秒を超えた。同じ PC で、消す前の `poc/gpui` も 439〜477 ms だったので、この PC の環境によるとみられる。中身はまだ調べていない | 158〜189 ms（4 回）。WSLg を更新する前に測った値 |
| 4 | スクロール | 100 万行を作って正しく描けた。滑らかさの体感はユーザーが確かめる | 100 万行を作れた。滑らかさは、GPU のある実機の Linux で確かめる |
| 5 | メモリ | 下の項 | RSS で約 173〜180 MB（起動して 6 秒後、WSLg を更新する前）。ソフトウェア描画の分を含むので、実機の値ではない |
| 6 | メニューとショートカット | F7・F8・Ctrl+= / Ctrl++ / Ctrl+- を確かめた。メニューの操作感はユーザーが確かめる | 同じ |

#### Windows x64 のメモリ

ワーキングセット（タスクマネージャーと同じ値）。

| 状態 | 値 |
|---|---|
| GPUI だけの最小のウィンドウ（`src/bin/minimal.rs`、起動して 6 秒後） | 約 58 MB |
| 試作（起動して 6 秒後） | 約 62 MB |
| 試作（自己確認モードの Release ビルドで、4 つのフォントを一巡した後） | 約 72 MB |
| 同上（100 万行を作った後） | 約 309 MB（うち約 224 MB は 100 万行を持つ Rust のヒープ） |

Qualcomm Adreno で見つかった、文字を書き込むたびに 4 MB 増える問題は、NVIDIA GeForce GTX 1650 では起きなかった。`POC_MEMSTAT=6` の内訳でも、約 4 MB の領域が並ぶことはなく、プロセス専用の領域のワーキングセットは約 13 MB だった。値は、Adreno で回避策を当てたとき（起動直後 約 54 MB、100 万行の後 約 284 MB）と同じくらい。Intel・AMD の GPU ではまだ確かめていない。

#### Windows の Alt キー

GPUI は、Windows でウィンドウを前面に出すとき（`WindowsWindow::activate`）に、Alt キーを押して離す入力を自分で送る。消した `poc/gpui` では、新しいウィンドウを開くたびにこれを受けてメニューバーが選ばれた状態になったので、前面に出してから 300 ms のあいだの Alt を無視していた。`poc/gpui-textview` は、起動時にウィンドウを 1 つ開くだけで、前面に出す処理を呼ばないので起きない。起動直後の画像でも、メニューバーは選ばれていなかった。ウィンドウを増やすときには、同じ対処が要る。

### WSLg でウィンドウが描かれなかった件

最初は、Linux でウィンドウがタスクバーに出るだけで、画面には何も描かれなかった（Wayland でも X11 でも同じ）。試作のログでは最初の描画まで進んでいて、自己確認モードも通っていた。自己確認モードはキーの処理と中の状態しか見ないので、画面に出ないことには気づけなかった。

- WSLg の Weston のログ（`/mnt/wslg/weston.log`）に、`rdp_allocate_shared_memory: Failed to open "/mnt/shared_memory/{...}" with error: Input/output error` が出続けていた。ウィンドウの中身を Windows に送るための共有メモリが開けず、中身が届いていなかった。試作や GPUI の問題ではない
- `wsl --shutdown` で WSL を止め、`wsl --update` で WSL 2.6.1.0・WSLg 1.0.66 から WSL 3.0.1.0・WSLg 1.0.79 に更新すると、描かれるようになった

### ユーザーの確認（2026-10-10）

- Windows x64: 試した操作はすべて問題なかった
- Linux x64（WSLg 1.0.79）: 上の「変えたこと」を入れた版で、タイトルバー（移動とボタン）、F7 でのフォントの切り替え、Ctrl+C（行のコピー）と Ctrl+V での Windows との受け渡しを試し、ほぼ問題なかった。Ctrl+= / Ctrl+-、F8、Alt でのメニューは、変える前の版で動いた。Ctrl+O は、`sudo apt install xdg-desktop-portal xdg-desktop-portal-gtk` でダイアログが出るようになった
- 文字列の選択と、選んだ部分のコピーはできない。試作にまだ作っていない（Windows・Mac も同じ）。PoC のうちに作ることにした（下の「選択とクリップボード」）

### 選択とクリップボード（2026-10-10）

ユーザーの指摘を受けて、選択とコピー・切り取り・貼り付けを足した。キーはサクラの既定の割り当て（`sakura_core/func/CKeyBind.cpp`）に合わせた（[README](../../poc/gpui-textview/README.md)）。

- Shift+移動キー、マウスのドラッグ、Shift+クリックで選ぶ。Ctrl+A ですべて選択。入力・Backspace・Delete・貼り付けは、選んだ部分を置き換える
- 選んでいないときのコピーと切り取りは、カーソルの行を改行ごと扱う。それを貼り付けると、カーソルの位置によらず行の上に入る（サクラの既定の「選択なしでコピー」と「ラインモード貼り付け」、`sakura_core/cmd/CViewCommander_Clipboard.cpp`）。ほかのアプリからの貼り付けは、ふつうの貼り付けになる
- 編集メニューに、切り取り(T)・コピー(C)・貼り付け(P)・すべて選択(A) を置いた（サクラの `MainMenu.ini` と同じアクセスキー）
- 自己確認モードで、Windows x64 と Linux x64（X11）で確かめた。Linux の Wayland では、GPUI がクリップボードを持つのに本物のキーかマウスの入力（の serial）を要るので、自己確認モードからは確かめられない。人が操作するときは問題ない
- まだないもの: 元に戻す、単語の選択（ダブルクリック）、矩形選択。タブを含む行では、表示用にタブを広げているので、選択の位置がずれる（カーソルと同じ、もとからの制限）

### Linux（WSLg）での日本語入力（2026-10-10）

WSLg の Linux のアプリには、Windows の IME（Microsoft IME など）は届かない。Linux 側に IME を入れて、X11 で起動した試作と XIM でつないだ。準備の手順は [README](../../poc/gpui-textview/README.md) の「WSL2（WSLg）で動かす」にまとめた。

- IME: fcitx5 5.1.7 と Mozc 2.28（Ubuntu 24.04 のパッケージ）。「半角/全角」キーで日本語入力に切り替わった
- そのまま `fcitx5 -d` で起動すると、すぐに終了していた。WSLg が IME 用の Wayland の接続（`zwp_input_method_v1`）を断り、fcitx5 がそれをエラーとして終了するため。`--disable=wayland,waylandim` を付け、`WAYLAND_DISPLAY` を空にして起動すると動いた
- 試作は `WAYLAND_DISPLAY=` で X11 を使わせ、`XMODIFIERS=@im=fcitx` を付けて起動する。GPUI は X11 では XIM で IME とつなぐ（ログに `Try connect fcitx`、`XConnected server`）。Wayland では GPUI は text-input-v3 を使うが、WSLg は fcitx5 が Wayland に入力方式としてつながることを許さない（上の `zwp_input_method_v1` のエラー）ので、Wayland のままでは使えない
- ユーザーの確認: 変換中の文字は入力位置に出た。確定で文字の重複や欠けはなかった。候補の窓は、起動して最初の変換だけウィンドウの左下に出て、2 回目からは入力位置の左下に出た（最初の 1 回だけ位置が IME に伝わっていないとみられる。原因はまだ調べていない。Wayland では起きなかった）
- 実機の Linux（GNOME や KDE の Wayland）では、text-input-v3 で Wayland のままつながるはず。まだ確かめていない（2026-10-14 追記: GNOME の Wayland で確かめた。下の「Hyper-V の Ubuntu（Wayland）での確認」）

### ユーザーに確かめてもらうこと（x64）

- [x] Windows x64: 本物の IME での入力、文字の見え方、Alt キーでのメニュー操作、100 万行のスクロールの滑らかさ
- [x] Linux x64: 表示（メニュー・日本語・行番号・スクロールバー）、ファイルを開くダイアログ、日本語入力（fcitx5 と Mozc）

## Hyper-V の Ubuntu（Wayland）での確認（2026-10-13〜14）

ADR-0004 の「残っている確認」のうち、Wayland での日本語入力とデスクトップとの統合を確かめた。WSLg では、IME を Wayland の入力方式としてつなげず、タイトルバーもサーバー側で描かないので、試せなかった。開発用の PC の Hyper-V で Ubuntu のデスクトップを動かし、main の `poc/gpui-textview` を WSL2（Ubuntu 24.04 aarch64）で Release ビルドしたもの（機能なし）を写して動かした。

### 環境

- ホスト: Windows 11 Pro（10.0.26200）、Snapdragon X Elite（X1E80100）、メモリ 64 GB
- 仮想マシン: Hyper-V の第 2 世代、仮想 CPU 4、メモリ 8 GB、セキュアブート（Microsoft UEFI CA のテンプレート）。vmconnect の基本のセッションで試した（拡張セッションは X11（xrdp）の画面になる）
- ゲスト: Ubuntu 26.04.1 LTS（arm64）、カーネル 7.0.0-38、GNOME 50.1 の Wayland。画面は hyperv_drm で、1024×768 と 1920×1080 で試した
- 日本語入力: IBus と Mozc 2.29（ibus-mozc）。入力ソースは「日本語 (Mozc)」と「日本語」
- 描画: GPU は使えない。GPUI は Vulkan のソフトウェア処理（llvmpipe、Mesa 26.0.8）を選んだ

### 結果

ユーザーが確かめて、どれも問題なかった。

- 日本語入力: 変換中の文字が入力位置に下線付きで出た。候補の窓は入力位置の近くに出た。確定、長い文の文節の移動と長さの変更、Esc での取り消し、変換中の BackSpace も正しく動いた
- タイトルバー: GNOME はタイトルバーをサーバー側で描かない（xdg-decoration がない）ので、GPUI は試作に描かせる（ログ: `Server-side decorations requested, but the Wayland server does not support them. Falling back to client-side decorations.`）。WSLg のときに足した gpui-component の `TitleBar` で、ドラッグでの移動、ダブルクリックでの最大化、最小化・最大化・閉じるのボタンが動いた
- ウィンドウの端での大きさの変更: できた（WSLg ではできなかった）。試作は作っていないが、gpui-component の `Root` が付ける枠（`window_border`）が端に当たり判定を置き、GPUI の `start_window_resize` を呼ぶ
- 拡大率: 1920×1080 で 200% にして、文字はぼやけず、配置も崩れず、候補の窓の位置も合っていた
- ファイルを開くダイアログ（Ctrl+O）と印刷ダイアログ（Ctrl+P）が、xdg-desktop-portal で出た。ほかのアプリとのコピーと貼り付けもできた
- 最初の描画まで 133 ms（1 回。ソフトウェア描画）

### Wayland での IME とのやり取り

`WAYLAND_DEBUG=1` で起動して、text-input-v3 のやり取りを記録した。

- GPUI は、フォーカスを得ると `enable` に続けて `set_cursor_rectangle` でカーソルの位置を送る（`gpui-pre-linux` の `enable_ime`）。最初の変換の前に位置が伝わるので、WSLg の X11（XIM）で見えた「起動して最初の変換だけ候補の窓がウィンドウの左下に出る」件は起きなかった
- 変換中は、`preedit_string` が届くたびに GPUI が入力位置を求め、変わっていれば送り直す。確定（`commit_string`）の後は送り直さない
- 試作は `Window::invalidate_character_coordinates()` を呼んでいない。本番のエディタでは、カーソルが動いたときに呼んで、IME に新しい位置を伝える

### 仮想マシンで起きたことと対処

どれも Hyper-V とゲストの環境の問題で、試作や GPUI の問題ではない。

- ゲストの時計の遅れ: ホストがバッテリーで DRIPS（モダン スタンバイの一番深い状態）に入っている間、仮想マシンが止まり、復帰しても時計が戻らなかった。Windows のイベント（Kernel-Power 507）の `DripsResidencyInUs` の合計 1 時間 57 分と、遅れの約 1 時間 57 分が一致した。AC 電源のときは DRIPS に入らず、遅れなかった。時計が遅れていると、Ubuntu のインストーラーの apt が `Release file ... is not valid yet` で失敗する。Linux の hv_utils のモジュール引数 `timesync_implicit=1`（ゲストがホストより 5 秒以上遅れたら、ホストから届く時刻で合わせ直す。`drivers/hv/hv_util.c` の `hv_implicit_sync`）を `/etc/modprobe.d/` に置いた。時計を 120 秒戻す試験では、15 秒以内に戻った
- 半角/全角キー: vmconnect は、JIS キーボードの半角/全角キーを押すと「離す」と「押す」を続けて送り、指を離したときは何も送らない。ゲストではキーが押されたままになり、GNOME のキーの繰り返しで Mozc の入力モードが切り替わり続けた。このキーだけ繰り返さない XKB のオプションを `~/.config/xkb` に作り、GNOME の `xkb-options` に入れて直した。gnome-shell 50.1 は、`xkb-options` を変えてもその場では反映しない（`keyboardManager.js` の `reapply()` が何もしない）ので、入力ソースを切り替えて反映させた
- vmconnect の「クリップボードのテキストを入力」は、Linux のゲストでは使えなかった（空白が d に、`$` が j になるなど、文字の番号がキーの番号として届く）。コマンドは SSH で送った（Windows の `ssh.exe` を使う。WSL からは Hyper-V の Default Switch に届かない）

## Mac での試作（`poc/gpui`）

Mac で別に作った試作。Mac で要った設定や作りは `poc/gpui-textview` に移したので、試作は `poc/gpui-textview` にまとめ、このコードは 2026-10-10 に消した。コードと、動かし方と確認の手順（`poc/gpui/README.md`）は、git の履歴に残っている（`git log -- poc/gpui` で探せる）。

### 環境

| 項目 | 内容 |
|---|---|
| 機械 | Apple M1 の Mac、macOS 26.1、JIS キーボード |
| Xcode | 26.2（Metal Toolchain は入れていない） |
| Rust | 1.99.0 |
| GPUI | gpui-pre 0.3.8（Zed の zed@279fe07 のスナップショット） |

### 確認項目

| # | 項目 | 結果 |
|---|---|---|
| 1 | 日本語入力 | 未確認（ユーザーが試す）。IME と同じ呼び出しを流す自己確認では、変換中の文字列の表示、確定、Undo 1 回で変換前に戻ること、候補ウィンドウの位置の計算が動いた |
| 2 | 文字の見え方 | 未確認（ユーザーが判断する）。下の「文字の見え方について」を参照 |
| 3 | 起動時間 | Release ビルドで、起動から最初の描画まで中央値 169 ms（12 回、160〜183 ms）。ビルド直後の初回は 233〜487 ms。目安の 0.3 秒に収まる |
| 4 | スクロール | 100 万行（7,600 万字、UTF-8 で 108 MB）の生成が 80 ms。50 万行目付近の表示を確認した。引っかかりの有無はユーザーが確かめる |
| 5 | メモリ | 6 行のファイルを開いた状態で、physical footprint（アクティビティモニタの「メモリ」）58〜59 MB、最大 64 MB。RSS は約 104 MB。実行ファイルは 10 MB。Windows で見つかった、文字を書き込むたびに 4 MB 増える問題が Mac でも起きるかは確かめていない |
| 6 | メニューとショートカット | 作った。macOS の作法に合うかはユーザーが確かめる。.app にまとめ、`open -a` でファイルを渡すと（Finder の「このアプリケーションで開く」と同じ Apple イベント）、最初のウィンドウに開くことは確認した。ユーザーが試して効かなかったキーを直した（下の「ユーザーの確認」） |
| 7 | ほかの環境 | 未確認。このコードは Mac でしか動かしていない |
| 8 | AI での開発のしやすさ | 下の「手戻りと分かったこと」を参照 |

#### 起動時間の内訳

Release ビルド、シェーダーは起動時にコンパイルする設定で測った。

| 区間 | 時間 |
|---|---|
| プラットフォームの初期化まで | 約 50 ms |
| フォントの一覧の取得（`all_font_names`） | 約 75 ms。メニューで未インストールのフォントを灰色にするためだけに使うので、バックグラウンドに移した |
| メニューの設定 | 約 15 ms |
| ウィンドウの作成と最初の描画 | 約 90 ms（シェーダーのコンパイルを含む） |

シェーダーをビルド時にコンパイルした場合（`--no-default-features`、Metal Toolchain が要る）は測っていない。

#### 文字の見え方について

- サクラと同じく、全角文字を半角 2 文字分の幅に揃えて描く方式を作った（設定で切り替えられる）。GPUI の `shape_line` にある `force_width` は全角を考慮せず全部を 1 桁にするので、字形の位置を自前で付け直した
- Menlo ＋ ヒラギノ角ゴシック: Menlo の半角は 0.6em、ヒラギノの全角は 1em なので、揃えると全角文字の間が少し空く
- BIZ UDゴシック: 半角と全角がちょうど 1:2 なので、揃えても揃えなくても同じ見え方になる
- Osaka−等幅は選べない。GPUI はファミリー名と太さ・スタイルでフォントを選ぶので、Osaka ファミリーの中では等幅でない Osaka が選ばれる

### 手戻りと分かったこと

試作中に API の確認や作り直しが要ったこと。

- 公式の `gpui` は crates.io では 0.2.2（2025-10）で止まっている。gpui-component 0.7 は、gpui-component の作者が毎週公開している Zed のスナップショット（`gpui-pre`）を使っているので、試作もそれに合わせた
- GPUI はプラットフォームごとのクレート（`gpui_macos` など）に分かれた。アプリは `gpui_platform::application()` から起動する
- macOS では `font-kit` 機能を有効にしないと、文字が 1 つも描かれない（何もしないテキストシステムになる）。エラーは出ない
- Metal Toolchain がないとビルドできない。`runtime_shaders` 機能で、シェーダーを起動時にコンパイルするようにした
- 上の 2 つ（`font-kit` と `runtime_shaders`）は、`poc/gpui-textview` を Mac で動かすときにも有効にした（上の「poc/gpui-textview を Mac で動かした結果」）
- コンパイルエラーでの手戻りは 2 回だけだった（関数の戻り値の型、`Context::spawn` の引数の数）。API は、依存先の examples（`input.rs` など）とソースを読めば分かった
- ウィンドウの描画結果を画像にする `Window::render_to_image` は、`test-support` 機能を有効にしたときだけ使える。これで、macOS の画面収録の許可なしに表示を確かめる自己確認モードを作った
- サクラの Ctrl+W（単語選択）、Ctrl+←→（単語単位の移動）は、Cmd に読み替えると macOS の閉じる・行頭・行末と重なる（[ADR-0008](../adr/0008-commands-and-keybindings.md) の読み替えの案で決める必要がある）。試作では Mac の意味にした
- GPUI は、⌘ や ⌃ と一緒に押したキーを、そのときのキーボード配列とキーボードの種類から読み直す（`UCKeyTranslate`）。JIS キーボードと ABC 配列では、⌘⇧- が `cmd-=`、⌘⇧; が `cmd-+` と、キーの刻印どおりに読まれる。GPUI にはヨーロッパの配列向けのショートカットの読み替え表もある。JIS は入っていないが、刻印どおりに読まれるので要らなかった
- 自己確認でキーを流すとき、`NSEvent` の `keyEventWithType` で作ったイベントにはキーボードの種類がなく、US 配列として読まれた（⌘⇧- が `cmd-_` になった）。キーボードの種類を付けた `CGEvent` から作ると、本物のキーと同じに読まれる。アクセシビリティの許可は要らない（自分のイベントキューに入れるだけなので）。なお、GPUI が使うアプリのキーボードの種類（`LMGetKbdType`）は、起動直後は実際のキーボードと違うことがあった（JIS のキーボードなのに 198 = US）。キーを受け取ると、そのキーのキーボードの種類（42 = JIS）に変わる

### ユーザーの確認（2026-10-09）

Mac（JIS キーボード）で試してもらった結果と、その対応。Windows 版のキー（F7、Ctrl+= / Ctrl+-）は、Mac 版では割り当てが違うので効かなかった。

| 試したこと | 結果と原因 | 対応 |
|---|---|---|
| `cargo run --release -- メモ.txt` | 開けなかった。ないファイルを指定していて、エラーにしていた | サクラと同じく、その名前の空の文書として開き、保存したときにファイルを作るようにした（`CLoadAgent::OnLoad`） |
| ⌃= / ⌃- で文字を大きく・小さく | 効かなかった。Windows 版は Ctrl+= / Ctrl+- だが、Mac 版では ⌘= / ⌘- にだけ割り当てていた（⌘ なら効くことは、JIS キーボードのキーイベントで確かめた） | Mac でも ⌃ で押せるようにした。⌘+ / ⌃+ も足した。設定 > 文字の大きさ に「フォントサイズ拡大」「フォントサイズ縮小」を置き、キーがメニューに出るようにした |
| F7 でフォントの切り替え | 効かなかった。Windows 版は F7 でフォントを切り替えるが、Mac 版はサクラの割り当てに合わせたので、フォントの切り替えにはキーを割り当てていなかった（メニューだけ）。⌘F7 は、サクラと同じく英字を大文字にする変換。また、Mac のキーボードの F1〜F12 は、fn なしでは明るさや音量、再生などのキーになり、アプリに届かない | 大文字・小文字への変換を、サクラにある ⌥⌘L / ⇧⌥⌘L（Ctrl+Alt+L / Shift+Ctrl+Alt+L）でも押せるようにした。README に fn のことを書いた。2 つの試作のキーの割り当てを揃えるかは、まとめ方と一緒に決める |

サクラの既定の割り当ては F キーを多く使う（F3 で次を検索、F6 で選択開始、F7 で切り取りなど）。Mac のキーボードでは fn が要るので、Mac での読み替え（ADR-0008）で扱いを決める必要がある。

### 依存のライセンス

Mac 版の試作の依存（GPUI が使うものを含む、全プラットフォームで 707 個）に、GPL 系だけのライセンスのものはない。GPL・LGPL を含むもの（`self_cell`、`r-efi`）は、MIT か Apache-2.0 との選択式。

MPL-2.0（ファイル単位のコピーレフト）が 3 つある。本番で GPUI を使うなら、配布の前に扱いを決める。

| クレート | 入り方 |
|---|---|
| `cbindgen` | GPUI の macOS 版のビルド時だけ使う（実行ファイルには入らない） |
| `option-ext` | GPUI が使う font-kit（Zed の fork）→ `dirs` → `dirs-sys` |
| `dwrote` | 同じ font-kit の Windows 版 |

Windows 版の試作の依存（gpui-component を含む）は調べていない。
