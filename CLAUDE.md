# CLAUDE.md

このリポジトリで作業する AI エージェント向けの指示です。

## プロジェクト

サクラエディタの操作感を macOS・Windows・Linux で再現するテキストエディタ。Rust で書き、MIT OR Apache-2.0 で公開する。

- 名前は、英字で書く場所では「UME Editor」とする（SAKURA Editor に合わせる）。ウィンドウのタイトル、メニュー、README、文書の中の製品名など。小文字しか使えない場所や慣習で小文字にする場所（リポジトリ名の `ume_editor`、クレート名の `ume-*`、アプリの識別子（例: `io.github.stofu5678.ume-editor`））はそのままにする
- カタカナで書く場所では「ウメエディタ」とする（サクラエディタに合わせる。2026-10-17 にユーザーが判断）。サクラの表示名に「サクラエディタ」が入るもの（メニューの「サクラエディタの全終了」など）は、「ウメエディタ」に置き換える
- 計画と現在のフェーズ: [docs/roadmap.md](docs/roadmap.md)
- 決定事項: [docs/adr/](docs/adr/README.md)。ADR と食い違う変更をするときは、先に ADR を更新するか、ユーザーに確認する
- 初期開発では Windows ARM64 で動かして確かめることを優先する（[ADR-0009](docs/adr/0009-initial-development-target.md)）

## ビルドとテスト

