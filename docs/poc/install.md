# ADR-0032 下調べ: インストーラ・署名・パッケージマネージャ・ストア

調べた日: 2026-11-13（担当: 調査のサブエージェント。リポジトリは読むだけ、Web は公式ページ中心）。
印: 確認 = ページかファイルを読んだ。推測 = 読んだことからの推論か一般知識。未確認 = 読めていない。
Web の検索結果の要約だけで一次ページを読んでいないものは「要約のみ」と書いた。

## 1. リポジトリの現状（ファイルを読んで確認）

### 1.1 リリースの成果物

`.github/workflows/release.yml`（push to main で release-please、nightly、版の添付）と `package.yml`（再利用。5 つの対象でビルドして artifact にするだけ）。

| ファイル | 中身 |
|---|---|
| `matsu-vX.Y.Z-windows-x64.zip`、`-windows-arm64.zip` | `matsu-<対象>/` の中に `matsu.exe`、`matsu-script.exe`、`LICENSE-MIT`、`LICENSE-APACHE`、`THIRD_PARTY_NOTICES.txt`、`icons/`（SVG・PNG・`matsu_editor.ico`）、`README.txt` |
| `-macos-arm64.tar.gz` | 同じ構成。ただし実行ファイルの代わりに `MATSU Editor.app`（`scripts/package-mac-app.sh` が作る）。`Contents/MacOS/matsu`、`matsu-script`、`Contents/Resources/matsu_editor.icns`、ライセンス 3 点、`Info.plist` |
| `-linux-x64.tar.gz`、`-linux-arm64.tar.gz` | Windows と同じ構成（`matsu`、`matsu-script`、ライセンス、`icons/`、`README.txt`）。`.desktop` ファイルも、アイコンの置き場所への配置もない |

- `matsu` は自分の隣の `matsu-script` を探す（`crates/matsu-plugin/src/session.rs`）。だから 2 つを同じフォルダーに置く必要がある
- Windows は C ランタイムを静的リンク（`.cargo/config.toml` の `+crt-static`）。`package.yml` が `pe_imports.py` で DLL を読まないことを検査する。VC++ 再頒布パッケージは要らない
- Linux は OS の FreeType・Wayland・X11・xkbcommon・fontconfig・alsa などにリンクする（`package.yml` の apt のリスト）。`libfreetype2.a` が出来ていないことを検査する。ビルドに使う apt のリストはあるが、実行に要る共有ライブラリの一覧は **未確認**（`ldd` で確かめる必要がある。`.deb` の `Depends` に書くため）
- GPUI の Linux 描画は Vulkan（`docs/poc/ui-framework-result.md` 81 行、223 行。WSLg では llvmpipe）。`e2e.yml` は `mesa-vulkan-drivers libvulkan1` を入れる。Cargo.lock に wgpu 29.0.4（GPUI の依存）。Wayland と X11 の両方（`libwayland-dev`、`libx11-xcb-dev`、`libxkbcommon-x11-dev` のリンク）。実行時に Vulkan のドライバーが無い環境でどうなるか（OpenGL などへの退避）は **未確認**

### 1.2 署名

- Windows: 署名なし。`crates/matsu/build.rs` と `matsu.rc` はアイコン（ID 1）を埋めるだけで、版情報（VERSIONINFO）は **無い**（`matsu.rc` は ICON 1 行）。MSI/ストアの審査は「製品名・発行元・版が追加と削除に出る」ことを求める（5.3 を参照）ので、足す必要がある（推測）
- macOS: `codesign -s -`（アドホック署名）だけ。Developer ID も公証もない。`README.txt` に「開発元を確認できない」の回避方法を書いている（`package.yml`）。ハードンドランタイムも付けていない（`package-mac-app.sh` を読んだ限り）
- `docs/release.md` の「署名」節と ADR-0014 32 行: 署名と公証は「最初の設定だけ人が行い、あとは CI で自動」。macOS は Apple Developer Program が要る

### 1.3 識別子・名前

- アプリの識別子: `io.github.stofu5678.matsu-editor`（`Info.plist` の `CFBundleIdentifier`。`package.yml` が検査）
- 表示名「MATSU Editor」、`CFBundleShortVersionString` はワークスペースの版（`Cargo.toml` 0.13.0）。nightly は 0.0.0
- 開発元の著作権表示は「MIT OR Apache-2.0」（`NSHumanReadableCopyright`）。人の名前や会社名は、どの成果物にも入っていない。署名の証明書の「発行先の名前」を何にするかは未決（8 参照）

### 1.4 設定とデータの場所（`crates/matsu-config/src/location.rs`、ADR-0022、ADR-0026）

