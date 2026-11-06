# MATSU Editor

サクラエディタの操作感を、macOS・Windows・Linux で使えるように作り直すテキストエディタです。

> 開発を始めたばかりで、エディタとしてはまだ使えません。UI の試作（[poc/gpui-textview](poc/gpui-textview/README.md)）だけ、Windows と Mac と Linux（WSLg で確認）で動かせます。

サクラエディタとは別のプロジェクトです。サクラエディタのソースとヘルプ（zlib License）を仕様の参考にしています。

## 目指すもの

- サクラエディタのキー割り当てと操作感
- 巨大なファイルでも軽快に開く（大きさに上限はなく、とても大きなファイルは範囲ごとに読み込む）
- サクラエディタと同じ正規表現エンジン（Onigmo）
- Python や Ruby で書けるマクロとプラグイン

## 起動のしかた

```sh
matsu [--separate-process] [サクラエディタのオプション] [--] [path...]
```

MATSU Editor は、使う設定ファイル（`matsu.ini`）1 つにつき、1 つのプロセスで動きます（[ADR-0016](docs/adr/0016-single-instance.md)）。`matsu` をもう一度起動すると、開くファイルと作業フォルダーを、動いている MATSU Editor に渡して、すぐに終わります。もう開いているファイルは、そのウィンドウが前に出ます。ファイルを付けないと、空のウィンドウが 1 つ開きます。

- `--separate-process`: 動いている MATSU Editor を探さず、見つけられもしない、独立したプロセスとして動かします（試験や開発のため）
- `--`: これ以降は、`-` で始まる名前でもファイルとして扱います

サクラエディタのコマンドラインオプション（`help/sakura/res/HLP000109.html`）も使えます。名前の大文字小文字は区別せず、値は `=` か `:` に続けて書きます。`-X=` `-Y=` `-VX=` `-VY=` は開いた直後のキャレットと表示の位置、`-CODE=` は文字コード（サクラの番号）、`-TYPE=` は拡張子でタイプ別設定を強制、`-R` はビューモード、`-SX=` `-SY=` `-WX=` `-WY=` は新しいウィンドウの大きさと位置、`-GREPMODE` `-GREPDLG` `-GKEY=` `-GREPR=` `-GFILE=` `-GFOLDER=` `-GOPT=` `-GCODE=` は Grep、`-M=` は読み込み後に実行するマクロのファイル、`-PROF=` は設定のプロファイル（設定ファイルのサブフォルダー）、`-@=` はレスポンスファイルです。`-NOWIN` `-GROUP=` `-DEBUGMODE` `-PROFMGR` は受け付けて何もしません。

### macOS で Finder や Dock から起動したときの PATH

Finder・Dock・Launchpad から起動したアプリはシェルを通らないので、`PATH` が最小のもの（`/usr/bin:/bin:/usr/sbin:/sbin`）になり、`~/.zshrc` も読まれません。そのままだと、Homebrew（`/opt/homebrew/bin`）や、自分で `PATH` に足した場所にあるプログラムが見つかりません。そこで MATSU Editor は、起動したときにログインシェル（環境変数 `SHELL`、無ければ `/bin/zsh`）を裏で 1 回動かして（`~/.zshrc` も読まれます）`PATH` を読み取り、外部コマンド実行・データベースのクライアント・マクロの言語（node・python3・ruby・php・pwsh・perl・emacs）・ctags・PowerShell を開く操作を、その `PATH` で探して動かします。

- 読み取りは起動を待たせません。プログラムを動かすときに、まだ読み終わっていなければ、読み終わるまで待ちます。
- 10 秒で読めなかったときは、MATSU Editor が起動したときの `PATH` のままで動かします（標準エラーに 1 行出します）。起動した直後に動くマクロ（`-M=` など）が、読み終わる前に動いたときも、起動したときの `PATH` で動きます。
- 渡すのは `PATH` だけです。ほかの環境変数は渡しません。`PATH` の相対パスの項目（`.` など）は使いません。起動したときの `PATH` にある場所は、後ろに足します。
- 重い設定を飛ばしたいときは、ログインシェルには環境変数 `MATSU_RESOLVING_ENVIRONMENT=1` が付いているので、`~/.zshrc` で見分けられます。

  ```sh
  [ -n "$MATSU_RESOLVING_ENVIRONMENT" ] && return
  ```

- 端末から `matsu` を起動したときは、端末の `PATH` をそのまま使います（シェルは動かしません）。`>` で出力をファイルに向けたり、`&` や `nohup` を付けたりしても同じです（制御端末があるかで見分けます）。Windows と Linux では何もしません。

### フォント

本文の既定のフォントは、Windows と macOS では BIZ UDGothic、Linux では Noto Sans Mono CJK JP です。macOS には BIZ UDGothic が最初から入っていないので、フォントブックでダウンロードすると、既定のフォントで描きます。入っていない（設定で選んだフォントも入っていない）ときは、Menlo とヒラギノ角ゴシックで描きます（設定のファイルは書き換えません）。

### フォントの選び方を調べる

