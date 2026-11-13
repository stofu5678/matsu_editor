# リリース

[ADR-0014](adr/0014-development-process.md) の「バージョン」を実装したもの。仕組みは 3 つのワークフローと 2 つの設定ファイルでできている。

| ファイル | 役割 |
|---|---|
| `.github/workflows/release.yml` | main への push のたびに動く。リリース PR の更新、毎日版の公開、版のリリースへの添付 |
| `.github/workflows/package.yml` | 5 つの対象で `matsu` と `matsu-script` をリリースビルドして、アーカイブにする（再利用する部品。公開はしない） |
| `.github/workflows/package-check.yml` | リリース関係のファイルを変える PR で、`package.yml`、Windows のインストーラの試験、パッケージマネージャのファイルの検査を動かす（公開しない） |
| `release-please-config.json`・`.release-please-manifest.json` | release-please の設定と、今の版 |

## 毎日版（nightly）

- main に push（PR のマージ）するたびに、5 つの対象のパッケージを作り、プレリリース `nightly` に載せる。「毎日」と呼んでいるが、実際はマージのたび（ADR-0014 のとおり）
- `nightly` は動くタグ。push のたびにタグを新しいコミットに移し、アセットを同じ名前で置き換える。リリースの説明にコミットの SHA と件名を書く
- PR からは何も公開しない
- 同時に 2 つの実行は走らせない（`concurrency`）。新しい push が待ちに入ると、古い待ちは捨てられるので、`nightly` が古いコミットに戻ることはない

### アセットの名前

| 対象 | ファイル名 |
|---|---|
| Windows x64 | `matsu-<版>-windows-x64.zip`、`matsu-<版>-windows-x64-setup.exe`（インストーラ） |
| Windows ARM64 | `matsu-<版>-windows-arm64.zip`、`matsu-<版>-windows-arm64-setup.exe`（インストーラ） |
| macOS ARM64 | `matsu-<版>-macos-arm64.tar.gz`、`matsu-<版>-macos-arm64.dmg` |
| Linux x64 | `matsu-<版>-linux-x64.tar.gz` |
| Linux ARM64 | `matsu-<版>-linux-arm64.tar.gz` |
| Linux x64（パッケージ） | `matsu-<版>-linux-x64.deb`、`matsu-<版>-linux-x64.rpm` |
| Linux ARM64（パッケージ） | `matsu-<版>-linux-arm64.deb`、`matsu-<版>-linux-arm64.rpm` |

`<版>` は、毎日版では `nightly`、版のリリースではタグ（例: `v0.1.0`）。中身は `matsu-<対象>/` のディレクトリで、実行ファイル（`matsu` / `matsu.exe`）、同梱のマクロのエンジンを動かす `matsu-script`（`matsu-script.exe`。`matsu` は自分の隣にあるものを使う）、`LICENSE-MIT`、`LICENSE-APACHE`、`THIRD_PARTY_NOTICES.txt`（プログラムに入っている第三者のコードの著作権表示とライセンス。ライセンスが配布物に添えることを求める）、アイコンの絵（`icons/`。`assets/icons` の SVG・PNG・ICO。ライセンスはコードと同じ）、短い `README.txt` が入る。Linux では、FreeType を同梱してビルドしていないことを確かめる（同梱の FreeType のライセンスは `THIRD_PARTY_NOTICES.txt` に含まれないため。CI のテストと同じ確認）。署名はまだない。

Linux には、tar.gz のほかに `.deb` と `.rpm` を付ける（ADR-0032 の段階 A。`scripts/package-linux.sh`）。`matsu` と `matsu-script` は `/usr/lib/matsu-editor/` に並べ、`/usr/bin/matsu` はそこへのシンボリックリンク（`matsu` は `current_exe`、つまり実体のパスの隣の `matsu-script` を使う）。`.desktop`（`MimeType=text/plain;` だけで、関連付けは奪わない）、AppStream の metainfo、`hicolor` のアイコン、ライセンス類（`/usr/share/doc/matsu-editor/`）も入る。依存する共有ライブラリは、`.deb` は `dpkg-shlibdeps`、`.rpm` は rpm が実行ファイルから求める。インストール・削除のスクリプトは持たず、設定のフォルダー（`~/.config/matsu-editor`）に触れない。`.deb` は作った環境（Ubuntu 24.04）の glibc などを要求するので、それより古い配布版には入らない。CI（`package.yml`）が `.deb` を入れて外し、利用者のファイルが残ることを確かめ、`.rpm` は Fedora のコンテナで同じことを確かめる（`.github/scripts/test-linux-install.sh`）。署名はない。

