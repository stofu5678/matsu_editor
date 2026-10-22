# E2E のスクリーンショットと UI Automation

実物の `ume` を 5 つの対象（Windows x64・ARM64、macOS ARM64、Linux x64・ARM64）で起動して、画面の写真を撮る。写真は人間や AI が見るためのもので、保存した画像との画素の比較はしない（文字の描き方が実行ごと・システムごとに違うため）。Windows では UME 自身のウィンドウを UI Automation（UIA）で、macOS では Accessibility API（System Events）で、外から操作してユーザーと同じ手順で動かす。

設定画面が見切れる（#160）、フォントが効かない（#171）といった問題を、デバイスを用意せずに CI の画像で見つけるためのしくみ。

## 構成

| 場所 | 役割 |
|------|------|
| `.github/workflows/e2e.yml` の `screens` ジョブ | 5 つの対象で写真を撮り、アーティファクトにする。ほかの CI と並行して動く |
| `crates/ume/tests/e2e_screens.rs` | 場面ごとに `ume` を一時プロファイルで起動し、起動時のキーマクロ（`-M=`）でウィンドウを開き、描かれたら画面を PNG にする |
| `crates/ume/tests/windows_uia.rs` | Windows だけ。UIA で UME のウィンドウを操作する |
| `crates/ume/tests/macos_ax.rs` | macOS だけ。`osascript`（System Events）で UME のメニューとウィンドウを操作する |
| `.github/workflows/mac-check.yml` | 手動で動かす。リポジトリのスクリプトを macOS のランナーで 1 つ動かし、写真とログを残す |
| `crates/ume/tests/common/` | 起動・画面の保存・UIA の共通部分 |
| `scripts/e2e-screens.sh` | 写真を手元に取ってくる |

## 写真の撮り方

画面全体を撮る（開いているメニューやほかのウィンドウも写る）。

- Windows: PowerShell と System.Drawing（`crates/ume/tests/common/capture.ps1`）。Windows のランナーには操作できるデスクトップがある。描画は WARP
- Linux: xvfb（仮想ディスプレイ）の画面を ImageMagick の `import -window root` で撮る。ワークフローが `imagemagick` を apt で入れる
- macOS: `screencapture`。GitHub の macOS ランナー（macos-15）は、画面収録とアクセシビリティの許可を最初から持っている（下の「macOS ランナーの許可」）。ウィンドウが写っていなければ、ほかの対象と同じく失敗にする

Windows ARM64 のランナー（`windows-11-arm`）は、デスクトップの全面に Windows の初回設定の画面（"Choose privacy settings"）を出していて、写真はその画面になっていた（2026-10-21 に確認）。`scripts/windows-clear-privacy-screen.ps1` が、方針 `DisablePrivacyExperience` を書き、その画面のプロセスを止め、そのあとに出てくるスタートメニューと `wsl.exe` のコンソール（"Windows Subsystem for Linux must be updated"）を閉じてから写真を撮る。そのスクリプトが前後のデスクトップの写真（`windows-arm64-desktop-before.png` と `-after.png`）も残すので、効いているかは成果物で分かる。Linux のランナーには日本語フォントが無いので、ワークフローが `fonts-noto-cjk` を入れる。

手動で全 5 対象を動かすには `gh workflow run e2e.yml --ref <ブランチ>`。

（環境変数 `UME_E2E_SOFT_SCREENS=1` は、画面を信用できない環境のために残してある。ワークフローは今は設定しない。）

検査は 2 つだけ。画面が 640x480 以上であること、起動前の画面と違うこと。

## 場面

ファイル名は `<対象>-<場面>.png`（対象は `windows-x64`、`linux-arm64` など）。

| 場面 | 内容 | 撮り方 |
|------|------|--------|
| `main-window` | 日本語の文書を開いた本体 | 全対象 |
| `open-menu` | 開いているメニュー（右ボタンのメニュー） | 全対象 |
| `common-settings` | 共通設定の最初のページ（全般） | 全対象 |
| `type-settings` | タイプ別設定の最初のページ | 全対象 |
| `print-preview` | 印刷プレビュー | 全対象 |
| `print-settings` | 印刷のページ設定 | 全対象 |
| `find-dialog` | 検索ダイアログ | 全対象 |
| `uia-main-window` `uia-menu-設定` `uia-typed` `uia-close-question` | UIA の各段階の写真 | Windows |
| `ax-main-window` `ax-common-settings` `ax-about` `ax-typed` | Accessibility API の各段階の写真 | macOS |
| `uia-common-settings-page-NN` | 共通設定の全ページ（NN は左のリストの順） | Windows |
| `uia-type-settings-page-NN` | タイプ別設定の全ページ | Windows |

キーマクロで開ける場面は全対象で撮る。設定ダイアログの 2 ページ目以降は、クリックが要るので Windows の UIA でだけ撮る（キーマクロの名前に、ページを指定するものが無いため）。

## UIA で操作すること（Windows）

`windows_uia.rs` は、アクセシビリティの名前（#169）で要素を探して、ユーザーと同じ順で操作する。

