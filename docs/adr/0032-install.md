# ADR-0032: インストーラとパッケージ、ストアでの公開

- 状態: 提案
- 日付: 2026-11-13（ユーザーが 2026-11-11 に、企画の 6 番目として出した）
- 関連: [ADR-0014](0014-development-process.md)（署名は最初の設定だけ人が行う）、[ADR-0027](0027-self-hosted-mac-runner.md)（Mac の VM の実行機）、[ADR-0028](0028-terminal-tab.md)（シェルのタブ）、[ADR-0026](0026-resident-plugins.md)（プラグイン）、[ADR-0023](0023-database.md)、[ADR-0022](0022-autosave-unsaved.md)、[ADR-0018](0018-decision-policy.md)、[ADR-0016](0016-single-instance.md)、[ロードマップの Phase 9](../roadmap.md)
- 下調べの詳細（出典、確かめていないこと）: [docs/poc/install.md](../poc/install.md)

## 背景

- ユーザーの企画（2026-11-11）の 6 番目。「インストーラ」として次の 3 つ
  - Windows のようなインストーラ
  - インストールパッケージと、brew cask のような CLI でのインストール
  - Windows と Apple のストアでの公開の企画まで
- 今の配布物は、5 つのアーカイブ（Windows の zip 2 つ、macOS の tar.gz、Linux の tar.gz 2 つ）だけ。Windows と Linux には、スタートメニューも、ファイルの関連付けも、アンインストールも、`.desktop` もない。macOS は `.app` で、`CFBundleDocumentTypes` はあるが、署名はアドホックで、公証はない（`docs/release.md`「署名」、`scripts/package-mac-app.sh`）
- 署名と公証は、ADR-0014 で「最初の設定だけ人が行い、あとは CI で自動」と決めてある。まだ人の設定がない
- macOS のパッケージは、ADR-0027 の追記（2026-11-06）で、ユーザーの Mac の上の使い捨て VM で作る。VM には secrets がなく、リリースの公開は GitHub の Linux の実行機で行う。ユーザーの前提は「VM を使うのでローカル PC の情報がそのまま流出しないこと。旅行のときなどに通常の CI に戻したり、帰ってきたら自前の実行機にしたり、切り替えやすいこと」
- MATSU は他のプログラムを動かす: シェルのタブ（ADR-0028）、外部コマンド、プラグインと外部の言語（ADR-0026・0025）、データベース（ADR-0023）。ストアの砂（サンドボックス）の規則と、ぶつかる

## 調べた結果

詳しくは下調べのファイル。ここには、案の根拠になるものだけを書く。

### 今の作りで、インストーラが守ること（リポジトリを読んだ）