- Windows: 1 番目の候補は `<exe のフォルダー>\matsu.ini`（**あれば**これを読む。持ち運び用）。なければ `%APPDATA%\matsu-editor\matsu.ini`。書き込み先は「あるファイル、なければ最後の候補（`%APPDATA%`）」
- macOS: `~/Library/Application Support/matsu-editor/matsu.ini`
- Linux: `$XDG_CONFIG_HOME/matsu-editor/matsu.ini`、なければ `~/.config/matsu-editor/matsu.ini`
- `-PROF` を使うとプロファイル名の下位フォルダー
- 同じフォルダーに `unsaved/`（未保存の文書の自動保存。ADR-0022）、`plugins.ini`、`databases.ini`、履歴、`plugins\`（ユーザーのプラグイン。ADR-0026）、端末の設定は `matsu.ini` の `[Terminal]`
- 実行ファイルの隣の `plugins\` も読む（ADR-0026 の決めたこと 8）
- **インストーラへの影響**: (a) インストーラはインストール先に `matsu.ini` を作ってはいけない（作ると、全利用者の設定がその場所に固定され、持ち運び用の動きになる）。(b) アンインストールは、インストールしたファイルだけを消し、`%APPDATA%\matsu-editor` と `exe のフォルダーの matsu.ini`、`plugins\` の利用者のファイルに触れない。(c) 利用者が `exe のフォルダー` に自分の `plugins\` や `matsu.ini` を置いた場合、インストール先のフォルダーごと消す方式（`{app}` の再帰削除）は利用者のデータを消す。Inno Setup の既定は、インストーラが入れたファイルだけを消し、残りがあるとフォルダーを残す（推測。Inno の挙動は一般知識。実装で確かめる）

### 1.5 ファイルの関連付け、「開く」

- macOS: `Info.plist` に `CFBundleDocumentTypes`（`public.plain-text`、`public.source-code`、役割 Editor、`LSHandlerRank` Alternate）。`app.rs` 176 行で `on_open_urls`、`on_reopen` を受ける
- Windows・Linux: 関連付けの登録も、`.desktop` も、Explorer の「プログラムから開く」の登録も **無い**。コマンドラインは `matsu file...` を受け取る（`crates/matsu/src/main.rs`、`start_options.rs` に `-X=` `-Y=` `-CODE=` など）
- 1 つのプロセス（ADR-0016）。Windows は名前付きパイプ（`\\.\pipe\matsu-editor-<SID>-<key>`）、Unix は `instance/unix.rs`。2 度目の起動は最初のプロセスに渡す
- 自動更新の仕組みは **無い**（`updater` などの語を `crates/` で検索して該当なし）。更新はインストーラの上書きか、パッケージマネージャに任せる形になる

### 1.6 macOS の実行機（ADR-0027）と署名の衝突

- `package.yml` の macOS ジョブは、`MAC_RUNNER=self-hosted` のとき、ユーザーの Mac の上の使い捨て Tart VM（GitHub の JIT 実行機）で動く。nightly と版（`package-release`）の両方。ADR-0027 の 2026-11-06 の追記（147〜170 行）
- 守り: VM は使い捨て、**secrets が無い**（`release.yml` は `with:` だけを渡す。`package.yml` の先頭コメントも「It has no secrets」）、`GITHUB_TOKEN` は読み取りだけ、動かせるのは main への push と同じリポジトリの PR だけ、`pre-job` の確かめ。「リリースの公開（書き込み権限を持つ nightly・release のジョブ）は GitHub の Linux 実行機で動き、VM には来ない」
- つまり **Developer ID の秘密鍵と公証の資格を VM のジョブに渡す案は、ユーザーの前提（VM を使うのでローカル PC の情報が流出しない。簡単に GitHub の実行機に戻せる）に反する**。VM の中のジョブは、公開リポジトリの Release に載る成果物を書き換えられる、という残りの弱点（ADR-0027 追記の「変わる前提」）もある。鍵を VM に入れると、その弱点が「署名された成果物」まで広がる
- 署名を足す場合の選び方は 2 章の最後と ADR 案 3 に書いた

## 2. 署名（公式ページで確認）

### 2.1 Windows

#### Azure Artifact Signing（旧 Trusted Signing）

- 名前は **Artifact Signing**（`learn.microsoft.com/en-us/azure/artifact-signing/faq` の 2026-10-27 更新版。URL の旧名は `trusted-signing`）。確認
- 個人が使えるか（`.../artifact-signing/quickstart` の Prerequisites）。確認:
  > Public Trust certificates are available to organizations in the United States, Canada, the European Union, the United Kingdom, Australia, New Zealand, Japan, South Korea, Singapore, Switzerland, Norway, and Israel. Individual developers must be located in the United States or Canada.
  - つまり **日本に居る個人は Public Trust の証明書を取れない**。日本の「組織」は取れる（法的な事業体の名前、事業者の識別子、事業体のドメインのメール、事業体の住所が要る。検証は 1〜20 営業日）。個人事業主（DBA）が通るかは **未確認**（quickstart は「Organization or a DBA」と書くが、日本の屋号付き個人事業主の書類で通るかは書いていない）。オーナーが日本に居るか、法人か個人かは、このリポジトリからは分からない
  - 個人の検証の中身: Azure の請求アカウントの種類が Individual、政府発行の ID、AU10TIX の顔認証と Microsoft Authenticator の Verified ID。確認
- 条件: 有料の Azure サブスクリプション（無料・試用・スポンサーは不可。FAQ）。確認
- 値段: 公式の料金ページ（`azure.microsoft.com/en-us/pricing/details/artifact-signing/`）の取得結果には金額が出ていない（Basic は月 5,000 署名、Premium は 100,000 署名。金額は「$-」）。**金額は未確認**。プレビュー期間の Basic の月額は 9.99 ドルだったと記憶するが、これは推測で、現在の額ではない。FAQ: 料金は日割りでなく、選んだ SKU の全額（確認）
- 証明書は 3 日で切れる、署名には必ずタイムスタンプを付ける（`http://timestamp.acs.microsoft.com`）。鍵は渡されず、署名の時にだけサービスが使う（FIPS 140-3 Level 3）。EV 証明書は出さない、今後も出す予定なし。確認
- CI: GitHub Actions 用の action（`github.com/azure/artifact-signing-action`）、SignTool + dlib（Windows 10 1809 以降、.NET 8、Windows SDK 10.0.2261.755 以降）。認証は `DefaultAzureCredential`（サービスプリンシパルの環境変数や、GitHub の OIDC のフェデレーションが使える。後者は一般知識で **未確認**）。確認
- SmartScreen: 署名しても評判は「ファイルのハッシュの配布の履歴」で自動で付く。出続けるときは Microsoft Security Intelligence に出す（FAQ）。確認。EV でないので、最初の数日〜数週間は警告が出る可能性が高い（推測）

#### SignPath Foundation（OSS 向けの無料の署名）

`signpath.org/terms` を読んだ。確認:
- 条件: マルウェアなし。**全部の部品が OSI 承認のライセンスで、商用の二重ライセンスなし**。自分のプロジェクトで、自分のソースからビルドした実行ファイルだけ。すでに署名したい形でリリースされ、保守されていること。機能が配布ページに書かれていること
- 運用の条件: コードの署名の方針（Code signing policy）をホームと配布ページに載せる。役割（作者、レビューする人、承認する人）を決める。全員が多要素認証。製品名と版のメタデータを署名する実行ファイルに入れる（上の 1.2 の VERSIONINFO が要る）。ビルドが検証できること、リリースの承認を手でする。インストール手順があるならアンインストールも用意する
- **1 人で出せるか**は、ページに書いていない（レビューと承認の役割を分けるので、1 人だと難しい見込み。SignPath に問い合わせが要る）。未確認
- 署名は SignPath のクラウドで行い、GitHub Actions のコネクタがある（一般知識、未確認）。証明書の発行先は「SignPath Foundation」になる（推測）
- MATSU には Onigmo、mruby、MicroPython、QuickJS、Lua などの同梱コードがある（全部 OSS。`THIRD_PARTY_NOTICES.txt`）。ライセンスが OSI 承認かの確認は、申し込みのときに要る（FreeType の FTL は OSI 承認。ADR の範囲外）

