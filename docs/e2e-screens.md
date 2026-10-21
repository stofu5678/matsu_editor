# E2E のスクリーンショットと UI Automation

実物の `ume` を 5 つの対象（Windows x64・ARM64、macOS ARM64、Linux x64・ARM64）で起動して、画面の写真を撮る。写真は人間や AI が見るためのもので、保存した画像との画素の比較はしない（文字の描き方が実行ごと・システムごとに違うため）。Windows では、UME 自身のウィンドウを UI Automation（UIA）で外から操作して、ユーザーと同じ手順で動かす。

設定画面が見切れる（#160）、フォントが効かない（#171）といった問題を、デバイスを用意せずに CI の画像で見つけるためのしくみ。

## 構成

| 場所 | 役割 |
|------|------|
| `.github/workflows/e2e.yml` の `screens` ジョブ | 5 つの対象で写真を撮り、アーティファクトにする。ほかの CI と並行して動く |
| `crates/ume/tests/e2e_screens.rs` | 場面ごとに `ume` を一時プロファイルで起動し、起動時のキーマクロ（`-M=`）でウィンドウを開き、描かれたら画面を PNG にする |
| `crates/ume/tests/windows_uia.rs` | Windows だけ。UIA で UME のウィンドウを操作する |
| `crates/ume/tests/common/` | 起動・画面の保存・UIA の共通部分 |
| `scripts/e2e-screens.sh` | 写真を手元に取ってくる |

## 写真の撮り方

画面全体を撮る（開いているメニューやほかのウィンドウも写る）。

- Windows: PowerShell と System.Drawing（`crates/ume/tests/common/capture.ps1`）。Windows のランナーには操作できるデスクトップがある。描画は WARP
- Linux: xvfb（仮想ディスプレイ）の画面を ImageMagick の `import -window root` で撮る。ワークフローが `imagemagick` を apt で入れる
- macOS: `screencapture`。ランナーの画面収録の許可が無いと、ウィンドウの写っていない写真になる。そのときはテストを失敗にせず、警告を出す

Windows ARM64 のランナー（`windows-11-arm`）は、デスクトップの全面に Windows の初回設定の画面（"Choose privacy settings"）が出ていて、画面の写真はその画面になる（2026-10-21 に確認）。そこで `UME_E2E_SOFT_SCREENS=1` にして、写真が撮れなくても失敗にせず、UIA のテストは写真を撮らない（UIA の操作そのものは動く）。Windows ARM64 の見た目は、Windows x64 の写真で見る。

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