macOS だけ、実行ファイルを裸で置かず、アプリのバンドル `MATSU Editor.app` に入れる（`scripts/package-mac-app.sh`）。裸の実行ファイルを Finder で開くと Terminal.app が起動して黒い画面が出るため（ユーザーの依頼、2026-10-22）。`matsu` と `matsu-script` は `MATSU Editor.app/Contents/MacOS/` に並べる（`matsu` が自分の隣の `matsu-script` を使うので変わらない）。`.dmg`（ADR-0032 の段階 A。`scripts/package-mac-dmg.sh`）は、ボリューム名「MATSU Editor」で、`MATSU Editor.app` と `/Applications` へのリンクだけを入れる（UDZO。arm64 の macOS ならどれでも読め、他の OS の展開ツールも読める。ULFO は小さいが 10.11 以降だけ）。アプリはアドホック署名のまま、`.dmg` には署名しない。`.dmg` は何もインストールしないので、`~/Library/Application Support/matsu-editor` には触れない。ほかは同じで、`LICENSE-*`、`THIRD_PARTY_NOTICES.txt`、`icons/`、`README.txt`（初回の開き方つき）は `MATSU Editor.app` と同じ階層に置く。`Contents/Info.plist` の識別子は `io.github.stofu5678.matsu-editor`、最低の macOS は 11.0（arm64 が動く最も古い版）、アイコンは `assets/icons` の Mac 用の PNG（`matsu_editor_mac_*.png`）から作った `.icns`。ライセンス類（`LICENSE-*`、`THIRD_PARTY_NOTICES.txt`）は `Contents/Resources/` にも入れる（アプリだけを Applications に移しても、第三者のコードの表示が離れないように）。ターミナルからは `MATSU Editor.app/Contents/MacOS/matsu ファイル` で使える。`matsu.ini` の場所と多重起動の判定は、実行ファイルの場所に依らない（macOS は `~/Library/Application Support/matsu-editor/matsu.ini` だけ）ので変わらない。ファイルの種類の関連づけ（`CFBundleDocumentTypes`）は、テキストとソースコードを「別の選択肢」（`LSHandlerRank` が `Alternate`）として入れた。「このアプリで開く」に出るだけで、既定のアプリは奪わない。Finder から開く処理（`on_open_urls`）は `crates/matsu-ui/src/app.rs` にあるが、バンドルからの動作は Mac で確かめていない。

## Windows のインストーラ

[ADR-0032](adr/0032-install.md) の段階 A。zip と同じ中身を、Inno Setup 6.7.4 で `matsu-<対象>-setup.exe` にまとめる（定義は `packaging/windows/matsu.iss`、作る手順は `package.yml` の「Install Inno Setup」「Build the installer」）。署名はまだない。

- 既定は利用者ごと（`%LOCALAPPDATA%\Programs\MATSU Editor`、管理者権限なし）。最初のダイアログで「全利用者」（`Program Files`）も選べる。コマンドラインでは `/CURRENTUSER`・`/ALLUSERS`。静かに入れるには `/VERYSILENT /SUPPRESSMSGBOXES`
- 画面の言語は Windows の表示言語に従う（日本語の Windows では日本語、それ以外は英語。Inno Setup 付属の `Japanese.isl` と、選択肢の日本語の文言を使う。言語を聞くダイアログは出さない。`/LANG=ja`・`/LANG=en` で決められる）
- スタートメニューの項目と、「アプリ」（アンインストール）の項目を作る。`AppId` は固定で、同じ `AppId` の古い版の上に入れると更新になる（x64 と ARM64 も同じ `AppId`）
- 選択肢（`/TASKS="openwith,associatetxt,addtopath"` のように名前でも指定できる）
  - `openwith`（既定でオン）: 「プログラムから開く」の一覧に `matsu.exe` を出す（`Applications\matsu.exe`）
  - `associatetxt`（既定でオフ）: `.txt` を開くプログラムとして登録し、「既定のアプリ」に出す。Windows 10 以降は、既定のプログラムを決められるのは利用者だけ（ハッシュで守られている）ので、`.txt` を奪うことはしない。サクラの拡張子の一覧に合わせるのは、別の PR
  - `addtopath`（既定でオフ）: インストール先のフォルダーを PATH に足す。アンインストールで取り除く