#### OV / EV 証明書（従来の CA）

公式の価格表は読んでいない。**金額・本人確認の細かい条件は未確認**。分かっていること（一般知識、推測）: 2023-06 から、コード署名証明書の秘密鍵は FIPS 140-2 Level 2 以上のハードウェア（USB トークンか、クラウドの HSM）に置く決まり（CA/Browser Forum）。EV は SmartScreen の評判が最初から付くとされていたが、2024 年ごろから EV でも即時とは限らない。クラウドの HSM 式（CA が持つ鍵に API で署名）なら GitHub Actions で使えるが、CA ごとに有料で、鍵が自分の手元に無い。個人でも OV を出す CA はあるが、法人より審査が重い（推測）

#### Windows の署名の要るところ（Store 以外）

- winget は署名を必須にしていないが、「インストーラが警告なしに入ること」を検査し、Defender とほかの AV で動的に検査する（5.1）。署名が無いと SmartScreen の警告で、手動の審査になる可能性（推測）
- Microsoft Store の MSI/EXE 提出（方針 10.2.9）は **署名が必須**（8 参照）。MSIX は Store が署名する（5.3）

### 2.2 macOS

- Apple Developer Program: 年 **99 ドル**（`developer.apple.com/programs/enroll/`、確認）。ページは円の額を書いていない（現地通貨で登録時に表示）。円の額は未確認。個人は、二要素認証付きの Apple Account、法的な成年、法的な名前、メール・電話・住所（私書箱は不可）が要る。個人で入ると Store の販売者名は **本名**（同ページ。確認）。組織で入る場合は D-U-N-S 番号などが要る（一般知識、未確認）
- 費用の免除は、非営利団体・教育機関・政府のみ（同ページ）。個人の OSS には無い
- Developer ID Application 証明書で署名 + ハードンドランタイム + タイムスタンプ + `notarytool` で公証 + `stapler`（`developer.apple.com/documentation/security/notarizing-macos-software-before-distribution` はタイトルしか取れず、手順は一般知識。**要約は推測**）。App Store Connect の API キー（`.p8`）か、アプリ用パスワードで `notarytool` に認証する
- 公証の追加費用は無い（プログラム費に含む。推測）
- Linux の実行機から署名・公証できる道具: `apple-codesign`（`rcodesign`。`github.com/indygreg/apple-platform-rs`）。README は「Linux・Windows・macOS で動く」「Rust だけで、Apple 製の物が要らない」と書く。ライセンスはページから読めなかった（**未確認**。MPL-2.0 と記憶するが推測）。`notary-submit` の手順は README では確認できず **未確認**
- 秘密の置き場所: Developer ID の秘密鍵（`.p12`）と API キーは、GitHub の Actions の secrets（できれば Environment を作り、必須レビュアー付きにして、タグのリリースのジョブだけが読めるようにする。一般知識）

### 2.3 鍵の置き場所と、VM の前提・実行機の切り替え

ユーザーの前提（ADR-0027 の背景）: 「VMを使うのでローカルPCの情報がそのまま流出しないことが前提です。旅行のときなどに通常CIに戻したり、帰ってきたらSelfHostedRunnerにしたり切り替えやすいことも希望します」。

| 置き場所 | VM との関係 | 切り替えとの関係 | 評価 |
|---|---|---|---|
| GitHub Actions の secrets / Environment | VM のジョブには渡さない（今も渡していない）。署名のジョブは GitHub の Linux / Windows / macOS の実行機に置く | 実行機を切り替えても署名のジョブは動かない。前提どおり | 推奨 |
| クラウドの署名サービス（Artifact Signing、SignPath） | 鍵は渡されない。認証の資格（OIDC なら固定の秘密は無い）だけ | 同上 | 良い。ただし日本の個人には Artifact Signing が使えない |
| ユーザーの Mac のキーチェーン（Developer ID） | VM は Mac の鍵を見られない（前提）。ホストの scripts が署名するなら、VM の成果物をホストに取り込む必要がある | 旅行中は署名できない。前提の「切り替えやすさ」に反する | 避ける |
| VM に鍵を入れる | 前提に反する | — | 却下 |

署名を足すときの形（推奨案）: VM（または GitHub の `macos-15`）は今までどおり署名なし（アドホック）の `.app` を作り、**別の署名のジョブ**（GitHub の実行機、secrets あり、版のリリースのときだけ、Environment の承認付き）が artifact を取って署名・公証する。ただし、VM で作った成果物に本物の署名を付けると、VM の弱点（成果物が書き換えられうる）が署名に広がる。これを避ける選択肢は、版のリリースの macOS だけ GitHub の `macos-15` で作る（VM に移した 2026-11-06 の決めを部分的に戻す。macOS の実行機が足りない日は、v0.7.0 のように、パッケージが載らないことがある）か、VM の成果物を署名の前に再ビルドして照合する（ビット単位で一致しないので難しい。推測）。これはユーザーの判断（ADR-0027 の変更）になる。

オーナーだけができること: Apple Developer Program の登録（本人確認、支払い、規約の同意）、Developer ID 証明書の発行（CSR のアップロード）、API キーの作成、Azure のサブスクリプションと請求先の用意、身分の検証（顔認証など）、SignPath や Chocolatey のアカウントの作成、GitHub の secrets の登録、他のリポジトリへの提出と PR の送信（CLAUDE.md「ほかの OSS への報告」と判断の方針）。

## 3. Windows のインストーラの比較