Rust のバージョンは `rust-toolchain.toml` で固定している。push の前に、次を通す（CI と同じ内容）。

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny check        # cargo install --locked cargo-deny
typos                   # cargo install --locked typos-cli
```

- CI（`.github/workflows/ci.yml`）は Windows x64・ARM64、macOS ARM64、Linux x64・ARM64 でテストする。ほかに、依存のライセンスと脆弱性（cargo-deny、設定は `deny.toml`）、綴り（typos）、`cargo doc` の警告、CI の設定（actionlint）を確かめる
- 依存のライセンスは `deny.toml` の一覧にあるものだけ通る。一覧に足すときは、MIT / Apache-2.0 と両立するか確かめ、ユーザーに確認する
- クレートは `crates/` に置く。`ume-core` は UI に依存しないコア
- `poc/` は捨てる前提の試作で、ワークスペースにも CI にも入れていない。試作のビルドとテストは、その試作のディレクトリで行う（例: [poc/gpui-textview](poc/gpui-textview/README.md)）

## ブランチと PR

- main は GitHub で保護していて、直接 push できない。作業はブランチで行い、main への PR を作る。PR は CI が通ってからマージする
- PR のマージは、[ADR-0019](docs/adr/0019-auto-merge-trial.md) の条件をすべて満たすものは調整役（メインのセッションの AI）が行い、それ以外はユーザーが行う。担当（エージェント）はマージしない
- 試作は、試作のブランチ（`poc/gpui` など）にそのまま push してよい
- PR は squash でマージする。PR のタイトルがそのまま main のコミットになり、バージョンの自動付与に使うので、[Conventional Commits](https://www.conventionalcommits.org/ja/v1.0.0/) の形で英語で書く（例: `feat(commands): add delete_line`、`fix: keep the BOM on save`、`docs: record the printing PoC`、`ci: add cargo-deny`、`chore(poc): ...`）。CI の「PR title」が確かめる

## 設計のルール

- コア（バッファ・文字コード・検索・編集操作）は UI に依存させない。依存の向きは UI → コアの一方向にする
- 操作はすべて名前付きのコマンドとして実装する。1 コマンド 1 ファイルにし、全員が書き換える一覧（巨大な `match` など）を作らない。キー割り当て・メニュー・マクロはコマンドを呼ぶ
- 文書へのアクセスは範囲指定で行う。全文を 1 つの文字列で受け渡す API を作らない（巨大なファイルで、メモリと時間を使い切るため）
- OS 固有の処理は専用のモジュールに閉じ込める。macOS がメインだが、Windows と Linux でもビルドとテストが通る状態を保つ

## ライセンスのルール

- 依存を追加するときは、ライセンスが MIT / Apache-2.0 と両立するか確認する。GPL 系（GPL・LGPL・AGPL）のものは使わない
- MPL-2.0 のものは使ってよい（2026-10-10 にユーザーが判断）。ただし、MPL-2.0 のファイルを改変したら、そのファイルを公開する
- パーサー生成器（Bison・Lrama など）が作ったファイルは、先頭が GPL でも、生成物を好きな条件で配布してよいとする特別例外（Bison の "special exception"）が付いていれば使ってよい。GPL なのは生成器とその型紙（スケルトン）で、生成物は例外で外れるため。生成器そのものや型紙は使わない（2026-10-16 にユーザーが判断。mruby の `y.tab.c`）
- Zed のエディタ部分（GPL / AGPL のクレート）からコードを写さない。参考にしてよいのは GPUI 本体と gpui-component（どちらも Apache-2.0）まで
- bregonig（サクラが使う正規表現の DLL）のソースは、GPL か Artistic License の選択制で、このリポジトリでは使えない。ソースは読まず、写さない。挙動は説明書（`bregonig.html`）と、実物の bregonig.dll を動かした結果から決める（2026-10-16 にユーザーが判断）
- サクラエディタ（zlib License）のコードを移植するときは、ファイルの先頭に移植元のファイル名、元の著作権表示、zlib License の全文を残し、改変したことを明記する
- GPUI の Linux 版が使う wayland-protocols-plasma（MIT）は、KDE の Wayland プロトコルの定義（blur.xml など、LGPL-2.1-or-later）から作ったコードを含む。定義はプロトコルの取り決め（名前と引数の型）なので、そのまま使う（2026-10-17 にユーザーが判断）。THIRD_PARTY_NOTICES.txt に LGPL の本文と事情を載せる。cargo-deny はこれを検出しない（`deny.toml` に同じ注記がある）

## 判断の方針

PR を作るときの判断は、[ADR-0018](docs/adr/0018-decision-policy.md) の型に従う。

- 次の型の判断は自分で決める。PR の「判断したこと」に、型と理由を書く
  - A: サクラの明らかな不具合・効かない設定は、正しく動く形にする。保存したファイルやマクロの結果が変わっても同じ（利用者がまだいないため。2026-10-21 にユーザーが判断）。不具合と決める前にサクラのソースで理由を確かめ、マクロが頼っている動きは不具合として扱わない
  - B: OS や GPUI でサクラどおりにできないものは、いちばん近い形にする
  - C: 画面の細部やタイミングは、サクラに合わせる
  - D: 足りない土台が要るものは見送り、「残っていること」に理由を書く
  - E: CI の手間は、1 つのジョブが 20 分ほどまでで、ほかと並行して動くなら決めてよい
  - F: サクラに文言がない表示の言葉は、名前のルールに従って決める
- 次は決めずに、「判断してほしいこと」に書く
  - ライセンスと依存
  - ADR
  - 利用者のデータ（保存・バックアップ・削除、`ume.ini` の形式と置き場所）
  - セキュリティ
  - リリース
  - ほかの OSS への投稿
  - 保存したファイルやマクロの結果が変わる、サクラとの違い（型 A のサクラの不具合を直すものを除く）
  - ユーザーが前に決めたことの変更
  - 型に迷うもの
- サクラの挙動についての主張には、ソースの場所か、サクラ本体で確かめた結果を添える

## ほかの OSS への報告

GPUI・Zed・gpui-component・ashpd など、ほかのプロジェクトに issue・コメント・PR を出すときのルール（2026-10-11 にユーザーが決めた）。curl のように、生成 AI が書いた確かめられていない報告が、メンテナーの負担になっている。報告する人が、いつも最先端のモデルを使っているとは限らないことも、その一因と考える。

- 投稿の前に、ユーザーが自分で再現して確かめる。AI は自分から投稿しない。投稿するのは、ユーザーが確かめて頼んだときだけ
- AI は、ユーザーが自分で追える再現の手順と、報告の下書きを用意する。手順には、環境、版、コマンド、期待する結果と実際の結果を書く。ユーザーが手順を変えて確かめたときは、変えた手順と結果を下書きに反映する
- 手順と下書きは、ユーザーに渡す前に Codex（別の会社のモデル）にも確かめさせる。Claude と同じ見落としを避けるため
- 下書きには、確かめたことだけを書く。確かめていない推測は、推測と分かるように書くか、書かない
- AI を使ったことを書く

## サクラエディタの仕様の調べ方

上流は https://github.com/sakura-editor/sakura 。作業用の一時ディレクトリに `git clone --depth 1` して読む。

- コマンド一覧: `sakura_core/Funccode_x.hsrc`
- 既定のキー割り当て: `sakura_core/func/CKeyBind.cpp`
- 正規表現の構文: `help/sakura/res/HLP000089.html`
- 検索の実装: `sakura_core/agent/CSearchAgent.cpp`

サクラの挙動を再現したコードには、根拠にしたソースかヘルプの場所をコメントで残す。

## 書き方

- ドキュメント（README と docs/）は日本語で書く
- コードの識別子とコメントは英語で書く