- 利用者のデータには触れない（ADR-0032 の決めたこと 3）。インストール・更新・アンインストールは、`matsu.ini`、`unsaved\`、`plugins.ini`、`databases.ini`、`plugins\`、履歴、設定のフォルダー（`%APPDATA%\matsu-editor`）を作らず、書き換えず、消さない。インストール先にも `matsu.ini` を作らない（作ると持ち運び用の動きに固定されるため）。アンインストールはインストーラが入れたファイルだけを消すので、利用者がインストール先に置いた持ち運び用の `matsu.ini` は残り、そのフォルダーも残る。「設定も消す」という選択肢はない
- `.github/scripts/test-windows-installer.ps1` が、`package-check.yml` の `installer-test`（`windows-2025` と `windows-11-arm`）で、利用者ごとの静かなインストール、更新、アンインストール、選択肢、全利用者を試し、`%APPDATA%\matsu-editor` の `matsu.ini` と `unsaved\x.txt` がハッシュも更新日時も変わらないことを確かめる
- `matsu.exe` には版情報（製品名「MATSU Editor」、版、発行元、ライセンス）が入る（`crates/matsu/matsu.rc`。`build.rs` が `Cargo.toml` の版を入れる）。発行元は、`LICENSE-MIT` の著作権者と同じ `stofu5678`

## インストールの方法ごとに触れるもの

ADR-0032 の決めたこと 3 のとおり、どの方法も、利用者のデータ（`matsu.ini`、`unsaved/`、`plugins.ini`、`databases.ini`、`plugins/`、履歴、設定のフォルダー）を作らず、書き換えず、消さない。アンインストールの後も、設定と未保存の文書は残る（消したいときは、利用者が設定のフォルダーを自分で消す）。

| 方法 | 入れるもの | 外すと |
|---|---|---|
| Windows のインストーラ（`-setup.exe`） | インストール先（既定は `%LOCALAPPDATA%\Programs\MATSU Editor`、全利用者は `Program Files`）、スタートメニューの項目、「アプリ」の項目（レジストリの `Uninstall`）。選択で、`Applications\matsu.exe`・`.txt` の登録・PATH | 入れたファイルと上の項目だけを消す。持ち運び用の `matsu.ini` を置いたなら、それとフォルダーは残る |
| Windows の zip | なし（展開するだけ） | フォルダーを消す。持ち運び用の `matsu.ini` もそこにあれば消える |
| macOS の `.dmg` | なし（`MATSU Editor.app` を `Applications` に置くのは利用者） | アプリを捨てる。設定は `~/Library/Application Support/matsu-editor` に残る |
| macOS の tar.gz | なし | 同上 |
| Linux の `.deb`・`.rpm` | `/usr/lib/matsu-editor/`、`/usr/bin/matsu`、`.desktop`、metainfo、アイコン、`/usr/share/doc/matsu-editor/` | パッケージが入れたファイルだけを消す。`~/.config/matsu-editor` は残る |
| Linux の tar.gz | なし | フォルダーを消す |

## パッケージマネージャのファイル

ADR-0032 の決めたこと 5 のとおり、Homebrew・Scoop・winget のファイルは `packaging/` に置くだけで、まだ公開していない。tap・bucket のリポジトリを作ることと、winget-pkgs への最初の PR は、オーナーが行う（段階 B）。

| 方法 | ファイル | 何を使うか |
|---|---|---|
| Homebrew（自分の tap） | `packaging/homebrew/matsu-editor.rb` | `.dmg`（macOS ARM64）。`MATSU Editor.app` を `/Applications` に置き、`matsu` コマンドは、アプリの中の `matsu` を呼ぶ小さなシェルスクリプトとして入れる（`matsu` は自分の実体の隣の `matsu-script` を使うので、アプリの中へのシンボリックリンクにはしない）。アドホック署名なので、`caveats` に隔離属性の外し方を書く。`zap` は書かない（設定を消してしまうため）。`livecheck` は GitHub のリリース |
| Scoop（自分の bucket） | `packaging/scoop/matsu-editor.json` | 持ち運び用の zip（x64・ARM64）。`bin` は `matsu.exe`。`checkver` と `autoupdate` は GitHub のリリース |
| winget | `packaging/winget/stofu5678.MATSUEditor*.yaml`（マニフェスト 1.28.0。バージョン・インストーラ・既定の言語 `en-US`・`ja-JP` の 4 ファイル） | Inno Setup のインストーラ（`InstallerType: inno`）。x64・ARM64 のそれぞれに、利用者ごと（`/CURRENTUSER`）と全利用者（`/ALLUSERS`）。`ProductCode` は `matsu.iss` の `AppId` に `_is1` を付けたもの |

- **Scoop に `persist` を付けない。** Scoop は版ごとのフォルダー（`apps\matsu-editor\<版>`）に展開し、更新で古い版のフォルダーを消す。`matsu.ini` を `persist` すると、インストール先に `matsu.ini` ができて、持ち運び用の動きに固定される（設定の置き場が `%APPDATA%\matsu-editor` からインストール先に移る）。付けなければ、設定は `%APPDATA%\matsu-editor` に置かれ、更新・アンインストールでも残る。代償は、利用者が Scoop のフォルダーに持ち運び用の `matsu.ini` を置くと、更新で消えること（`notes` に書いてある）
- 4 つのファイルとも、版は `0.0.0`、SHA-256 は 0 だけの見本になっている。新しい版のリリースを出したら、`tools/packaging/update-manifests.py <版>` が、リリースのアセットを `gh release download` で取り（読むだけで、何も送らない）、5 つの SHA-256（`.dmg`・zip 2 つ・`-setup.exe` 2 つ）と版、URL を、すべてのファイルに書く。ファイルが想定と違えば、何も書かずに止まる。`--assets-dir` を付けると、ダウンロードの代わりに手元のファイルを使う
- `tools/packaging/check-manifests.py` が、ファイルの構文、winget のスキーマ（Microsoft の `winget-cli` の固定したコミットから取り、SHA-256 を確かめる）、ファイル同士と `matsu.iss` の食い違い（版、`ProductCode`、アセット名）、`zap` と `persist` が無いことを確かめる。CI（`package-check.yml` の `manifests`）が動かす。Homebrew の cask は `ruby -c` で構文だけ確かめる（`brew style` は macOS の実行機に Homebrew を入れる必要があり、軽くないため）
- 段階 B で、tap・bucket に置くとき: Homebrew は `Casks/matsu-editor.rb`、Scoop は `bucket/matsu-editor.json` に、更新したファイルをコピーする。winget は、更新した 4 つのファイルを `manifests/s/stofu5678/MATSUEditor/<版>/` に置いて、`wingetcreate` か手で PR にする

## リリースビルドのプロファイル

`Cargo.toml` の `[profile.release]` は `strip = true`（シンボルを取り除く）だけを指定している。Linux x64 で測った大きさ（表の大きさと時間は Linux x64 だけの値）と、リリースビルド 1 回の時間（`cargo build --release -p matsu -p matsu-script`、依存を含めて全部作り直す。測定中は同じ計算機で他のビルドも動いていたので、時間は目安）は次のとおり。

| 設定 | `matsu` | `matsu-script` | 時間 |
|---|---|---|---|
| 既定 | 74.0 MB | 5.9 MB | 623 秒 |
| `strip = true` | 50.8 MB | 4.6 MB | 675 秒 |
| `strip = "debuginfo"` | 74.0 MB | 5.9 MB | 583 秒 |
| `lto = "thin"` | 75.2 MB | 5.9 MB | 597 秒 |
| `codegen-units = 1` | 56.0 MB | 5.6 MB | 583 秒 |
| `strip` + `lto = "thin"` | 52.1 MB | 4.6 MB | 454 秒 |
| `strip` + `lto = "thin"` + `codegen-units = 1` | 42.4 MB | 4.5 MB | 390 秒 |

- `strip = true` で約 31% 小さくなる。ビルド時間はほぼ変わらない。`strip = "debuginfo"` は、リリースの既定ではデバッグ情報がもともと入っていないので、効かない（大きさが変わるのはシンボル表）
- 代償: Linux では、パニックのバックトレースに関数名が出ず、アドレスだけになる（Windows と macOS の挙動は確かめていない）。パニックのメッセージ（場所と理由）は変わらない。`panic` は `unwind` のまま
- `lto = "thin"` だけでは小さくならない。`codegen-units = 1` を足すと、さらに小さくなる。ただし CI のビルドが長くなる恐れがあるので、今は入れていない（測定では遅くならなかったが、測定の条件がそろっていない）。入れるかどうかは、5 つの対象の CI の時間を見て決める

### シンボル付きのビルド

バックトレースに関数名が要るときは、取り除かずにビルドする。

```sh
CARGO_PROFILE_RELEASE_STRIP=none cargo build --release --locked -p matsu -p matsu-script
```

配布物と同じコミットで作れば、同じコードのシンボル付きが得られる（バイナリのビットまで同じになるとは限らない）。

## 版のリリース

- 版は main のコミットから決める。PR は squash でマージするので、PR のタイトル（Conventional Commits）がコミットになる。`feat` で minor、`fix` で patch を上げる。0.x のあいだは、破壊的変更（`!` や `BREAKING CHANGE`）も minor を上げる。1.0 にする時期は人が決める（1.0 にするときは、`release-please-config.json` の `bump-minor-pre-major` を外し、`Release-As: 1.0.0` を付けたコミットを入れる）
- [release-please](https://github.com/googleapis/release-please) が、前のリリースからのコミットを見て、リリース PR（`chore: release x.y.z`）を作って更新し続ける。PR には `Cargo.toml` の `[workspace.package] version`、`CHANGELOG.md`、`version.txt` の変更が入る
- 版を上げると `Cargo.lock` のワークスペースのクレートの版も変わる（変わらないと `--locked` のビルドが落ちる）ので、同じワークフローが `cargo update --workspace` と `cargo about generate`（`THIRD_PARTY_NOTICES.txt` にも版が載るため）を回して、リリース PR のブランチに足す
- リリース PR をマージすると、release-please がタグ `vX.Y.Z` とリリースを作り、そのコミットから作った 5 つのパッケージを、同じ実行の中でリリースに添付する
- 最初のリリースは、これまでの全コミットを拾うので、`CHANGELOG.md` が長くなる。`initial-version` で `0.1.0` にしてある（指定しないと release-please は最初を 1.0.0 にする。最初のリリース PR の版は、マージの前に確かめる）

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
   gh release upload vX.Y.Z matsu-vX.Y.Z-*.zip matsu-vX.Y.Z-*.tar.gz matsu-vX.Y.Z-*.dmg matsu-vX.Y.Z-*.deb matsu-vX.Y.Z-*.rpm
   ```

   ファイル名は上の表のとおり（artifact の中の名前は `matsu-<対象>.zip` / `.tar.gz` / `.dmg` / `.deb` / `.rpm` なので、`matsu-vX.Y.Z-<対象>` に付け替える）