1. 本体が上がるのを、テキストビューの名前（`テキストエディタ`）で待つ
2. メニューバーの「設定」を押し、アクセスキー（C）で「共通設定...」を選ぶ。ポップアップメニューの項目にはまだアクセシビリティの名前が無く UIA で探せないため、キーを送る。開かなければ、同じことをするツールバーのボタンを押す
3. 設定ダイアログの全ページを順に開いて撮る。最初のページのチェックボックスを切り替え、OK を押す
4. 「設定」→「タイプ別設定...」で同じようにページを回り、キャンセルを押す
5. 本体に文字を送り（`WM_CHAR`）、ステータスバーの「カーソル位置」が変わることを確かめる
6. ウィンドウを閉じて、「保存しますか」に「保存しない」と答え、プロセスが終わることと、ファイルが変わっていないことを確かめる

`CI` が設定されているか `UME_UIA_TESTS=1` のときだけ動く（#164 と同じ。手元で動かすと、ウィンドウを開いて文字を送るため）。要素が見つからないときは、UIA が見ているツリーをメッセージに出して失敗する。

## macOS ランナーの許可と Accessibility API

2026-10-22 に macos-15（15.7.9、arm64）のランナーで確かめた。

- SIP は無効（`csrutil status`: disabled）。システムと利用者の TCC データベース（`/Library/Application Support/com.apple.TCC/TCC.db` と `~/Library/Application Support/com.apple.TCC/TCC.db`）は、`sudo sqlite3` で読める
- 画面収録（`kTCCServiceScreenCapture`）、アクセシビリティ（`kTCCServiceAccessibility`）、`AppleEvents` は、`/bin/bash` と `/usr/bin/osascript` に、イメージが最初から許可（auth_value 2）を付けている。TCC の行を足す必要は無く、足していない
- 実際に UME のウィンドウ付きの写真が撮れ（`macos-arm64-main-window.png`）、`osascript` でメニューバー・ウィンドウの一覧を読み、メニュー項目を押せた

`macos_ax.rs` がすること: 本体が上がるのを、テキストビュー（`AXTextArea` の `テキストエディタ`）がアクセシビリティの木に出ることで待ち、ツールバーのボタン（新規作成・上書き保存・共通設定...）があることを確かめる → 「設定」→「共通設定」を押し、2 つ目のウィンドウ（共通設定）が開くのを確かめて写真を撮り、Esc で閉じる → 「ヘルプ」→「バージョン情報」を押し、本体の木に `AXWindow`（名前 `バージョン情報`）とその中の `OK` ボタン・バージョンの文字が出ることを確かめ、写真を撮り、Esc で木から消えることを確かめる → 文字（`abc`）を送り、ステータスバーの「カーソル位置」が変わることと、ファイルが変わっていないことを確かめる。メニュー項目は名前の前方一致で探す。見つからなければ失敗にして、木を出す。

本体の中の要素（ツールバー・テキストビュー・ステータスバー・About のパネル）は、AccessKit（`crates/ume-ui/src/a11y.rs`）の名前で、macos-15 のランナーの木に出る（実行 39695553047 と 39696042566 の `script.log` に木がある）。メニューバーは macOS のネイティブのもの。共通設定のウィンドウは、中の要素は木に出ず、ウィンドウの名前とタイトルバーだけが見える（Windows のようにページを回ることは、まだできない）。

### 手動の macOS 確認

```sh
gh workflow run mac-check.yml --ref <ブランチ> -f script=scripts/mac-checks/ax-test.sh
gh run watch
gh run download <実行の id> -n mac-check
```

`script` はリポジトリの中のスクリプト（相対パス）。`ume` と `ume-script` を release でビルドしてから `bash` で動かす（`-f build=false` でビルドを省ける）。写真は `$UME_E2E_SCREENS_DIR` に PNG で書くと成果物になる。例は `scripts/mac-checks/`。`pack-build.sh` は、ビルドした `ume` と `ume-script` を `ume-macos-arm64.tar.gz`（パッケージと同じく、フォルダ `ume-macos-arm64/` の中の `UME Editor.app` に 2 つ）にして成果物に入れる。Rust の無い Mac（Tart の VM など）で、PR の UME を動かすために使う。書き込み権限のある人だけが動かせ、フォークの PR のコードは動かない。ワークフローのファイルが既定のブランチに入ってから `gh workflow run` で見つかる。

## 写真を取ってくる

```sh
scripts/e2e-screens.sh <PR 番号 | 実行の id | ブランチ名> [保存先]
```

`gh run download` で、その PR（またはブランチ）の最新の完了した実行から `e2e-screens-*` を取ってくる。保存先の既定はリポジトリの外（`$UME_E2E_SCREENS_OUT`、なければ `${TMPDIR:-/tmp}/ume-e2e-screens`）で、`<保存先>/<実行の id>/<アーティファクト名>/<対象>-<場面>.png` に置く。アーティファクトは `e2e-screens-<対象>`、14 日で消える。

## 手元で動かす

```sh
cargo build --release --locked -p ume -p ume-script
UME_E2E_SCREENS_DIR=/some/folder \
  cargo test --release --locked -p ume --test e2e_screens -- --ignored --test-threads=1 --nocapture
# Windows だけ
UME_UIA_TESTS=1 cargo test --release --locked -p ume --test windows_uia -- --ignored --nocapture
```

Linux では `xvfb-run` の中で動かし、ImageMagick を入れておく。
