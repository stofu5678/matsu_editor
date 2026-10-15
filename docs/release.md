# リリース

[ADR-0014](adr/0014-development-process.md) の「バージョン」を実装したもの。仕組みは 3 つのワークフローと 2 つの設定ファイルでできている。

| ファイル | 役割 |
|---|---|
| `.github/workflows/release.yml` | main への push のたびに動く。リリース PR の更新、毎日版の公開、版のリリースへの添付 |
| `.github/workflows/package.yml` | 5 つの対象で `ume` をリリースビルドして、アーカイブにする（再利用する部品。公開はしない） |
| `.github/workflows/package-check.yml` | リリース関係のファイルを変える PR で、`package.yml` だけを動かす（公開しない） |
| `release-please-config.json`・`.release-please-manifest.json` | release-please の設定と、今の版 |

## 毎日版（nightly）

- main に push（PR のマージ）するたびに、5 つの対象のパッケージを作り、プレリリース `nightly` に載せる。「毎日」と呼んでいるが、実際はマージのたび（ADR-0014 のとおり）
- `nightly` は動くタグ。push のたびにタグを新しいコミットに移し、アセットを同じ名前で置き換える。リリースの説明にコミットの SHA と件名を書く
- PR からは何も公開しない
- 同時に 2 つの実行は走らせない（`concurrency`）。新しい push が待ちに入ると、古い待ちは捨てられるので、`nightly` が古いコミットに戻ることはない

### アセットの名前

| 対象 | ファイル名 |
|---|---|
| Windows x64 | `ume-<版>-windows-x64.zip` |
| Windows ARM64 | `ume-<版>-windows-arm64.zip` |
| macOS ARM64 | `ume-<版>-macos-arm64.tar.gz` |
| Linux x64 | `ume-<版>-linux-x64.tar.gz` |
| Linux ARM64 | `ume-<版>-linux-arm64.tar.gz` |

`<版>` は、毎日版では `nightly`、版のリリースではタグ（例: `v0.1.0`）。中身は `ume-<対象>/` のディレクトリで、実行ファイル（`ume` / `ume.exe`）、`LICENSE-MIT`、`LICENSE-APACHE`、短い `README.txt` が入る。署名はまだない。

## 版のリリース

- 版は main のコミットから決める。PR は squash でマージするので、PR のタイトル（Conventional Commits）がコミットになる。`feat` で minor、`fix` で patch を上げる。0.x のあいだは、破壊的変更（`!` や `BREAKING CHANGE`）も minor を上げる。1.0 にする時期は人が決める（1.0 にするときは、`release-please-config.json` の `bump-minor-pre-major` を外し、`Release-As: 1.0.0` を付けたコミットを入れる）
- [release-please](https://github.com/googleapis/release-please) が、前のリリースからのコミットを見て、リリース PR（`chore: release x.y.z`）を作って更新し続ける。PR には `Cargo.toml` の `[workspace.package] version`、`CHANGELOG.md`、`version.txt` の変更が入る
- 版を上げると `Cargo.lock` のワークスペースのクレートの版も変わる（変わらないと `--locked` のビルドが落ちる）ので、同じワークフローが `cargo update --workspace` を回して、リリース PR のブランチに足す
- リリース PR をマージすると、release-please がタグ `vX.Y.Z` とリリースを作り、そのコミットから作った 5 つのパッケージを、同じ実行の中でリリースに添付する
- 最初のリリースは、これまでの全コミットを拾うので、`CHANGELOG.md` が長くなる。`0.1.0` になる

### なぜ release-please か

| 候補 | 評価 |
|---|---|
| release-please | Conventional Commits から版・変更履歴・リリース PR を作る。Rust の `Cargo.toml` を `toml` の更新で直せ、crates.io に出さない構成に合う。squash マージと相性が良い。ADR-0014 に名前がある。採用 |
| release-plz | Rust 向けで `Cargo.lock` の更新も自動。ただし crates.io への公開と、クレートごとの版を前提にしていて、ワークスペース全体で 1 つの版（`version.workspace`）を持つこの構成には合わせにくい |
| cargo-release・自作のスクリプト | リリース PR とマージの自動化を自分で作ることになる |

release-please の Rust 向けの種類（`rust`）は、クレートごとの `Cargo.toml` の `version` を書き換える。このワークスペースは版を 1 か所（`[workspace.package]`）に持つので、種類は `simple` にして、`Cargo.toml` の 1 か所だけを `toml` の更新で直している。`Cargo.lock` は上のとおりワークフローで直す。

## 人が設定すること

**リリース PR を自動でマージするには、次の設定が要る。** 設定しなくても、リリース PR を人が手でマージすれば動く。

`GITHUB_TOKEN` で作った PR では、ワークフロー（CI）が動かない。main は CI が通らないとマージできないので、リリース PR は止まる。次のどちらかで、CI が動くトークンを用意する。

1. GitHub App（おすすめ）: App を作り、リポジトリに入れる。権限は Contents（Read & write）、Pull requests（Read & write）。App の ID と秘密鍵を secrets に入れ、`actions/create-github-app-token` でトークンを作る形に `release.yml` を直す
2. 細かい権限を指定した個人用トークン（fine-grained PAT）: このリポジトリだけ、Contents と Pull requests の Read & write

どちらでも、できたトークンを secret `RELEASE_PLEASE_TOKEN` に入れると、`release.yml` が自動で使う（`GITHUB_TOKEN` より優先する）。このとき `release.yml` は、リリース PR に `gh pr merge --auto --squash` を付ける。そのために次も要る。

- リポジトリの設定で「Allow auto-merge」を有効にする
- main のブランチ保護で、必須のチェック（CI の各ジョブ）を指定してある（`--auto` はチェックが通るまで待つ）

このリポジトリの設定や secrets は、AI は変えない。

## 手でリリースするとき

自動化が壊れたとき、または急ぐとき。

1. 版を決める。`Cargo.toml` の `[workspace.package] version` を直し、`.release-please-manifest.json` の版も同じにして、`cargo update --workspace` で `Cargo.lock` を直し、PR にして main にマージする
2. 「Actions」の「Release」を見て、そのコミットのパッケージ（artifact `package-*`）を取る
3. タグとリリースを作る

   ```sh
   gh release create vX.Y.Z --target <SHA> --title vX.Y.Z --generate-notes
   gh release upload vX.Y.Z ume-vX.Y.Z-*.zip ume-vX.Y.Z-*.tar.gz
   ```

   ファイル名は上の表のとおり（artifact の中の名前は `ume-<対象>.zip` / `.tar.gz` なので、`ume-vX.Y.Z-<対象>` に付け替える）
4. release-please が混乱したら、`.release-please-manifest.json` を最後のリリースの版に合わせる

毎日版だけが壊れたときは、`nightly` のリリースを消してから main に push し直すと作り直される。

## 署名

配布物にはまだ署名していない。macOS の公証と Windows の署名は、ADR-0014 のとおり、最初の設定だけ人が行ってから CI に足す（macOS は Apple Developer Program への登録が要る）。それまでは、macOS では「開発元を確認できない」、Windows では SmartScreen の警告が出る。

## 費用

- `package.yml` は 5 つの対象でリリースビルドする。`Swatinem/rust-cache` の `shared-key: release` で、リリースビルドのキャッシュを `ci.yml` のテスト用とは分けている
- パッケージは main の push のたびに作るので、マージが続くと実行時間が増える。`concurrency` で待ちは 1 つに絞っている。足りなければ、毎日 1 回（`schedule`）に変える案がある