4. release-please が混乱したら、`.release-please-manifest.json` を最後のリリースの版に合わせる

毎日版だけが壊れたときは、`nightly` のリリースを消してから main に push し直すと作り直される。

## Windows の C の実行時ライブラリ

Windows 版の `matsu.exe` と `matsu-script.exe` には、C の実行時ライブラリ（MSVC の CRT と Universal CRT）を組み込む（静的リンク）。Visual C++ 再頒布可能パッケージ（`VCRUNTIME140.dll`）が入っていない Windows でも、そのまま起動できるようにするため（2026-10-21 にユーザーが判断）。

- 何も入れていない Windows（Windows サンドボックス）で、動的リンクの版が「VCRUNTIME140.dll が見つからない」で起動しなかったことがきっかけ
- 指定は `.cargo/config.toml` の `target-feature=+crt-static`。手元のビルドと `package.yml` のビルドに効く
- 環境変数の `RUSTFLAGS` は `.cargo/config.toml` の指定を置き換える。`ci.yml` と `e2e.yml` は `RUSTFLAGS` を使うので、テスト用のビルドは DLL を使ったままになる
- `package.yml` は、macOS では `MATSU Editor.app` の `Info.plist`（`plutil -lint`、識別子、実行ファイル）、`matsu-script`、`.icns` があることと、署名が壊れていないこと（`codesign --verify --deep --strict`）を確かめる。続けて `.dmg` を作り、読み取り専用でマウントして（`scripts/check-mac-dmg.sh`）、アプリ（署名が通る）と `/Applications` へのリンクがあることを確かめて外す。`package-check.yml` は `scripts/package-mac-app.sh` と `assets/icons/` の変更でも動く
- `package.yml` は、配る `matsu.exe` と `matsu-script.exe` が C の実行時ライブラリの DLL を読み込んでいないことを確かめる（`.github/scripts/pe_imports.py`）

## 署名

配布物にはまだ署名していない。macOS の公証と Windows の署名は、ADR-0014 のとおり、最初の設定だけ人が行ってから CI に足す（macOS は Apple Developer Program への登録が要る）。macOS のバンドルは、リンカーが裸の実行ファイルに付けるのと同じ、アドホック署名（`codesign -s -`）だけ付けている（Developer ID も公証もない）。それまでは、macOS では「開発元を確認できない」、Windows では SmartScreen の警告が出る。

## 費用

- `package.yml` は 5 つの対象でリリースビルドする。`Swatinem/rust-cache` の `shared-key: release` で、リリースビルドのキャッシュを `ci.yml` のテスト用とは分けている
- パッケージは main の push のたびに作るので、マージが続くと実行時間が増える。`concurrency` で待ちは 1 つに絞っている。足りなければ、毎日 1 回（`schedule`）に変える案がある
