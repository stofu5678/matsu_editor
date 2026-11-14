# ヘルプを GitHub Pages に出す

ヘルプ（`docs/help/`）を、GitHub Pages で公開する手順です。決めたことは [ADR-0031](../adr/0031-help.md) の「決めたこと」5（公開: GitHub Pages で出す。リポジトリの設定は持ち主が切り替える）と、案 7（版の管理: main の最新だけを出す）です。このファイルは本の一部ではありません。

## 出来上がり

- 公開する場所: `https://stofu5678.github.io/matsu_editor/`（リポジトリが公開なので、だれでも読める）
- 中身: main の最新のヘルプ。配布物に入るヘルプ（#430）と同じ作り方で、同じもの（mdBook 0.5.4 の本と、Pagefind 1.6.0 の検索）
- 検索: Pages は `https://` で配るので、Pagefind の検索の画面がそのまま動く（配布物のヘルプは `file://` で開くため、1 ページ版を Ctrl+F で探す形にしている）
- 版ごとの公開はしない。古い版の人は、配布物に入っているヘルプを読む

## 持ち主がすること（1 度だけ）

1. リポジトリの **Settings → Pages → Build and deployment → Source** を **GitHub Actions** にする
   - これで `github-pages` という環境（Environment）ができる。既定では、既定のブランチ（main）からだけ公開できる
2. **Settings → Secrets and variables → Actions → Variables** に、リポジトリの変数 **`HELP_PAGES`** を作り、値を **`on`** にする
   - この変数が `on` でない間は、公開のワークフローは何もしない（本を作るだけで、公開の手順を飛ばす）。1 の前に動いて失敗するのを防ぐため
3. Actions の「Help pages」（下のワークフロー）を、手で 1 回動かす（Run workflow）。数分で上の場所に出る

トークンや秘密の値は要りません。ワークフローは、GitHub が用意する `GITHUB_TOKEN` と OIDC（`id-token: write`）だけで公開します。

## 止めたいとき

- いったん止める: 変数 `HELP_PAGES` を `off` にする（次の更新から公開しない。出ているページはそのまま）
- ページを消す: Settings → Pages で公開をやめる（Unpublish）

## ワークフロー（`.github/workflows/help-pages.yml`）の動き

- 動くとき: main に push され、ヘルプに関係するファイル（`docs/help/**`、`tools/help/**`、ヘルプが取り込むサンプルの `crates/matsu-plugin/samples/**`、ワークフロー自身）が変わったとき。手でも動かせる（`workflow_dispatch`）
- 本を作る: `tools/help/install-tools.sh`（版と SHA-256 を固定した mdBook と Pagefind を取る）と `tools/help/build.sh`（本、検索の索引、1 ページ版の `search.html`）。CI の `help` ジョブと配布物のジョブと同じ道具
- 公開する: 本を作る `build` ジョブが `actions/upload-pages-artifact`（v5）で本を渡し、続く `deploy` ジョブが `actions/deploy-pages`（v5）で `github-pages` の環境に出す。権限は `contents: read`、`pages: write`、`id-token: write` だけ（`pages: write` と `id-token: write` は `deploy` ジョブにだけ付く）
- 変数 `HELP_PAGES` が `on` でないとき、`deploy` ジョブは動かない（`build` ジョブまでで終わる）
- 同時に動くのは 1 つ（`concurrency` の `help-pages`）。公開の途中のものは、あとから来たものに取り消されない
- PR では動かない（main の最新だけを出すため）。PR のヘルプは、今までどおり CI の `help` ジョブが作って確かめる

## 公開のあとにすること

- アプリの「ヘルプ目次」「ヘルプキーワード検索」（#434）は、配布物にヘルプがないとき、今は GitHub 上の `docs/help/ja/SUMMARY.md` と GitHub のコード検索（ログインが要る）を開く。公開が始まったら、Pages のページと Pagefind の検索を開くように変える（`crates/matsu-ui/src/support/help.rs` の `ONLINE_BASE` を 1 か所直す、別の PR）
- README からヘルプの場所へのリンクを足す（別の PR）

## まだ確かめていないこと

- 1 の直後の `github-pages` 環境の既定の保護（既定のブランチだけ）が、このリポジトリの main の保護の設定と食い違わないか。最初の公開のときに Actions の画面で確かめる