- 設定は、Windows は `<exe のフォルダー>\matsu.ini`（あれば。持ち運び用）、なければ `%APPDATA%\matsu-editor\matsu.ini`。macOS は `~/Library/Application Support/matsu-editor/`、Linux は `$XDG_CONFIG_HOME/matsu-editor/`（なければ `~/.config/matsu-editor/`）。同じフォルダーに `unsaved/`、`plugins.ini`、`databases.ini`、履歴、`plugins\`（`crates/matsu-config/src/location.rs`、ADR-0022、ADR-0026）
- インストール先に `matsu.ini` を作ると、持ち運び用の動きに固定される。だから、インストーラはインストール先に `matsu.ini` を作らない
- `matsu` は、隣の `matsu-script` を使う。2 つを同じフォルダーに置く（`crates/matsu-plugin/src/session.rs`）
- Windows の `matsu.exe` は、アイコンだけを埋めてあり、版情報（製品名・発行元・版）がない（`crates/matsu/matsu.rc`）
- Windows と Linux には、ファイルの関連付けの登録がない。コマンドラインは `matsu file...` を受ける。自動更新の仕組みはない
- Linux は OS の共有ライブラリ（FreeType、Wayland、X11、xkbcommon ほか）にリンクし、描画は Vulkan。実行に要るライブラリの一覧は、まだ確かめていない

### Windows のインストーラ

| | WiX（MSI） | Inno Setup | NSIS | MSIX |
|---|---|---|---|---|
| ライセンス | ソースは MS-RL。v6 から OSMF（年の収益 10,000 ドル超の組織が、`wixtoolset` にスポンサーする。EULA の同意が要る） | 「商用を含め、どんな目的にも使ってよい」 | zlib/libpng（未確認） | 形式。ツールは Windows SDK |
| ARM64 と x64 | v3 は限定的。`cargo-wix` は GitHub の ARM64 の実行機に WiX がないと書く | 6.3 から `x64compatible` と `arm64` | ARM64 のスタブは確認できず | 別々に作れる |
| 利用者ごと / 機械ごと | どちらも | 選ばせられる | どちらも | 利用者ごと |
| アンインストール | 入れたファイルだけ | 入れたファイルだけ | 自作 | **`%APPDATA%` に新しく作ったファイルも消える**（既定。仮想化） |
| 向く場面 | 企業の配布 | 一般の利用者 | — | Microsoft Store |

- MSIX の既定では、`%APPDATA%\matsu-editor` に新しく作る `matsu.ini` と `unsaved/` が、パッケージごとの私設の置き場に書かれ、アンインストールで消え、ほかのアプリから見えない（Microsoft Learn「Flexible virtualization」）。外すには `FileSystemWriteVirtualization` を無効にする（制限付きの機能 `unvirtualizedResources`）か、Windows 11 以降は `ExcludedDirectories` で `%APPDATA%\matsu-editor` だけを除く

### 署名

- Windows の Azure の Artifact Signing（旧 Trusted Signing）: **個人は米国かカナダにいる人だけ。日本は「組織」だけ**（法的な事業体、事業者の識別子、事業体のドメインのメールが要る）。有料の Azure サブスクリプションが要る。EV は出さない。現在の月額は、公式ページに金額がなく未確認
- SignPath Foundation（OSS 向けの無料）: 全部の部品が OSI 承認のライセンスで、レビューする人と承認する人の役割を分け、全員が多要素認証、署名の方針のページを公開する。1 人で出せるかは、ページに書いていない
- OV・EV 証明書: 価格と条件は未確認。秘密鍵はハードウェアかクラウドの HSM に置く決まり（一般知識）
- macOS: Apple Developer Program が年 99 ドル（個人は本名で登録）。Developer ID で署名し、ハードンドランタイムを付け、`notarytool` で公証する
- 鍵の置き場所は、GitHub の secrets（Environment を作り、版のリリースだけが読める形）か、クラウドの署名サービス。**Mac のキーチェーンと VM には置かない**（ADR-0027 の前提）。署名のジョブは、GitHub の実行機に置く。そうすれば実行機の切り替えと関係しない

### パッケージマネージャ

- winget: `microsoft/winget-pkgs` に PR。自動の検査（複数の AV での動的な検査、URL は発行元の直リンク、静かなインストール、アンインストール）と、人の確認がある。署名は必須と書かれていない。自動化は `wingetcreate`（MIT）と `komac`（GPL-3.0。classic の PAT が要る）
- Scoop: 自分のバケットなら提出は要らない。公式のバケットへの追加は PR
- Chocolatey: アカウントと `choco push`、人のモデレーター。手間が大きい
- Homebrew: 公式の `homebrew/cask` は、**GitHub のスターが 75 以上（自分で出すなら 225 以上）**、**Gatekeeper の検査に通ること**。Homebrew 5.0.0 は「署名のない cask は非推奨、2026 年 9 月に Gatekeeper に落ちる cask を無効にする」と告げた。今のアドホック署名だけの MATSU は、公式に入れられない見込み。自分の tap なら、受け入れの規則がない
- Linux: `.deb`・`.rpm` は GPUI の Vulkan と Wayland・X11 をそのまま使え、提出も要らない。AppImage は Vulkan のドライバーを持ち込めない。Flatpak は砂が端末・外部コマンドを損なう。**Flathub は AI が作った・補助したマニフェストを禁じ、AI が提出の PR を開くことも禁じる**。Snap は strict がシェルを動かせず、classic は審査が要る

### ストア

- Microsoft Store: 個人は登録無料（2025-09 から。本人確認あり）。提出は MSIX（Store が署名）か、MSI/EXE（**すべての PE に、信頼できる CA の署名が必須**、版ごとの URL、静かなインストール、きれいなアンインストール）。プライバシーポリシーの URL と IARC の年齢区分が要る。他のプログラムを動かすことの禁止はない。動的なコードの取り込みで機能を変えること（10.2.2）と、別のソフトへの依存の申告（10.2.4）の規則がある
- Mac App Store: 砂が必須（2.4.5）。追加のコードの実行で機能を変えてはならない（2.4.5(iv)、2.5.2）。子プロセスは砂の中で動き、任意のシェルや PATH のツールを動かせない。前例: Termius の Store 版は端末がない、Panic の Prompt は砂の外の helper を別に入れる。端末を持つ Store 版の前例は見つからなかった。ADR-0028 の端末、外部コマンド、プラグイン、外部の言語は、Store 版には入れられない見込み。設定の置き場所も、砂のコンテナの中に変わる

## 案と推奨

### 1. 段階に分ける

推奨: 次の 3 段階に分け、**段階 A だけをこの ADR で決めた直後に作る**。段階 B と C は、ユーザーの口座・支払い・提出が要るので、ユーザーが決めた順に進める。

- 段階 A（アカウントも費用もなしにできる）: インストーラとパッケージを作り、CI で確かめる。署名はなし。配布は今までどおり GitHub の Release
- 段階 B（ユーザーのアカウントと費用が要る）: 署名と公証、winget・Homebrew の tap・Scoop のバケットへの公開
- 段階 C（ストア）: Microsoft Store。Mac App Store と Flathub は今は見送り

理由: A は、ユーザーの判断をほとんど要らずに、「入れやすさ」の大部分（スタートメニュー、アンインストール、関連付け、CLI の入れ方）を実現できる。署名がなくても動く（SmartScreen と Gatekeeper の警告は残る。今の状態と同じ）。

### 2. Windows のインストーラ（段階 A）

選択肢:
- (a) Inno Setup で `.exe` のインストーラ（x64 と ARM64 の 2 つ）
- (b) WiX で `.msi`
- (c) NSIS
- (d) MSIX だけ

推奨: (a)。MSI は、企業の配布の要望が出てから足す。MSIX は段階 C の Store のために別に作る。

理由: Inno Setup は許諾が寛容（新しい許諾の判断が要らない）、利用者ごと（管理者権限なし）と機械ごとを選ばせられ、x64 と ARM64 の両方を作れ、アンインストーラと静かなインストールの引数が標準である。WiX は v6 以降が OSMF の EULA に同意する形で、ライセンスの判断が増え、`cargo-wix` は ARM64 の実行機に対応していない。NSIS は ARM64 のスタブが確かでなく、アンインストールを自作することになる。

インストーラの決まり（案 7 にも関わる）:
- 既定は利用者ごと（`%LOCALAPPDATA%\Programs\MATSU Editor`）。画面で、機械ごと（`Program Files`）を選べる
- インストールするのは、`matsu.exe`、`matsu-script.exe`、ライセンス 3 点、`icons\`、`README.txt` だけ。**`matsu.ini` も `plugins\` も `unsaved\` も作らない**
- アンインストールは、インストーラが入れたファイルだけを消す。**`%APPDATA%\matsu-editor`、実行ファイルのフォルダーにある利用者の `matsu.ini` と `plugins\`、レジストリの設定（今は無い）は消さない**。利用者のファイルがあってフォルダーが空にならないときは、フォルダーを残す
- 版情報（製品名、発行元、版、著作権）を `matsu.exe` に埋める（`matsu.rc` に VERSIONINFO を足す。追加と削除の表示にも使う）
- スタートメニューの項目、追加と削除の登録、PATH への追加（任意の選択肢。既定はオフ）
- ファイルの関連付け: 選択肢の画面で、`.txt` を MATSU に関連付ける（既定はオフ）。「プログラムから開く」の項目（`Applications\matsu.exe` の登録）は、既定でオン。サクラの関連付け（拡張子の一覧）は、サクラに合わせて別の PR で決める
- 静かなインストール: `/VERYSILENT /SUPPRESSMSGBOXES`、利用者ごとは `/CURRENTUSER`、機械ごとは `/ALLUSERS`。更新は上書き
- 自動更新は作らない（案 12）

### 3. macOS の配布物と署名の置き場所（段階 A は形、段階 B は署名）

推奨（段階 A）: 今の `.tar.gz` を残し、`.dmg`（`hdiutil`、`MATSU Editor.app` と Applications へのリンク）を足す。署名はアドホックのまま。

推奨（段階 B。ユーザーの判断。ADR-0027 の変更を含む）: **署名のジョブを、VM とは別に、GitHub の実行機（secrets あり、Environment で守る）に置く**。鍵は VM にも Mac にも置かない。署名のジョブは、版のリリースのときだけ動く。VM で作った `.app` を署名すると、VM の弱点（成果物が書き換えられうる）が署名に広がるので、次のどちらかにする。
- (a) 版のリリースの macOS だけ、GitHub の `macos-15` で作って署名する。nightly は今までどおり VM でアドホック署名。VM で動かす範囲が小さくなるが、前提（VM 優先、簡単に戻せる）は守れる。macOS の実行機が足りない日にリリースが載らない弱みは、v0.7.0 のときと同じ（流し直す）
- (b) VM で作った成果物を、署名のジョブの GitHub の `macos-15` で再ビルドして照合する。ビットが一致しないとみられ、難しい

理由: ユーザーの前提は、VM で作るのは信頼しすぎないこと、鍵を VM に渡さないこと、切り替えが簡単なこと。(a) は、鍵が GitHub の secrets だけにあるので、実行機を切り替えても動く。

署名の方法: `codesign` + `notarytool`（macOS の実行機）が確実。Linux の実行機で署名できる `apple-codesign`（`rcodesign`）はライセンスと手順を確かめていないので、使うならオーナーの判断。

### 4. Windows の署名（段階 B）

選択肢:
- (a) Azure の Artifact Signing（組織として）
- (b) SignPath Foundation
- (c) OV 証明書（クラウドの HSM 式）
- (d) 署名しない（今のまま）

推奨: 先に (b) を問い合わせる。通らなければ (c) か (d) をユーザーが決める。(a) は、ユーザーが日本の事業者として登録できるときだけ。

理由: Artifact Signing は個人が米国とカナダにいる人だけで、日本は事業者のみ。SignPath は無料だが、レビューと承認の役割と、署名の方針の公開と、MATSU の部品の OSI 承認の確認が要り、1 人で出せるか分からない。OV 証明書は有料で、年の費用が続く。署名がなくても、zip とインストーラは使える（警告が出るだけ）。SmartScreen は署名してもしばらく警告が出うる（評判は配布の履歴で付く）。

### 5. パッケージの公開の順（段階 A と B）

推奨の順:
1. 段階 A: **Homebrew の自分の tap**（`<オーナー>/homebrew-matsu`）の cask を、ファイルとして用意する（リポジトリの作成と公開はオーナー）。署名がないので `caveats` に `xattr -dr com.apple.quarantine` を書く。`binary` で `matsu` を PATH に出す。`zap` は書かない（利用者のデータなので、オーナーが決める）
2. 段階 A: **Scoop の自分のバケット**のマニフェストを用意する（`autoupdate` 付き）
3. 段階 A: **winget のマニフェスト**を `winget validate` が通るところまで作る（提出はしない）。形は、インストーラ（Inno Setup）の `exe` にする（zip の `portable` は、構造が合うか未確認）
4. 段階 B: winget の PR を、オーナーが出す。2 回目以降は、`wingetcreate update --submit` を `release.yml` に足す（オーナーの PAT を secret に）
5. 見送り: 公式の `homebrew/cask`（スター 225 以上と、署名が要る）、Scoop の Extras、Chocolatey

理由: 自分の tap とバケットは、他人の審査がなく、すぐ試せる。winget は利用者が多いが、PR が人の確認を通る。`wingetcreate` は MIT で、`komac`（GPL-3.0）より許諾の判断が軽い。公式の cask は、知名度（スター）と署名の条件を満たせない見込み。

### 6. Linux（段階 A）

推奨: `.deb`（`cargo-deb`）と `.rpm`（`cargo-generate-rpm`）を、x64 と ARM64 で作り、tar.gz は残す。`.desktop`、アイコン（`hicolor`）、`MimeType`（`text/plain`）、AppStream のメタ情報を添える。AppImage は任意（要望が出てから）。Flatpak（Flathub）と Snap は見送り。

理由: `.deb` と `.rpm` は OS の Vulkan・Wayland・X11・FreeType をそのまま使え、今の作りに合う。Flatpak は砂が端末と外部コマンドを損ない、Flathub は AI が作ったマニフェストと AI の提出を禁じている（この規則は、人が手で書いて出すなら守れる）。Snap の strict はシェルを動かせない。

実装の前に、実行に要る共有ライブラリの一覧（`ldd`）と、Vulkan のドライバーがない環境の動きを確かめる。

### 7. インストールとアンインストールが、利用者のデータに触れないこと

推奨（ユーザーのデータの決め。全部のインストーラの共通の規則）:
- インストーラとパッケージは、`matsu.ini`、`unsaved/`、`plugins.ini`、`databases.ini`、`plugins\`、履歴、`%APPDATA%\matsu-editor`、`~/Library/Application Support/matsu-editor`、`~/.config/matsu-editor` を作らず、書き換えず、消さない
- 「設定も消す」という選択肢を、アンインストーラに作らない。必要なら、MATSU の中に「設定を書き出す・消す」を、別の判断で作る
- 実行ファイルのフォルダーの `matsu.ini` と `plugins\`（持ち運び用）は、アンインストールで残す
- CI で確かめる（案 10）

理由: 設定と未保存の文書は、利用者の書いたもの。`unsaved/` には未保存の文書の本文が入る（ADR-0022）。

### 8. Microsoft Store（段階 C）

選択肢:
- (a) MSIX で出す（Store が署名するので証明書が要らない）
- (b) MSI/EXE で出す（署名が必須）
- (c) 出さない

推奨: (a)。ただし、**`%APPDATA%\matsu-editor` の仮想化を外す**（Windows 11 以降は `ExcludedDirectories`、古い Windows は `FileSystemWriteVirtualization` の無効化と `unvirtualizedResources`）。外せない、または審査で通らないなら、(b)（署名は案 4）か (c)。出す前に、ユーザーが実機（Windows ARM64）で、MSIX の中で設定が `%APPDATA%` に残り、アンインストールで消えないことを確かめる。

理由: 個人の登録は無料で、MSIX は証明書が要らない。ただし、MSIX の既定は設定の消去と私設の置き場への書き込みで、利用者のデータの決めに反するので、必ず外す。外すための制限付きの機能 `unvirtualizedResources` と、`runFullTrust` が、個人のアカウントで通るかは未確認。

審査に備えて、説明に書くこと: シェル・外部コマンド・プラグイン・同梱のスクリプトの処理系は、利用者が自分で選んで動かす（10.2.4 の依存の申告、10.2.2 の動的なコード）、「プラグインは利用者の権限で、制限なしに動く」（ADR-0026）。プライバシーポリシー: MATSU は文書を外に送らない（企画 4 の生成 AI が入るときに更新する）。年齢区分は IARC の質問票。

### 9. Mac App Store と Flathub

推奨: **今は見送る。** 段階 C の Microsoft Store の経験と、署名（段階 B）の後に、改めて決める。

理由: Mac App Store は砂が必須で、ADR-0028 の端末、外部コマンド、プラグイン、外部の言語を Store 版に入れられない（Termius と Prompt の前例）。Store 版だけ機能を減らす形（Cargo の feature）は技術的に作れるが、(a) 企画の中心（端末、生成 AI の CLI）が丸ごと無くなる、(b) 設定の置き場所が砂のコンテナに変わる（利用者のデータの決めの変更）、(c) 同梱の処理系が 2.5.2 に当たるか不明、(d) 2 つの版を保つ負担、(e) 年 99 ドルと審査がある。得は「Store で入れられる」だけ。Flathub は、AI が作ったマニフェストの禁止と、砂が端末と外部コマンドを損なうことが理由。

もし後で作るなら、Store 版の差は feature で切り、機能の差を製品の説明に書く。

### 10. リリースの自動化

推奨: 既存の `release.yml` の形を変えずに、次を足す。
- `package.yml` に、インストーラと `.deb`・`.rpm`・`.dmg` の作成を足す。成果物の名前は `matsu-<版>-windows-x64-setup.exe`、`-windows-arm64-setup.exe`、`-macos-arm64.dmg`、`-linux-x64.deb`、`-linux-arm64.deb`、`.rpm` など。既存の 5 つのアーカイブは残す
- `package-check.yml` の `paths` に、インストーラの定義ファイルを足す。PR で作成が壊れていないことを確かめる
- Windows の実行機（x64 と ARM64）で、インストール → ダミーの `matsu.ini` を `%APPDATA%\matsu-editor` に置く → アンインストール → ダミーが残る、を試験にする（案 7）。macOS と Linux も、`.dmg` と `.deb` の展開で同じことを確かめる
- 段階 B: 署名のジョブ（GitHub の実行機、Environment）、winget の `wingetcreate update --submit`（オーナーの PAT を secret）、Homebrew の tap の更新（tap のリポジトリだけに `Contents: write` の fine-grained PAT）。トークンが無いときは、その手順を飛ばす（`release.yml` の `HAS_TOKEN` と同じ作り）
- Scoop は、バケットの `autoupdate` と、バケット側の CI に任せる（こちらからの push は要らない。未確認）
- VM（macOS の実行機）には、署名や配布のトークンを渡さない（ADR-0027 のまま）

手でやること（残る）: アカウントの登録・本人確認・支払い、証明書の発行と更新、secrets の登録と更新、Store の初回の提出（説明文、スクリーンショット、プライバシーポリシー、年齢区分）、winget の最初の PR、他のリポジトリへの提出、署名が失敗したときの調査。

理由: 今の `release.yml` が使っている再利用のワークフローと、「トークンがないときは動かさない」の形を、そのまま使える。他のリポジトリへの提出は、CLAUDE.md の決まりで、オーナーの判断と操作である。

### 11. 自動更新

推奨: 作らない。更新は、インストーラの上書き、winget・Homebrew・Scoop の更新に任せる。Store 版は Store が更新する。

理由: 更新の仕組みは、署名と配布の信頼に関わり（セキュリティの判断）、署名の前に作る意味が薄い。ユーザーが要るときに別の ADR で決める。

### 12. ライセンスと依存の確認（ツール）

推奨: 製品に入るコードは増えない。CI の道具として、次を使う（どれも、ユーザーが決める）。
- Inno Setup（独自の寛容な許諾。商用を含む利用可）
- `cargo-deb`、`cargo-generate-rpm`（MIT と記憶。実装の PR で確かめる）
- `wingetcreate`（MIT）。`komac`（GPL-3.0）は使わない
- 段階 B: SignPath か Artifact Signing の action（利用規約に同意が要る）、`rcodesign` を使うなら許諾を確かめる

## PR の順番

段階 A:
1. `matsu.rc` に版情報を足す（製品名、発行元、版、著作権）。Windows の試験で確かめる
2. Windows のインストーラ（Inno Setup）の定義と、`package.yml` への作成の手順。x64 と ARM64
3. インストールとアンインストールが利用者のデータに触れないことを確かめる CI の試験（Windows x64・ARM64）
4. Linux: `.deb` と `.rpm`、`.desktop`、アイコン、`MimeType`、AppStream。実行に要る共有ライブラリの一覧
5. macOS: `.dmg`
6. 自分の Homebrew の tap の cask、Scoop のバケットのマニフェスト、winget のマニフェストを、ファイルとして用意（`docs/` か、別のリポジトリのひな形）。`winget validate` を通す
7. `docs/release.md` を更新（成果物の表、インストールの手順、アンインストールが何を消さないか）。README に「インストール」の節
8. 関連付け（拡張子の一覧と、「プログラムから開く」）。サクラの一覧に合わせる

段階 B（ユーザーの準備ができてから）:
9. Windows の署名（案 4 の選択が決まってから）
10. macOS の署名と公証（Apple Developer Program の登録後。署名のジョブを GitHub の実行機に）
11. winget の最初の PR（オーナーが出す）と、`wingetcreate update --submit` を `release.yml` に
12. Homebrew の tap と Scoop のバケットの公開と、自動更新

段階 C:
13. MSIX の作成と、データの仮想化の確認（ユーザーが実機で）
14. Microsoft Store の提出（ユーザー）

## 判断してほしいこと

1. **段階に分けること**（案 1）。推奨: 段階 A（署名・口座なし）を先に作る
2. **Windows のインストーラの形**（案 2）。推奨: Inno Setup の `.exe`（利用者ごとが既定、機械ごとも選べる）。MSI は要望が出てから。**新しい道具のライセンスの判断**を含む（Inno Setup の許諾）
3. **インストールとアンインストールが利用者のデータに触れないこと**（案 7）。推奨: 設定・未保存の文書・プラグイン・履歴を作らず、消さない。「設定も消す」の選択肢を作らない。**利用者のデータの決め**
4. **macOS の署名の置き場所**（案 3）。推奨: 鍵は GitHub の secrets（Environment）だけに置き、VM と Mac には置かない。署名のジョブは GitHub の実行機で、版のリリースの macOS は GitHub の `macos-15` で作る（ADR-0027 の 2026-11-06 の決めを、版のリリースについて戻す）。**リリースとセキュリティ、ユーザーが前に決めたことの変更**
5. **Apple Developer Program に年 99 ドルで登録するか**（案 3）。推奨: Homebrew の tap と `.dmg` を使ってもらい、署名が欲しくなった時点で登録する。個人は本名で登録することになる。**費用とアカウント（オーナーだけができる）**
6. **Windows の署名の道**（案 4）。推奨: SignPath Foundation に 1 人で出せるかを問い合わせる。通らなければ、署名なしのまま使う（OV 証明書は費用が続くので、利用者の声が出てから）。Artifact Signing は、日本の事業者として登録できるときだけ。**費用とアカウント、ライセンス（利用規約）**
7. **パッケージマネージャの公開**（案 5）。推奨: 自分の Homebrew の tap と Scoop のバケットを先に。winget はマニフェストを用意し、**最初の PR はオーナーが出す**。公式の `homebrew/cask`、Scoop の Extras、Chocolatey は見送り。**他のリポジトリへの提出（オーナーの判断と操作）**
8. **自動化のトークン**（案 10）。推奨: winget 用の PAT と、tap 用の fine-grained PAT を、オーナーが作って Actions の secrets に入れる。VM には渡さない。トークンが無いときは手順を飛ばす。**セキュリティ**
9. **ツールのライセンス**（案 12）。推奨: `wingetcreate`（MIT）を使い、`komac`（GPL-3.0）は使わない。`cargo-deb`、`cargo-generate-rpm` は、実装の PR でライセンスを確かめてから。**ライセンスと依存**
10. **Linux の配布**（案 6）。推奨: `.deb` と `.rpm` と tar.gz。AppImage は任意。Flatpak と Snap は見送り（Flathub は AI が作ったマニフェストと AI の提出を禁じるので、出すなら人が手で書いて出す）
11. **Microsoft Store**（案 8）。推奨: 段階 C で MSIX を試し、`%APPDATA%\matsu-editor` の仮想化を必ず外す。外せなければ出さない。個人は登録無料。プライバシーポリシーの URL を用意する。**ストアへの提出（オーナーの判断と操作）、利用者のデータ**
12. **Mac App Store と Flathub を見送ること**（案 9）。推奨: 見送り。後で作るなら、Store 版は feature で機能を減らす形（端末・外部コマンド・プラグイン・外部の言語を外す）で、設定の置き場所が変わる点も含めて、改めて決める
13. **自動更新を作らないこと**（案 11）。推奨: 作らない
14. **ファイルの関連付けと PATH**（案 2）。推奨: 関連付けは既定でオフ、「プログラムから開く」は既定でオン、PATH への追加は選択でオフ。サクラの拡張子の一覧に合わせるのは、別の PR