| | WiX（MSI） | Inno Setup | NSIS | MSIX |
|---|---|---|---|---|
| ライセンス | ソースは MS-RL（弱いコピーレフト。作ったインストーラ自体は MS-RL の対象外）。**v6 から Open Source Maintenance Fee（OSMF）**。確認（下） | 独自の寛容な許諾。「商用も含め、どんな目的にも使ってよい」。ソースの再配布は著作権表示を残す。確認（`jrsoftware.org/files/is/license.txt`） | zlib/libpng（一般知識。ページは取得できず **未確認**） | 形式。作るツール（`makeappx`、Windows SDK）は SDK の許諾 |
| x64 / ARM64 | v3: ARM64 は限定的（推測）。v4 以降は ARM64 を扱う（一般知識）。`cargo-wix` の README: GitHub の Windows 11 ARM64 の実行機に WiX が入っていない。確認 | 6.3 から `x64compatible`（x64 と ARM64 の Windows 11 の両方）と `arm64`（ARM64 だけ）の識別子。確認（要約のみ） | NSIS 3.11（2025-03）に ARM64 のネイティブのスタブがあるかは **確認できず**（SourceForge に対応依頼が開いたまま。要約のみ） | x64・ARM64 のパッケージを別々に作れる（一般知識） |
| 利用者ごと / 機械ごと | どちらも可（`ALLUSERS`、`MSIINSTALLPERUSER`。一般知識） | `PrivilegesRequired=lowest` と `PrivilegesRequiredOverridesAllowed=dialog` で選ばせられる（一般知識。**要確認**） | 可（`RequestExecutionLevel`） | 利用者ごとが基本（インストールは利用者に紐づく） |
| 関連付けと「プログラムから開く」 | `ProgId`、`Extension`、`Verb` の要素 | `[Registry]` に `HKA\Software\Classes\...`（`uninsdeletekey` で消す） | `WriteRegStr` | マニフェストの `uap:FileTypeAssociation`（`windows.fileTypeAssociation`）。関連付けを決めるのは利用者（Windows 10/11 の仕様）。確認は未 |
| 更新・アンインストール | 上書きの更新（メジャーアップグレード）が標準。アンインストールは、入れたファイルだけ。変更の追跡が堅い | 上書きで更新。`unins000.exe`。入れたファイルだけ消す | 自分でアンインストーラを書く。消し残しは自分の責任 | 自動更新、削除は全部。**AppData の新しいファイルは、パッケージごとの私設の置き場に書かれ、アンインストールで消える**（下） |
| 静かなインストール（winget・Store 用） | `msiexec /qn` | `/VERYSILENT /SUPPRESSMSGBOXES`（`/CURRENTUSER`、`/ALLUSERS`） | `/S` | `Add-AppxPackage`、`winget` |
| GitHub の Windows 実行機での CI | `cargo-wix`（MIT OR Apache-2.0。確認）の既定は WiX v3.14.1（旧）、`Modern` で v4 以降。windows-2025 には v3 が入っているか **未確認**。ARM64 の実行機には無い（確認） | Inno は windows のイメージに入っている（推測。**未確認**）。入っていなくても `choco install innosetup` かインストーラの取得（ハッシュの確認付き）で入る。ARM64 の実行機でも `ISCC.exe` は x86/x64 のエミュレーションで動くはず（推測） | choco で入る。ARM64 のスタブは上のとおり未確認 | `makeappx.exe` は Windows SDK に入る（一般知識）。署名は Store が行うので、Store 提出用なら不要（要確認） |

### 3.1 WiX のライセンス（確認。`docs.firegiant.com/wix/osmf/`）

- WiX のソースは MS-RL のまま。**v6 から OSMF が付いた**。OSMF の EULA は v6 が「v1.0」、v7 が `wix7`「v1.1」。EULA の同意の強制は v7 から
- 費用: 「年間の収益が 10,000 ドルを超える組織は、`wixtoolset` の GitHub 組織にスポンサーする」（要約）。10,000 ドル以下の収益は払わない。個人の無収益の OSS が OSMF の対象か、EULA の本文（同意が要る）は **読んでいない**
- つまり WiX v6 以降は「使ってよい」が、EULA に同意する形のツールになる。MIT OR Apache-2.0 の製品を作るツールとしては問題なさそうだが、ライセンスの判断はオーナー（CLAUDE.md）。WiX v3（`cargo-wix` の既定）は OSMF 前の版（推測）だが、WiX 側は「Legacy は開発者がもう支持していない」（`cargo-wix` の README。確認）
- `cargo-wix`: 787 コミット、MIT OR Apache-2.0。Legacy（v3.14.1、candle/light）が既定、Modern（v4+、`wix.exe`）も使える。確認

### 3.2 MSIX と利用者のデータ（確認。`learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization`）

- 既定の MSIX では、`%APPDATA%` に **新しく作ったファイルとフォルダー**は、パッケージごと・利用者ごとの私設の場所に書かれ、実行中はアプリにだけ合成して見える。**アンインストールで消える**。ほかのアプリからは見えない。既存のファイルの変更は本物に書く（1809 より新しい Windows）
- 外す方法: `desktop6:FileSystemWriteVirtualization = disabled`（制限付きの機能 `unvirtualizedResources` が要る）、または Windows 11 以降は `virtualization:ExcludedDirectories` で `%APPDATA%\matsu-editor` だけ除く（`$(KnownFolder:RoamingAppData)\...`）
- MATSU は `%APPDATA%\matsu-editor` に `matsu.ini` や `unsaved/` を新規に作る。そのまま MSIX にすると **アンインストールで設定と未保存の文書が消え、ほかのエディタから `matsu.ini` が見えない**。利用者のデータの決めに反する。だから MSIX は、仮想化を外す宣言（と、Store の審査での説明）が必須
- `unvirtualizedResources`（制限付きの機能）は Store の提出で審査される（一般知識、**個人が通るかは未確認**）

### 3.3 比較のまとめ（推測を含む）

- 最初の 1 つは Inno Setup が軽い: 許諾が寛容、利用者ごとに入れられる、ARM64 と x64 の両方、静かなインストールの引数が決まっている、Rust のビルドの外で完結。弱み: MSI でないので、企業の配布（GPO、Intune）には使いにくい（Intune は `.intunewin` で包めば EXE も可。一般知識）
- MSI（WiX）は、企業の配布や winget の `msi` が要るときに足す。OSMF の EULA の判断が要る
- NSIS は ARM64 の不確かさと、アンインストールを自作する負担で後回し
- MSIX は Store の道（Store が署名するので証明書が要らない）と、Windows の「クリーンなアンインストール」の道。ただし 3.2 のデータの問題がある

## 4. パッケージマネージャ

### 4.1 winget（確認。`learn.microsoft.com/en-us/windows/package-manager/package/repository`）

