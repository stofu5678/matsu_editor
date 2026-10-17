# UME Editor

サクラエディタの操作感を、macOS・Windows・Linux で使えるように作り直すテキストエディタです。

> 開発を始めたばかりで、エディタとしてはまだ使えません。UI の試作（[poc/gpui-textview](poc/gpui-textview/README.md)）だけ、Windows と Mac と Linux（WSLg で確認）で動かせます。

サクラエディタとは別のプロジェクトです。サクラエディタのソースとヘルプ（zlib License）を仕様の参考にしています。

## 目指すもの

- サクラエディタのキー割り当てと操作感
- 巨大なファイルでも軽快に開く（大きさに上限はなく、とても大きなファイルは範囲ごとに読み込む）
- サクラエディタと同じ正規表現エンジン（Onigmo）
- Python や Ruby で書けるマクロとプラグイン

## ドキュメント

- [開発の計画](docs/roadmap.md)
- [決定事項（ADR）](docs/adr/README.md)

## ライセンス

次のどちらかを選んで利用できます。

- MIT License（[LICENSE-MIT](LICENSE-MIT)）
- Apache License 2.0（[LICENSE-APACHE](LICENSE-APACHE)）

サクラエディタのコードを移植した部分には、そのファイルに元の著作権表示と zlib License を残します。

### サードパーティの著作権表示（Third-party notices）

プログラム（`ume`、`ume-script`）には、Rust のクレートや、クレートが同梱する C のソース（MicroPython、mruby、Onigmo、Lua、QuickJS など）、アイコンが入っています。これらのライセンスは、配布物に著作権表示とライセンス本文を添えることを求めています。その写しが [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) です。プログラムを配布するときは、このファイルをプログラムの隣に置きます。

- 形式はプレーンテキストにしました。どの OS の標準の道具でも開けて、梱包にもそのまま入れられるためです。
- `Cargo.lock` から [cargo-about](https://github.com/EmbarkStudios/cargo-about)（0.9.2）で作る生成物なので、手で直しません。設定は [about.toml](about.toml)（受け入れるライセンスは `deny.toml` と同じ一覧、対象は CI が作る 5 つのターゲット、ライセンスファイルの検出が足りないクレートの補正）、テンプレートは [about.hbs](about.hbs) です。
- 依存を更新したら、次のコマンドで作り直してコミットします。`cargo install --locked cargo-about --version 0.9.2 --features cli` で入れられます。

  ```sh
  cargo about generate --locked --all-features --workspace --fail about.hbs -o THIRD_PARTY_NOTICES.txt
  ```

- このリポジトリ自身のクレートは、リストに載せません（先頭の文で足ります）。新しいクレートを足しても、表示は変わりません。ただし、第三者のコード（C のソースなど）を同梱するクレート（今は `ume-regex`、`ume-micropython-sys`、`ume-mruby-sys`）は載せます。そういうクレートを足すときは、`about.toml` に補正を足し、`about.hbs` に名前を足します。
- 作り直すにはネットワークが要ります（クレートの取得と、パッケージにライセンスファイルがないクレートの補正で、リポジトリからライセンスファイルを取るため）。補正はライセンスファイルのチェックサムを持っていて、ファイルが変わると使われなくなります。
- Linux 版は、OS の FreeType（`libfreetype-dev` など）にリンクします。OS に無いまま Linux でビルドすると、`freetype-sys` が FreeType を同梱してビルドします。同梱の FreeType は FreeType License（FTL）で、この表示には含まれていません。CI の Linux のテストジョブが、同梱ビルドの成果物（`libfreetype2.a`）がないことを確かめます。
- Wayland のプロトコル定義（`wayland-protocols-plasma`、LGPL-2.1-or-later）については、`THIRD_PARTY_NOTICES.txt` に LGPL の本文と事情を載せています（[CLAUDE.md](CLAUDE.md) に判断を記録）。
- CI の `notices` ジョブが同じコマンドで作り直し、コミット済みのファイルと違えば失敗します。

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