文字が思ったフォントで描かれないときは、環境変数 `MATSU_FONT_DIAGNOSTICS=1` を付けて起動します。付けたときだけ、標準エラーに次を出します（付けないときの動きは変わらず、ファイルには何も書きません）。

- GPUI と MATSU の記録（warn・error のほか、GPUI・MATSU とフォントのライブラリの info・debug）。時刻が付きます
- 起動したときのフォントの診断: OS と版、MATSU の版、言語、設定のフォントと GPUI に頼む家族名、OS が持つ家族名の件数と、頼む名前が入っているか（大文字小文字を無視した場合も）、`BIZ`・`UD`・`ゴシック`・`Osaka`・`Hiragino`・`ヒラギノ` を含む名前、頼んだフォントが読み込めたか（GPUI が別のフォントに替えたか）、本文を描くフォントと選んだ理由（macOS で頼んだフォントが無いときの代わり）、`0`・`M`・`あ`・`漢` の送り幅と全角÷半角の比（2.0 なら 1:2）。macOS では、Spotlight が見つける BIZ UDGothic のファイルの場所も出します

すでに MATSU Editor が動いていると、後から起動したものはファイルを渡して終わるので、診断は出ません。先に動いている MATSU Editor を終えるか、`--separate-process` を付けて起動します。

```sh
# macOS
MATSU_FONT_DIAGNOSTICS=1 "/Applications/MATSU Editor.app/Contents/MacOS/matsu" 2>&1 | tee ~/Desktop/matsu-font.txt

# Linux
MATSU_FONT_DIAGNOSTICS=1 matsu 2>&1 | tee matsu-font.txt

# Windows（PowerShell。リリース版はコンソールを持たないので、標準エラーをファイルへ流す）
$env:MATSU_FONT_DIAGNOSTICS = "1"; .\matsu.exe 2> matsu-font.txt
```

## ドキュメント

- [開発の計画](docs/roadmap.md)
- [決定事項（ADR）](docs/adr/README.md)

## ライセンス

次のどちらかを選んで利用できます。

- MIT License（[LICENSE-MIT](LICENSE-MIT)）
- Apache License 2.0（[LICENSE-APACHE](LICENSE-APACHE)）

サクラエディタのコードを移植した部分には、そのファイルに元の著作権表示と zlib License を残します。

### サードパーティの著作権表示（Third-party notices）

プログラム（`matsu`、`matsu-script`）には、Rust のクレートや、クレートが同梱する C のソース（MicroPython、mruby、Onigmo、Lua、QuickJS など）、アイコンが入っています。これらのライセンスは、配布物に著作権表示とライセンス本文を添えることを求めています。その写しが [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) です。プログラムを配布するときは、このファイルをプログラムの隣に置きます。

- 形式はプレーンテキストにしました。どの OS の標準の道具でも開けて、梱包にもそのまま入れられるためです。
- `Cargo.lock` から [cargo-about](https://github.com/EmbarkStudios/cargo-about)（0.9.2）で作る生成物なので、手で直しません。設定は [about.toml](about.toml)（受け入れるライセンスは `deny.toml` と同じ一覧、対象は CI が作る 5 つのターゲット、ライセンスファイルの検出が足りないクレートの補正）、テンプレートは [about.hbs](about.hbs) です。
- 依存を更新したら、次のコマンドで作り直してコミットします。`cargo install --locked cargo-about --version 0.9.2 --features cli` で入れられます。

  ```sh
  cargo about generate --locked --all-features --workspace --fail about.hbs -o THIRD_PARTY_NOTICES.txt
  ```

- このリポジトリ自身のクレートは、リストに載せません（先頭の文で足ります）。新しいクレートを足しても、表示は変わりません。ただし、第三者のコード（C のソースなど）を同梱するクレート（今は `matsu-regex`、`matsu-micropython-sys`、`matsu-mruby-sys`）は載せます。そういうクレートを足すときは、`about.toml` に補正を足し、`about.hbs` に名前を足します。
- 作り直すにはネットワークが要ります（クレートの取得と、パッケージにライセンスファイルがないクレートの補正で、リポジトリからライセンスファイルを取るため）。補正はライセンスファイルのチェックサムを持っていて、ファイルが変わると使われなくなります。
- Linux 版は、OS の FreeType（`libfreetype-dev` など）にリンクします。OS に無いまま Linux でビルドすると、`freetype-sys` が FreeType を同梱してビルドします。同梱の FreeType は FreeType License（FTL）で、この表示には含まれていません。CI の Linux のテストジョブが、同梱ビルドの成果物（`libfreetype2.a`）がないことを確かめます。
- Wayland のプロトコル定義（`wayland-protocols-plasma`、LGPL-2.1-or-later）については、`THIRD_PARTY_NOTICES.txt` に LGPL の本文と事情を載せています（[CLAUDE.md](CLAUDE.md) に判断を記録）。
- CI の `lint` ジョブ（`notices` の段階）が同じコマンドで作り直し、コミット済みのファイルと違えば失敗します。

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