- 置き場: `microsoft/winget-pkgs`（GitHub）。PR で `manifests/<頭文字>/<発行元>/<アプリ>/<版>/` に YAML を足す。1 回の PR は 1 パッケージの 1 版だけ
- 検査: 自動で、マニフェストの検証、URL の検証、インストーラを複数の AV と Defender で動的に検査、インストールとアンインストールが管理者・一般利用者の両方で動くこと、非対話のインストール、**インストーラの URL は発行元の直リンク（リダイレクト不可、HTTPS）**。最後に、人（moderator）が見て通す。「Microsoft は理由なく拒否できる」
- 形式: `msi`、`exe`（Inno・NSIS・Burn などを自動で解析）、`msix`、`zip`（中に `portable` や installer を入れる）、`portable`。zip の `NestedInstallerType: portable` は一般知識で、**今回の zip（フォルダー 1 つに exe 2 つ）がこの形で通るかは未確認**
- 署名: 必須とはページに書いていない（確認）。SmartScreen の評判の検査は「URL の評判」として入る（`URL-Validation-Error`）
- 自動化:
  - `wingetcreate`（`microsoft/winget-create`、**MIT**。確認）: `wingetcreate update <PackageId> -u <URL> -v <版> -t <token> --submit`。トークンは GitHub の PAT（`WINGET_CREATE_GITHUB_TOKEN`。README は `repo` 権限と書く）。確認
  - `komac`（`russellbanks/Komac`、**GPL-3.0**。確認）: README では、トークンは **classic の PAT（`public_repo`）で、fine-grained は PR を出せない**。GPL-3.0 の道具を CI で実行するだけで、製品に入れない。ただし CLAUDE.md の「GPL 系は使わない」は依存の話で、CI の道具は想定外 → ライセンスの判断はオーナー。推奨は、MIT の `wingetcreate`
  - 公式の「WinGet Releaser」などの GitHub Action もあるが、読んでいない（未確認）
- 最初の登録は PR（人が見る）。2 回目以降も、PR を出し、自動検証 + 承認（大抵は自動で通る。一般知識）
- **提出は他人のリポジトリへの PR。オーナーの判断と操作**（AI は出さない）

### 4.2 Scoop

- マニフェスト（JSON）を、バケットに置く。自分のバケット（`scoop bucket add matsu https://github.com/<オーナー>/scoop-matsu`）か、公式の Main（コマンドラインのツール向けで、GUI アプリは Extras に行く。一般知識。`Criteria for including apps in the main bucket` のページは取得できず **未確認**）
- 自分のバケットなら、提出は要らない。`checkver` と `autoupdate` を書けば、バケットの CI（Excavator のテンプレート）が新しい版を見つけて直す（一般知識、未確認）
- 配る zip の構造（`matsu-windows-x64/` の下に exe）は `extract_dir` で扱える。`bin`、`shortcuts` で PATH とスタートメニュー（一般知識）
- Extras への追加は、Extras の PR（人が見る）。オーナーの操作

### 4.3 Chocolatey

- コミュニティのリポジトリ（`community.chocolatey.org`）。パッケージを作って `choco push`（API キーが要る。アカウント登録はオーナー）。自動の検査（ウイルス検査、検証）と、人のモデレーターの審査がある（一般知識）。`docs.chocolatey.org/docs/community-packages-disclaimer` は、リダイレクトで中身を取得できず **未確認**
- 手間が大きく、利用者数は winget より少ない。優先度は低い（推測）

### 4.4 Homebrew

- 公式 `homebrew/cask` の受け入れ（確認。`docs.brew.sh/Acceptable-Casks`、`Package-Acceptance-Policy`）:
  - 知名度: GitHub のリポジトリが **30 フォーク・30 ウォッチャー・75 スター**のどれか以上。**自分で出す場合（提出者がリポジトリの所有者）は 90 / 90 / 225**。作って 30 日未満は通常不可。ソースが公開リポジトリでないものは、知名度の証拠を PR に書く
  - 配布: 開発者が公開している、または開発者が認めたダウンロードを使う
  - Gatekeeper: 「Homebrew の Gatekeeper の検査に通ること」。Acceptable Casks に署名・公証の語はない。Homebrew 5.0.0 のリリース記事（`brew.sh/2025/11/12/homebrew-5.0.0/`。確認）: 「コード署名のない cask は非推奨」「**2026 年 9 月に、Gatekeeper の検査に落ちる `homebrew/homebrew-cask` の cask を全部無効にする**」。`--no-quarantine` は非推奨。公証の語は記事にない。9 月の後に実際どうなったかは、公式の情報を見つけられず **未確認**（今日は 2026-11-13）
  - つまり、アドホック署名だけの MATSU は、公式 cask に **入れられない**見込み。さらに自分で出すので 225 スターが要る。今のスター数は、GitHub に問い合わせない決まり（アカウントの停止があった。memory）のため **未確認**
- 自分の tap（`<オーナー>/homebrew-matsu`。`brew install --cask <オーナー>/matsu/matsu-editor`）:
  - 受け入れの規則は無い。`homebrew/cask` の知名度と Gatekeeper の規則は適用されない（推測。ページは「自分の tap」を書いていない）
  - 署名なしだと、利用者の Mac で Gatekeeper の警告が出る。`--no-quarantine` は非推奨（上）なので、cask の `caveats` に `xattr -dr com.apple.quarantine` を書く（Cask Cookbook の例にも「未署名のとき `caveats`」とある。確認）
  - cask の書き方（確認 + 推測）: `app "matsu-macos-arm64/MATSU Editor.app"`（tar.gz の中のフォルダーの中にある .app を指す。**推測**）、`binary "#{appdir}/MATSU Editor.app/Contents/MacOS/matsu", target: "matsu"`、`depends_on arch: :arm64`、`zap trash: ["~/Library/Application Support/matsu-editor"]`（`zap` は `brew uninstall --zap` のときだけ動く。**ユーザーのデータなので、書くか書かないかはオーナーが決める**。推奨は、書かない）
  - 更新: tap のリポジトリで、リリースのたびに版と sha256 を書き換える PR / push。自動化は、`livecheck` + `brew bump-cask-pr`（公式 cask 向け）か、自前のスクリプト
  - 提出は不要（自分のリポジトリ）。リポジトリの作成と、書き込みのトークンはオーナー

### 4.5 Linux

