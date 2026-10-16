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

- 作り直すにはネットワークが要ります（クレートの取得と、パッケージにライセンスファイルがないクレートの補正で、リポジトリからライセンスファイルを取るため）。補正はライセンスファイルのチェックサムを持っていて、ファイルが変わると使われなくなります。
- CI の `notices` ジョブが同じコマンドで作り直し、コミット済みのファイルと違えば失敗します。

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