| | 形 | GPUI（Vulkan、Wayland・X11）との相性 | 提出 |
|---|---|---|---|
| `.deb` | `cargo-deb`（MIT。一般知識、**ライセンスは未確認**）。`Depends` に共有ライブラリ（libfreetype6、libxkbcommon、libvulkan1 または mesa-vulkan-drivers の「推奨」など）。`.desktop`、アイコン、`MimeType` | 良い。OS の Vulkan と Wayland・X11 をそのまま使う | 自分のリポジトリで配る（GitHub Release に添付）。公式 Debian・Ubuntu への提出は大きな作業（対象外） |
| `.rpm` | `cargo-generate-rpm`（MIT。一般知識、**未確認**）。Fedora・openSUSE | 同上 | 同上 |
| AppImage | `appimagetool`（MIT。一般知識）。FUSE が要る（Ubuntu 22.04 以降は FUSE 2 が既定で入らない。一般知識）。Vulkan のドライバーは AppImage に入れられず、ホストのを使う | 動くが、依存の取りこぼしと、`libfreetype`、`libvulkan` はホストのものを使う作りが難しい（推測） | 提出なし（Release に添付）。AppImageHub への登録は任意 |
| Flatpak | `org.freedesktop.Platform`（Mesa の Vulkan と GL を含む。一般知識）に載るので、GPUI の描画に向く（推測）。Flathub の提出は **ID `io.github.stofu5678.matsu-editor`**（`io.github.<利用者>.<名前>`、4 つ以上の部分、ハイフンは最後の部分だけ。確認）は規則に合う | 端末のタブ・外部コマンド・プラグインは **Flatpak の砂の中で動く**（ホストのシェルは見えない）。ホストを使うには `flatpak-spawn --host`（`org.freedesktop.Flatpak` への talk-name。審査で通りにくい）か、`--filesystem=host` 相当。**砂が MATSU の機能（シェルのタブ、外部コマンド）を損なう**（推測） | Flathub は `flathub/flathub` の `new-pr` ブランチへの PR（人が見る）。**Flathub は AI が作った・AI が補助した内容のマニフェストを禁止している**（確認。`docs.flathub.org/docs/for-app-authors/requirements`）。AI が提出の PR を開くことも、コミットメッセージや説明を書くことも禁止。違反が重なると永久追放。ビルドは**ネットワークなし**（cargo の依存を全部ソースとして列挙する必要）。つまりマニフェストは**人が手で書き、人が提出する**。この点で、このプロジェクトの普段のやり方（AI が書く）と合わない |
| Snap | `snapcraft`。strict は砂で、シェルを動かせない。classic は審査が要る（エディタや IDE は classic が通る前例があるが、ページを読めず **未確認**） | 同上 | Snap Store の登録と、classic の申請（人。Canonical のフォーラム） |

- 推奨の優先順（推測）: 1) `.deb` と `.rpm`（と tar.gz を残す）。`.desktop`、アイコン、`MimeType`。2) AppImage は任意。3) Flatpak は、砂の問題と Flathub の AI 規則から、今は見送り。Snap も見送り

## 5. ストア

### 5.1 Microsoft Store

- アカウント: **個人は登録無料**（Windows Developer Blog 2025-09-10 `blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/`、検索の要約のみ。2025-06 から。身分証と自撮りで本人確認、クレジットカード不要）。会社は 1 回 99 ドル。確認は要約のみ
- 提出の形（`learn.microsoft.com/en-us/windows/apps/publish/` 確認）: **MSIX**、**MSI または EXE**（HTTPS の直リンクを出す）、PWA。自動化は Microsoft Store Developer CLI と提出 API（MSI/EXE 向け）、GitHub Actions
- MSIX: Store が署名する（方針 10.2「証明書の取得は Microsoft Store の署名で提供される場合を除く」。確認）。審査 3 営業日まで
- MSI/EXE（方針 10.2.9。確認。`learn.microsoft.com/en-us/windows/apps/publish/store-policies`、版 7.20、2026-09-14）:
  - インストーラは `.msi` か `.exe` だけ
  - **すべての PE ファイルが、Microsoft Trusted Root Program の CA につながる証明書で署名されていること**（必須）
  - **版ごとの URL**を出す。URL の中身は提出後に変えてはならない。更新ごとに新しい URL を出す（更新はこちらの責任）
  - 静かなインストールが必須（UAC の画面は可）。オフラインのインストーラで、ダウンローダーは不可。PC だけ
  - 審査の検査: マルウェア、静かなインストール、一般の利用者でのインストール、スタートメニューへの登録、追加と削除に製品名・発行元・版、**きれいなアンインストール**（ファイル・フォルダー・レジストリの消し残しなし）、抱き合わせソフトなし、非 Microsoft のドライバー・NT サービスへの依存の申告（`learn.microsoft.com/.../msi/app-certification-process`）
  - **つまり MSI/EXE の道は署名が必須**。日本の個人は Artifact Signing が使えないので、SignPath（1 人で通るか未確認）か OV 証明書を買う必要がある。MSIX の道は証明書が要らない
  - 「きれいなアンインストール」の検査と、利用者のデータを消さない決めの関係: 検査が `%APPDATA%` のユーザーデータを残すことを減点するかは **未確認**（アプリが作った利用者のデータは別、と読むのが普通。推測）
- プライバシーポリシー: 個人情報を扱う、または法律が要るとき、URL を Partner Center に入れる（方針 10.5。確認）。MATSU は今のところ、ユーザーの文書を外に送らない（企画 4 の「生成 AI に送る」が入ると変わる。ADR-0028 の企画 4）。推奨は、最初から URL を用意する
- 年齢区分: IARC の質問票（方針 11.11。確認）
- 他のプログラムを動かすアプリ・コードを取ってくるアプリの規則（方針 10.2。確認）:
  - 10.2.2: 動的なコードの取り込みで、説明した機能を根本的に変えたり広げたり、方針に違反する機能を入れてはならない。例: 「遠隔のスクリプトをダウンロードして、説明した機能と合わない形で実行する」
  - 10.2.4: 統合されていないソフト（別の製品・モジュール・サービス）に主な機能が依存するなら、説明の冒頭で申告する
  - 10.2.3: 自分が作っていない二次的なソフトのインストールを勧めてはならない
  - シェル・外部コマンド・プラグインを「利用者が自分で選んで動かす」機能として説明に書く限り、禁止の文言には当たらない（推測）。ただし審査で止まるかは、出してみないと分からない（未確認）。「プラグインを追加すると利用者の権限で制限なく動く」（ADR-0026）は、説明に書く
- 通知: MSIX の `runFullTrust`（制限付きの機能）は、Win32 を MSIX に入れるのに必要。申請の審査で説明を求められる（一般知識、未確認）

### 5.2 Mac App Store

- 費用: Apple Developer Program 年 99 ドル（2.2）。さらに App Store 用の証明書（Apple Distribution）、プロビジョニング、App Store Connect、審査
- 規則（確認。`developer.apple.com/app-store/review/guidelines/`）:
  - 2.4.5(i): 「適切にサンドボックス化されていること」。(ii) Xcode の技術で作る、サードパーティのインストーラは不可、**自己完結の 1 つのバンドルで、共有の場所にコードや資源を入れない**。(iv) 「標準のアプリ、kext、**追加のコード、資源をダウンロードまたはインストールして、機能を足したり、審査で見たものから大きく変えたりしてはならない**」。(vii) 更新は Mac App Store だけ
  - 2.5.2: 「バンドルの中で自己完結。**指定のコンテナの外にデータを読み書きしてはならない**。**アプリの機能を導入または変更するコードを、ダウンロード・インストール・実行してはならない（他のアプリを含む）**」。例外は、学生が実行コードを試すための教育用アプリが、限られた場合に、そのコードを他に使わず、**ソースが全部見えて編集できる**ときだけダウンロードできる
  - 4.7: 「バイナリに入っていないソフトを提供してよいのは、HTML5 と JavaScript のミニアプリ、ミニゲーム、ストリーミングのゲーム、チャットボット、**プラグイン**（と、レトロゲームのエミュレーター）」。条件は 4.7.1〜4.7.5（読んでいない）。なお 4.7 の「plug-ins」が、MATSU のような、ネイティブなコードを動かす拡張に及ぶかは **未確認**
  - 5.1.1(i): プライバシーポリシーの URL を App Store Connect とアプリの中に
- App Sandbox（要約は Apple のページが取得できず、**一般知識。未確認**）: 既定でコンテナだけ。ユーザーが選んだファイルは entitlement と開く・保存するパネル、**security-scoped bookmark** で。**子プロセスは、同じ砂（`com.apple.security.inherit`）で動くか、自分の砂で動く。任意の実行ファイルやシェルを自由には動かせない**。ネットワークは `network.client`、`network.server`
- MATSU の機能との衝突（推測を多く含む。実機で確かめていない）:

| MATSU の機能 | 砂での扱い | 変えること |
|---|---|---|
| シェルのタブ（ADR-0028。`alacritty_terminal` で PTY を作り、`login` 経由でシェルを起動） | `forkpty` で起こす子は砂の中の新しい砂になり、ホストのシェルや PATH のツールを使えない。Apple の開発者フォーラムの助言は「Mac App Store の外で配る」か「端末の動きを自分のアプリに作り込む」（検索の要約のみ） | タブを外す |
| 外部コマンドの実行（`F_EXECMD`、ADR の「外部コマンド」）、`#!` スクリプト | 同上。BBEdit は砂で `#!` を持つ実行権限のファイルを編集するとき、追加の制限がかかると説明（`barebones.com/support/bbedit/AppSandboxing.html`。確認） | 外す。または、パイプ先を「アプリに同梱した、署名された実行ファイルだけ」に限る |
| プラグイン（ADR-0026。`plugins\` の中の利用者のコードを、利用者の権限で動かす） | 2.5.2 の「機能を導入または変更するコードの実行」に当たりうる（ダウンロードでなくても、利用者が置いたコードを実行する）。4.7 の「plug-ins」はミニアプリの意味（推測）。**審査に通らないか、制限が大きい** | 外す |
| マクロの外部の言語（Node.js・Python・Ruby・PHP・PowerShell・Perl・Emacs Lisp・AppleScript。ADR-0007・0025） | 利用者の PATH のインタープリターを動かせない | 外す |
| `matsu-script`（同梱の Lua・QuickJS・MicroPython・mruby。利用者が書いたマクロを動かす） | 同梱の処理系で、バンドルの中。2.5.2 の「機能を変更するコード」に当たるかは **未確認**（前例: JavaScript の Safari 拡張や、スクリプトを持つエディター。BBEdit は「Mac App Store 版は機能が同じ」と書く。要約のみ。仕組みは未確認）。子プロセスとして起動するなら `inherit` の署名が要る | 残せる可能性。審査の説明を要する |
| データベース（ADR-0023） | `network.client` があれば TCP の接続は可（推測）。SQLite などファイルの DB は、利用者が選んだファイルだけ | ファイル DB は選択を要する |
| 設定の置き場所（`~/Library/Application Support/matsu-editor/matsu.ini`） | 砂のアプリの `Application Support` は `~/Library/Containers/<ID>/Data/Library/Application Support` になる。**既存の利用者の設定は見えない。別のエディターから `matsu.ini` を直せない。利用者のデータの置き場所が変わる**（推測） | 設定の場所の決めを変える（ユーザーの判断） |
| ファイルを開く | 利用者が選んだ、または Finder から渡されたファイルだけ。同じフォルダーの別ファイル（インクルード、Git、`.editorconfig`）は、追加の許可（BBEdit は「起動ディスクの最上位へのアクセスを許可」を 1 回利用者に求める。確認） | 許可の画面を作る |
| 巨大ファイル、ファイルロック、印刷、IME、AX | 大きな問題はない（推測） | — |

- 前例（確認。いずれも、検索の要約か、公式ページ）:
  - **Termius**: 「Mac App Store と Snap Store の Termius は、砂の制限のため Local Terminal の機能を持たない」（`docs.termius.com/organize-and-connect-to-hosts/connecting-to-a-server`。確認）。Web 版の配布を勧める
  - **Panic の Prompt 3**: Mac App Store の版は砂のため、ローカルの端末を使うには「砂の外で動く helper app」を別に入れる（`help.panic.com/prompt/prompt-local-terminal/`。確認）。つまり Store の外の部品を足す
  - **rootshell**: Mac App Store 版は、ローカルのシェルに別の helper をサイトから入れるか、SSH で localhost に入る（検索の要約のみ）
  - **BBEdit**: 2014 に Store から引き上げ、2019（12.6）に砂に対応して戻った。「Mac App Store の版は、サイトの版と機能が同じ」（検索の要約のみ。`barebones.com/products/bbedit/appstore/faq.html`。一次ページは読んでいない）。シェルのワークシートや Unix フィルターの扱いは要約でも **未確認**
  - 端末の機能を持つエディターで、Store 版が端末を持つ例は、見つからなかった（未確認）
- 「Store 専用の機能を減らした版」は現実的か（推測）: 技術的には Cargo の feature（例 `store`）で、端末のタブ、外部コマンド、プラグインのホスト、外部の言語を外し、同梱の処理系だけを残す形で作れる。ただし (a) ADR-0028 以降の企画（端末、ワンライナー、生成 AI の CLI）の中心が丸ごと Store 版に無くなる、(b) 設定の置き場所が砂のコンテナになる、(c) 審査で `matsu-script` と同梱の処理系が 2.5.2 に当たるか不明、(d) 2 つの版を保つ負担、(e) Mac の Store は審査と年 99 ドルが要る。利用者の得は「Store で入れられる」だけ。**今は見送り、Windows の Store と署名の経験の後に再検討**が妥当（推測）

## 6. リリースの自動化

現在（確認）: `release.yml` の `release-please` が版を決めて PR を出し、PR のマージで `release_created` が立つ。`package-release`（`package.yml`）が 5 つの対象を作り、`release` ジョブ（GitHub の Linux 実行機、`contents: write`、`GITHUB_TOKEN`）が `matsu-${TAG}-<対象>` の名前で Release に添付する。`GITHUB_TOKEN` で作ったリリース・タグは別のワークフローの起動の元にならない（GitHub の仕様。一般知識）ので、インストーラの作成と配布は、`release.yml` の中の `needs: release` の後のジョブとして足す必要がある。

足す形（案）:
1. `package.yml` のあとか中に、対象ごとのインストーラ作成の手順を足す: Windows は `iscc`（Inno Setup）で `matsu-<版>-windows-<arch>-setup.exe`。Linux は `cargo deb` と `cargo generate-rpm`。macOS は、今の `.tar.gz` に加えて `.dmg`（`hdiutil`。署名がないうちは任意）
2. `package-check.yml` の `paths` に、インストーラの定義ファイルを足す（PR で壊れていないことを確かめる）
3. インストール・アンインストールの試験を Windows の実行機（x64、ARM64）で CI にする: 静かにインストール → ファイルがある → `%APPDATA%\matsu-editor` にダミーの `matsu.ini` を置く → アンインストール → ダミーが残る
4. 署名（段階 B 以降）: `release` の前に署名のジョブ（GitHub の実行機、secrets、Environment）
5. 各配布先の更新（段階 B 以降）: `release` の後に、
   - winget: GitHub の Windows か Linux の実行機で `wingetcreate update ... --submit`。必要なトークン: オーナーの classic PAT か `repo` 権限の PAT を Actions の secret に（`microsoft/winget-pkgs` へのフォークと PR を作る）。fine-grained は不可（`komac` の README。`wingetcreate` は未確認）
   - Homebrew の tap と Scoop のバケット: 別のリポジトリへの push が要るので、`GITHUB_TOKEN` では足りない。fine-grained の PAT（そのリポジトリだけ、`Contents: write`）か、GitHub App のトークン。Scoop は `autoupdate` + バケット側の CI で、こちらから push しなくてよい（未確認）
   - `release.yml` の `HAS_TOKEN` の仕組み（`RELEASE_PLEASE_TOKEN`）が前例。トークンが無いときは、その手順を飛ばす書き方にできる
6. 手でやること（残る）: アカウントの登録・本人確認・支払い、証明書の発行と更新（Developer ID は 5 年？、Apple は更新を要する。要確認）、Store の提出（初回の登録、説明文・スクリーンショット・年齢区分・プライバシーポリシー）、winget の最初の PR、他のリポジトリへの提出、トークンの更新（PAT は期限あり）、署名の失敗時の調査

## 7. 参考のリンク

- Artifact Signing: https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart、`.../faq`、`.../how-to-signing-integrations`
- SignPath Foundation: https://signpath.org/terms
- Apple: https://developer.apple.com/programs/enroll/、https://developer.apple.com/app-store/review/guidelines/
- Homebrew: https://docs.brew.sh/Acceptable-Casks、https://docs.brew.sh/Package-Acceptance-Policy、https://docs.brew.sh/Cask-Cookbook、https://brew.sh/2025/11/12/homebrew-5.0.0/
- winget: https://learn.microsoft.com/en-us/windows/package-manager/package/repository、https://github.com/microsoft/winget-create（MIT）、https://github.com/russellbanks/Komac（GPL-3.0）
- Microsoft Store: https://learn.microsoft.com/en-us/windows/apps/publish/store-policies、`.../publish-your-app/msi/app-certification-process`、https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/
- MSIX の仮想化: https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization
- WiX の OSMF: https://docs.firegiant.com/wix/osmf/、cargo-wix: https://github.com/volks73/cargo-wix
- Inno Setup の許諾: https://jrsoftware.org/files/is/license.txt、6.3 の変更点: https://jrsoftware.org/files/is6.3-whatsnew.htm（要約のみ）
- Flathub: https://docs.flathub.org/docs/for-app-authors/requirements、`.../submission`
- Termius: https://docs.termius.com/organize-and-connect-to-hosts/connecting-to-a-server、Prompt: https://help.panic.com/prompt/prompt-local-terminal/、BBEdit: https://barebones.com/support/bbedit/AppSandboxing.html
- リポジトリ内: `.github/workflows/package.yml`、`release.yml`、`scripts/package-mac-app.sh`、`crates/matsu-config/src/location.rs`、`docs/release.md`、`docs/adr/0014`、`0022`、`0026`、`0027`、`0028`

## 8. 取れなかった・確かめていないこと

- Artifact Signing の現在の月額（ページに金額なし）
- OV・EV 証明書の価格と本人確認の条件
- オーナーの居住国と、個人か事業者か（Artifact Signing の可否が分かれる）
- SignPath Foundation に 1 人で申し込めるか
- Snap の classic の審査の条件、Chocolatey の審査の細かい条件、Scoop の Main と Extras の基準（ページが取得できなかった）
- Apple Developer Program の円の額
- Homebrew の公式 cask の 2026-09 以降の実際の扱い
- リポジトリの GitHub のスター数（GitHub を呼ばない決まりのため）
- 実行時の共有ライブラリの一覧（`ldd`）、Vulkan が無い環境の動き
- 今回の zip 構造が winget の `portable`（入れ子）で通るか
- MSIX の `unvirtualizedResources` を個人の Store アカウントが通せるか
- Mac App Store の 4.7.1〜4.7.5 の中身、App Sandbox の公式ページ（取得できず一般知識で補った）
