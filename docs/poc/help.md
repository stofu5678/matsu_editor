# ADR-0031 の下調べ: ヘルプ（2026-11-13）

ユーザーの企画（2026-11-11）: 「ヘルプ」「Github Pagesで用意？」「サクラエディタ相当のヘルプに加えてスクリプトのサンプルも欲しい」。

調べた人: Sonnet 5.5 の担当。読み取りだけ。リポジトリは変えていない。確かめていないことは 推測 / 未確認 と書く。

作業場所: サクラは `git clone --depth 1 https://github.com/sakura-editor/sakura` で、作業用の一時ディレクトリに取った（HEAD の日付 2026-11-13）。以下 `sakura/` はそのクローン。

---

## 1. サクラのヘルプ

### 1.1 作りと数

`sakura/help/` は 3 つの別々のヘルプに分かれる。どれも HTML のファイルと、HTML Help Workshop の 3 点セット（`.hhp` プロジェクト、`.hhc` 目次、`.hhk` 索引）でできている。

| ヘルプ | 場所 | HTML の数 | 中身 |
|---|---|---|---|
| 本体のヘルプ（`sakura.chm`） | `help/sakura/res/` | 392 | `HLP000001.html`（目次）から `HLP000xxx.html` が 370、`HLP_UR000`〜 が 18、`CNT000924.html`（目次の別版）、`HLP_HISTORY` `HLP_LISENCE` `HLP_RELEASE` |
| マクロのヘルプ（`macro.chm`） | `help/macro/source/` | 365 | 関数の説明 329（`reference/` 下の 15 グループ）、サンプル 19、使い方 12、ほか 5 |
| プラグインのヘルプ（`plugin.chm`） | `help/plugin/Text/` | 10 | プラグインの仕様と実装の覚え書き |

- 合計で HTML 767 枚、画像 85 枚（PNG）、全体で約 5.8 MB（`du -sh help`）。
- 目次 `help/sakura/sakura.hhc`（2184 行）が章立てを持つ。索引 `sakura.hhk`（1643 行、409 項目）。目次の項目は各ページの `<META NAME="MS-HKWD">` のキーワード（`HLP000100.html` の例: 「目次」「ヘルプ目次」「HelpContents」）と対になる。
- 本体のヘルプの章立て（`sakura.hhc` を数えた。ページ数は重複を除いた HTML の数）:

| 章 | ページ数 | 内容 |
|---|---|---|
| サクラエディタ概要 | 10 | 機能概要、動作環境、使用条件、インストール手順 |
| 基本的な使い方 | 25 | 選択、矩形編集、正規表現、入力補完、タブバー、インクリメンタルサーチ、プラグイン、表示言語など |
| コマンド概要 | 322 | メニューごとに 1 ページずつ。下の表 |
| デフォルト設定値 | 4 | タイプ、キー割り当て一覧、コマンド一覧 |
| 仕様等 | 9 | 改行コード、コマンドラインオプション、単語、文字コード、メタ文字列、ExpandParameter、スマートインデント、ダイレクトタグジャンプ |
| マクロ | 6 | マクロの概要、キーボードマクロ、WSH と PPA、マクロ専用関数、記載例 |
| サクラエディタについて | 20 | 連絡先、変更履歴 |
| このヘルプファイルについて | 2 | ライセンス、更新履歴 |

コマンド概要の内訳（目次の項目の数。ページと一致するとは限らない）:

| メニュー | 項目（おおよそ） |
|---|---|
| ファイル | 27 |
| 編集 | 約 95（「カーソル前を削除」の下に 83 項目の入れ子がある） |
| 変換 | 約 26 |
| 検索 | 約 45 |
| ツール | 約 15 |
| 設定 | 約 60（共通設定 21 ページ、タイプ別設定 8 ページ、そのほか） |
| ウィンドウ | 約 40 |
| ヘルプ | 8 |
| メニューに入ってないコマンド | 約 30 |

- 画像は `help/sakura/res/images/`（`About.png`、`BoxEdit_1.png` など）。見た目は `dsk_sakura.css` と、各ページが読む `item.js`（目次への戻りリンクを出す）。

### 1.2 CHM の作り方

- `help/CMakeLists.txt`（Python 3.14 と MSBuild と PowerShell が要る）と、ルートの `build-chm.bat`。`hhc.exe`（Microsoft HTML Help Workshop）で `.hhp` から `.chm` を作る。`sakura/build-chm.bat` は変換した作業用ディレクトリ `build/build-chm/converted_help_source` に写してから `hhc` を動かす。`.gitignore` は `*.chm` を管理外にする。
- `sakura.hhp` は `Full-text search=Yes`、`Language=0x411 日本語`、既定のページ `res\HLP000001.html`。
- 本体のコードがヘルプのページを引く仕組み:
  - 機能コードからページ番号: `sakura_core/func/Funccode.cpp`（838 行付近 `case F_OPTION_GENERAL: return HLP000081;`）。メニューで項目を選んで F1 を押すと、この表でページが決まる。
  - ダイアログのヘルプボタン: `sakura_core/typeprop/CDlgTypeAscertain.cpp:58` の `MyWinHelp( GetHwnd(), HELP_CONTEXT, HLP000338 )` のように、ダイアログごとにページ番号が埋まっている。コントロールの「これは何？」は `HELP_CONTEXTMENU` と `help/sakura/Cshelp.txt`（3835 行）。
- MATSU に写すときの含み: ページ番号（`HLP000xxx`）は、サクラの中でだけ意味がある。MATSU は章ごとの意味のある名前（例 `commands/file/open.md`）にして、サクラのページ番号との対応表を別に持つのがよい（コードのコメントがすでに `HLP000089.html` のように番号を引いているため。例: `crates/matsu-regex/src/tests.rs`、`crates/matsu-core/src/tag_jump.rs:53`）。

### 1.3 ライセンス（重要）

**ヘルプのライセンスは、リポジトリの zlib License と違う。GNU Free Documentation License（GFDL）だ。**

- リポジトリの `sakura/LICENSE`: zlib License、`Copyright (C) 2018-2022, Sakura Editor Organization`。ヘルプのことは書いていない（`grep -n help LICENSE README.md` に当たりなし）。
- `help/sakura/res/HLP_LISENCE.html`（「このヘルプファイルについて」）: 「このヘルプファイルは GFDL (GNU Free Documentation License) Ver1.1 の規定に従って配布されます」と書き、GFDL 1.1 の全文を載せる。
- `help/sakura/res/HLP000001.html` の末尾（目次ページ）:
  > Copyright (c) 2001-2020 Project SAKURA Editor.
  > Permission is granted to copy, distribute and/or modify this document under the terms of the GNU Free Documentation License, Version 1.1 or any later version published by the Free Software Foundation.
  > A copy of the license is included in the section entitled "GNU Free Documentation License".

  Invariant Sections、Front-Cover Texts、Back-Cover Texts の指定は見当たらない（上の注記にない）。
- `help/macro/source/gfdl.html`: マクロのヘルプにも GFDL 1.1 の全文がある。`help/macro/source/right.html`（「このヘルプファイルについて」）は、「このマクロリファレンスは、すいさんによるサクラエディタ設定セット S_macppa_20030822_01.LZH (2003.8.22) に含まれる S_MACPPA.KHP を元にして作成されたもの」「2006.10.07 更新」と書く。元の S_MACPPA.KHP の権利者と条件は、ここからは分からない（**未確認**。マクロの関数の 329 ページに関わる）。
- `help/plugin/Text/index.html`: `Copyright (c) 2001-2014 Project Sakura-Editor.` と、同じ GFDL 1.1 or any later version。`help/plugin/Text/HLP_LISENCE.html` も同じ。
- `help/sakura/res/HLP000004.html`（使用条件等）: 「このソフトはフリーウェアです。ご自由にご利用下さい。ソースコードの取り扱いについては、sakura.exe と同時に配布される文書によって、別途規定する」。
- 著作権者: 本体のヘルプは「Project SAKURA Editor」（2001-2020）、プラグインは「Project Sakura-Editor」（2001-2014）。個人の名前は、この 2 つの表示にはない。ページごとの作者は `HLP_HISTORY.html`（更新履歴）にある（中身は読んでいない。**未確認**）。
- 「or any later version」とあるので、GFDL 1.3 の条件で配ってもよい。ただし GFDL はコピーレフトなので、**サクラのヘルプの文を写した派生物は、GFDL（1.1 以降）で配ることになる**。MIT OR Apache-2.0 には直せない。これが ADR の中心の判断になる。
- GFDL 1.1 が写した版に求めること（全文は `HLP_LISENCE.html` の英文。第 4 節「MODIFICATIONS」を要約。原文で確かめた行は 193 行目の G など。要約は私の読みで、**法的な助言ではない**）:
  - 派生版も同じ GFDL で配る。ライセンスの全文を付ける
  - 題名は、元と別にする。履歴（History）の節に、題名、年、著者、発行者を書き、元から変えたことを書く
  - 元の著作権表示を残す
  - 透明な写し（機械で読める形。HTML や Markdown は当たる）を付けるか、どこで取れるか書く
  - 技術的な手段で、読んだり写したりを妨げない
  - Invariant Sections がないので、残す義務のある節はない
- コード（MIT OR Apache-2.0）とヘルプ（GFDL の部分）を同じ配布物に並べるのは「集合物（aggregate）」で、コードに GFDL は及ばない（GFDL 第 7 節 AGGREGATION。**推測**。弁護士に確かめていない）。ただしアプリに組み込む（ヘルプの文を実行ファイルに埋める）形は避けて、別のファイルとして置くほうがよい。
- 事実（コマンドの動き）は著作物ではないが、サクラの文章を言い換えただけのものが「写した」と見なされるかの線引きは難しい（**推測**）。確実に避けるには、サクラの文を見ずに MATSU の動きから書く。MATSU は、動きの根拠をサクラのソースやヘルプのページ番号でコードのコメントに残している。ヘルプの文は書き直しても、ページ番号の引用は残せる。
- CLAUDE.md の「ライセンスのルール」は、GPL・LGPL・AGPL を使わないと決めている。GFDL（GNU の文書用）には触れていない。GPL 系に近い性格（コピーレフト）なので、**ユーザーの判断が要る**。bregonig の説明書（`bregonig.html`）の扱いとは別の話（あちらはソースが GPL / Artistic で、説明書は読んでよいと決めた）。

### 1.4 サクラのオンライン公開

- あり。`https://sakura-editor.github.io/` の上部「ヘルプ」が `https://sakura-editor.github.io/help/`。「サクラエディタ Ver 2.4.3 のヘルプファイルの目次」「最終更新 2026/08/08」。静的な HTML で、`HLP000002.html`、`HLP000204.html` のような相対リンク。ライセンスの表示は GFDL（1.1 以降）。本体のヘルプと、同じ構成のマクロのヘルプ（「マクロ」の章）が載る。プラグインのヘルプの索引は別になく、「基本的な使い方 > プラグイン」が `HLP000110.html` を指す（WebFetch の要約。ページの生の HTML は見ていない）。
- つまり、MATSU から「サクラのヘルプ」へリンクする案もある。ただしサクラの動きと MATSU の動きは同じではなく、ページの番号と内容は版で変わる。
- サクラの別の資料: GitHub Wiki（開発者向け）、land.to の Wiki（FAQ）。マクロの掲示板は過去ログ。

### 1.5 写すときに付けるもの（提案）

CLAUDE.md が、コードの移植に求める: 移植元のファイル名、元の著作権表示、zlib License の全文、改変したと明記。ヘルプのページに置き換えると、次になる。

1. 各ページの先頭（Markdown のコメントか、フロントマター）に:
   - 移植元のパス（例 `sakura/help/sakura/res/HLP000081.html`）と、取った日付とコミット
   - 元の著作権表示（`Copyright (c) 2001-2020 Project SAKURA Editor.`）
   - 「GNU Free Documentation License 1.1 or any later version」（SPDX で `GFDL-1.1-or-later`）
   - 改変したこと（「MATSU Editor 向けに書き直した」「追加した段落」など）
2. ページに見える形で、フッターに「このページは、サクラエディタのヘルプ（HLP000081）を元にしています。GFDL 1.1 以降」。
3. サイトの 1 か所（「このヘルプについて」）に GFDL の全文、履歴の節（元の題名「サクラエディタ ヘルプ」と、MATSU のヘルプは別の題名であること）、元の著作権者の一覧。
4. サクラの文を写した（ported）ページは `help/` の中でも別の場所に置くか、フロントマターの `license: GFDL-1.1-or-later` で印を付けて、MATSU が自分で書いたページ（ライセンスは MIT OR Apache-2.0 か、CC BY など別に決める）と見分けられるようにする。
5. サクラのマクロのヘルプ（329 ページ）は、元の S_MACPPA.KHP の権利が不明なので、**写さない**のを勧める（未確認が解けるまで）。関数の名前と引数（コードの事実）から、MATSU が書き直す。

---

## 2. MATSU の今のドキュメント

### 2.1 ヘルプのコマンド

| コマンド | 今の動き | ソース |
|---|---|---|
| `F_HELP_CONTENTS`（Alt+F1） | `CONTENTS_URL` = `<repository>/tree/main/docs`（GitHub の docs フォルダ）をブラウザで開く | `crates/matsu-ui/src/ui_commands/support/help_contents.rs`、`crates/matsu-ui/src/support/help.rs`（`CONTENTS_URL`）、`help_flow.rs`（`program_help`） |
| `F_HELP_SEARCH`（Alt+Shift+F1） | `INDEX_URL` = `<repository>/blob/main/docs/adr/README.md`（決定事項の一覧）を開く | `help_search.rs`、`help.rs`（`INDEX_URL`） |
| `F_EXTHELP1`（Ctrl+F1） | 設定の外部ヘルプ１のファイルを開く。HTML ヘルプ（`.chi` `.chm` `.col`）なら、キャレットの下の語で引く（Windows だけ `HtmlHelp`。ほかの OS は OS で開き、語は引かない）。未設定なら「今すぐ設定しますか?」 | `exthelp1.rs`、`help.rs`（`plan_ext_help`）、`crates/matsu-platform/src/help/{mod,windows}.rs` |
| `F_EXTHTMLHELP`（F1 の既定） | 外部 HTML ヘルプのファイルを語で引く。未設定なら同じ確認 | `exthtmlhelp.rs`、`help.rs`（`plan_html_help`） |
| `F_MENU_ALLFUNC`（Shift+F1） | コマンド一覧（機能の群ごとのメニュー）。ヘルプの文書は開かない | `menu_allfunc.rs`、`support/command_list.rs` |
| `F_ABOUT`（Ctrl+Shift+F1） | バージョン情報 | `about.rs` |
| ダイアログのヘルプボタン | ない。共通設定のボタンは `.disabled(true)`（`crates/matsu-ui/src/common_settings/dialog.rs:1589`）。タイプ別設定の 2 か所（`same_color.rs:392`、`keyword_select.rs:283`）にラベルがある。台帳は「検索・Grep・実行・タグ作成・印刷設定・コントロールコード・ファイルツリーで未対応」（`docs/sakura-gaps.md:124`、型 D） | |

- F1 の既定の割り当ては `crates/matsu-commands/src/sakura/key_bindings.rs:65`（サクラどおり: 無印 `F_EXTHTMLHELP`、Shift `F_MENU_ALLFUNC`、Ctrl `F_EXTHELP1`、Ctrl+Shift `F_ABOUT`、Alt `F_HELP_CONTENTS`、Alt+Shift `F_HELP_SEARCH`）。つまり、サクラの既定では、**無印の F1 は本体のヘルプではなく「外部 HTML ヘルプ」**で、未設定なら確認が出る。本体のヘルプは Alt+F1。MATSU もこれを引き継いでいる。
- URL を開く道は `cx.open_url(&url)`、ファイルは `cx.open_with_system(&path)`（`help.rs` の `perform`）。ファイルを開く道で `#アンカー` が渡るかは **未確認**（OS ごとに違いそう。`file://…#id` の URL なら通る見込みだが確かめていない）。
- `help.rs` の先頭のコメントに「MATSU Editor has none (help file)」とある。`F_HELP_CONTENTS` と `F_HELP_SEARCH` はウェブの文書で代用している。
- 「ヘルプ(H)」のメニューは `menu.rs`（ラベル `ヘルプ(H)`）。

### 2.2 利用者向けの文書

- `README.md`（104 行）: 目指すもの、起動のしかた（macOS の PATH、フォント）、ドキュメントのリンク（ロードマップと ADR だけ）、ライセンス。使い方の説明はない。
- `docs/`: `roadmap.md`、`sakura-gaps.md`（機能台帳）、`release.md`、`e2e-screens.md`、`adr/`（28 本）、`poc/`。**どれも開発者向け**。利用者向けの使い方はまだない。
- `docs/release.md`: パッケージは 5 つ（Windows x64 / ARM64 の `.zip`、macOS ARM64 / Linux x64 / Linux ARM64 の `.tar.gz`）。中身は `matsu`、`matsu-script`、`LICENSE-*`、`THIRD_PARTY_NOTICES.txt`、`icons/`、`README.txt`。ヘルプを入れるなら、ここに `help/` を足す。
- ドキュメントは日本語（CLAUDE.md「書き方」）。

### 2.3 サクラとの違い（ヘルプに書くこと）

`docs/sakura-gaps.md` の主な「違う」「ない」:
- 関数表のコマンド 363 個のうち、`SAKURA` 定数のあるものは 358 個（`docs/sakura-gaps.md:72`）。マクロ関数 59、マクロ専用 23 は、名前のコードがすべてある。
- 違い（型 A・B・C）: スマートインデントのプラグインを呼ぶ時機（`:138`）、Python のフォルダー設定（マクロのホストが自分で探す）、検索の灰色の設定 2 つ、印刷（プリンターの選択）、Migemo（ライセンスの判断待ち、`:58`）、巨大ファイルの残り（`:155`）。
- マクロの言語: サクラは WSH（JScript / VBScript）と PPA。MATSU はソースのままの互換を求めない（ADR-0007、ADR-0025）。JScript は QuickJS で動かす（`WScript` `ActiveXObject` を使わない部分）。VBScript は書き換えが要る。ADR-0025 に「VBScript から書き換える手引き」が残課題として書いてある。**この手引きはヘルプの最初の仕事になる**。

### 2.4 MATSU だけの機能（新しいページが要る）

| 機能 | 根拠 |
|---|---|
| 起動、`matsu.ini`、設定の置き場所（サクラの `sakura.ini` との違い） | README、ADR-0010、ADR-0021 |
| シェルのタブ（端末）と設定 `[Terminal]` | ADR-0028、#389 #390 |
| プレビューのタブ（Markdown） | ADR-0024 |
| タブとグループ | ADR-0015 |
| 多重起動の制御 | ADR-0016 |
| 巨大ファイル（10 GB まで。走査、概算の移動） | ADR-0005、ADR-0012、ADR-0020 |
| 未保存の文書の自動保存 | ADR-0022 |
| データベース（`databases.ini`、出力パネル、サンプルのマクロ） | ADR-0023 |
| 常駐するプラグイン（`plugin.def`、ジャック 6+、設定のページ） | ADR-0026 |
| スクリプトの言語（Node.js、TypeScript、Python、Ruby、PHP、PowerShell、Perl、Emacs Lisp、AppleScript は macOS だけ。同梱の Lua、QuickJS、MicroPython、mruby） | ADR-0007、ADR-0025、`crates/matsu-plugin/src/session.rs` の `Language`、`crates/matsu-script/` |
| 選択範囲のエンコード・デコード（URL・HTML・Base64） | roadmap Phase 9 (1) |
| 画面の読み上げ（スクリーンリーダー） | roadmap Phase 8 (4) |
| 印刷 | ADR-0011 |
| 正規表現（Onigmo、bregonig 互換） | ADR-0006 |
| サクラとの違いの一覧 | `docs/sakura-gaps.md` |
| 予定の機能（ワンライナー、生成 AI に送る）は、入ったあとにページを足す | roadmap Phase 9 (3)(4) |

---

## 3. 静的サイトの道具

調べ方: 公式の文書と検索（URL は下の表の下）。mdBook・Zola・Pagefind は、この PC（Linux ARM64）の作業用ディレクトリに、実行ファイルを取ってきて動かした（システムには入れていない）。ほかは文書だけ。

### 3.1 日本語の全文検索の実測

同じ小さな日本語のサイト（2 ページ、「検索ダイアログで文字列を検索します。置換もできます。」など）で、検索の索引の中に日本語の語が入るか調べた。

| 道具 | 結果 | 確かめ方 |
|---|---|---|
| mdBook 0.5.4（組み込みの検索） | **日本語の語は索引に 1 つも入らない**（英語の語 `option` `regex` は入る）。本文が空のように扱われる。`book.toml` に `language = "ja"` を書いても同じ。検索の結果は日本語では出ない | `searchindex-*.js` の `index.index.body.root` を読んだ。英語を足すと `option` `regex` だけが木にあった。elasticlunr の `trimmer`（`\W` を削る）が、日本語を消すとみられる（**推測**）。mdBook の Issue #1081 に同じ話（elasticlunr の他言語の対応の予定はない） |
| Zola 0.23.6（組み込み） | `default_language = "ja"` で**ビルドが失敗**（`Tried to build search index for language ja which is not supported`）。`en` にすると日本語の語は索引に入らない（`option` `regex` `us` だけ） | 実行して確認 |
| **Pagefind 1.6.0**（MIT。生成した HTML を後から索引にする） | **日本語で引ける**。「検索」「文字列」「ダイアログ」「マクロ」「保存」「置換」のほか、語の途中（「ログ」「存します」「正規」）でも当たった。形態素解析ではなく、文字を切って索引にするとみられる（索引の言語の部品は `wasm.unknown.pagefind` で、日本語専用ではない。**推測**）。語の途中の一致が当たるのは都合がよい一方、順位の質は未確認 | CLI で `pagefind --site site` を実行し、`pagefind.js` を `http://127.0.0.1` から読んで Node から `search()` を 12 語で試した。結果は全部期待のページ |
| MkDocs + Material | 組み込みは lunr + lunr-languages。日本語の語の切り出しは `lang: ja`（TinySegmenter）。公式の文書の検索のページに ja の記述はなく、実機で試していない（**未確認**）。過去に「v4.0.1 のあと日本語の検索が壊れた」という報告がある（検索結果で見えた。**未確認**） |
| Docusaurus | 組み込みの検索はない（Algolia の DocSearch か、プラグイン）。`@easyops-cn/docusaurus-search-local` は `language: "ja"` を受けるが、日本語専用の切り出しがあるかは分からない。別の派生の `@cmfcmf/docusaurus-search-local` は「ja と th は別の切り出しなので動かない」と書く（検索で見えた。**未確認**） |
| Hugo | 検索は付いていない。Pagefind などを足す |
| Jekyll | 検索は付いていない。lunr.js の追加か Pagefind。Just the Docs の lunr は中国語・日本語に対応していないという議論がある（**未確認**） |

**結論**: 日本語の検索は、生成する道具ではなく、Pagefind（どの道具の HTML にも載せられる）で確保するのがよい。

### 3.2 比較

| | mdBook | Zola | Hugo | MkDocs (+ Material) | Jekyll | Docusaurus |
|---|---|---|---|---|---|---|
| ライセンス（道具） | MPL-2.0（確認: 公式 GitHub） | 0.22 までの分は MIT、それ以降に入ったコードは **EUPL-1.2**（README の記述。確認: `https://github.com/getzola/zola`、取った 0.23.6 の `LICENSE` は EUPL、`LICENSE-MIT` も同梱） | Apache-2.0（記憶。**未確認**） | MkDocs は BSD-2、Material は MIT（記憶。**未確認**） | MIT（記憶。**未確認**） | MIT（記憶。**未確認**） |
| CI への入れ方 | 実行ファイル 1 つ（tar.gz 約 4.8 MB、`mdbook-v0.5.4-*-unknown-linux-musl`）。Rust のツールチェーンがあるので `cargo install --locked mdbook` もできるが遅い（**未確認**）ので、公開の実行ファイルのダウンロード + チェックサムか、`taiki-e/install-action` | 実行ファイル 1 つ（45 MB、展開後） | 実行ファイル 1 つ（Sass を使うなら extended 版）（**未確認**） | Python + pip（`mkdocs-material`）。依存が多い | Ruby + Bundler。GitHub Pages 組み込みの `actions/jekyll-build-pages` もある | Node.js + npm。`node_modules` が大きい（数百 MB。**推測**） |
| ビルドの時間（実測） | 2 ページで 0.024 秒 | 2 ページで 0.012 秒 | 未確認（一般に最速） | 未確認 | 未確認 | 未確認（数十秒〜。**推測**） |
| 日本語の検索 | **だめ**（3.1） | **だめ**（3.1） | なし（Pagefind） | ja 設定あり（未確認） | なし | プラグイン（未確認） |
| オフライン（同じページを配布物に入れ、ディスクから開く） | ◎ `file://` で動く作り。検索の索引が `.js` なので `file://` でも読める。1 ページ版の `print.html` もある。ただし日本語の検索はだめなので、ブラウザの Ctrl+F が代わり | `base_url` と相対リンクに注意（**未確認**） | 同左（`relativeURLs` の設定が要る。**未確認**） | Material に `offline` プラグインがあるが、組み込みの検索は `fetch` を使うので `file://` では不可（**未確認**） | 相対リンクにすれば開ける（**未確認**） | 通常のビルドは `file://` 向きでない（**未確認**） |
| 版ごとの管理 | 自前（`/v0.13/` のようにフォルダを分けて出す）。公式の仕組みはない | 自前 | 自前 | `mike`（Material の定番） | 自前 | `docusaurus docs:version` を内蔵（版ごとのスナップショット） |
| アンカー（`#...`） | 見出しの文字から ID を作る。**日本語の見出しはそのまま ID になる**（実測: `search.html#文字列を検索する`）。見出しの文字を変えると ID が変わる。明示の ID（`{#id}`）はなし | `{#id}` が書ける（Markdown の拡張。**未確認**） | 見出しの ID を `{#id}` で指定できる（**未確認**） | `attr_list` で `{ #id }` が書ける | kramdown の `{#id}` | `{#id}` が書ける |
| 向く点 | 小さくて速い。Rust の仲間。単一バイナリ | | 速い。テーマが多い | 見た目が良い。ただし Material は保守のみ | GitHub Pages が標準で作る | 版の管理が強いが重い |
| 弱い点 | 日本語検索、アンカーの安定、プラグインが少ない | 日本語検索、ライセンスの注意 | 文書向けのテーマを選ぶ手間 | Material は 2025-11 に保守のみ（新しい機能は後継の Zensical へ）。MkDocs 2.0 の話で先が不透明（検索で見えた。**未確認**） | Ruby。文書向けの機能が弱い | 重い。Node |

出典（検索と取得の結果。生の文書の全文は読んでいないものがある）:
- mdBook の検索の設定: https://rust-lang.github.io/mdBook/format/configuration/renderers.html （検索の言語の記述はなし）
- mdBook の Issue（elasticlunr、他の言語）: https://github.com/rust-lang/mdBook/issues/1081
- Zola のライセンス: https://github.com/getzola/zola （README「Code introduced after version 0.22 is licensed under the EUPL-1.2」）
- Zola の検索の言語: Issue 検索で「ko の検索の索引を足してほしい」（https://zola.discourse.group/t/add-search-index-support-for-korean-ko/2597）。ja は公式の実行ファイルでは使えないと実測した。
- Material for MkDocs: https://squidfunk.github.io/mkdocs-material/setup/setting-up-site-search/ ，保守のみの話: https://squidfunk.github.io/mkdocs-material/blog/2025/11/11/insiders-now-free-for-everyone/ ほか
- Docusaurus の検索: https://github.com/cmfcmf/docusaurus-search-local 、https://dev.classmethod.jp/articles/fix-local-search-on-docusaurus-v3-2023-11-13/
- Pagefind: `npm install pagefind`（1.6.0、`package.json` の license は MIT）。

### 3.3 これらを踏まえた推し

- 生成の道具: mdBook（小さい、速い、`file://` で動く、1 ページ版の出力、Rust の仲間）。検索は Pagefind を足す。弱点（日本語の検索の組み込みが効かない、アンカー）は、Pagefind と「見出しの ID を壊さないための検査」で補う。
- 代わりの案: Hugo + Pagefind（テーマを選ぶ手間があるが、多言語・版の管理が自由）。
- 避ける: Zola（日本語の検索が動かない、EUPL）、Docusaurus（重い）、MkDocs Material（保守のみ）。
- 注意: mdBook は 0.5 系（今 0.5.4）で、0.4 から設定と出力が変わった（記憶。**未確認**）。版を固定する。

### 3.4 Pagefind と `file://`

- 私の試験は `http://127.0.0.1` から読んだ。`file://` で Pagefind が動くかは**試していない**。ブラウザは `file://` からの `fetch` を許さないことが多い（**推測**）。そうなら、配布物の中のヘルプは、(a) mdBook の `print.html`（1 ページ版）をブラウザの Ctrl+F で検索する、(b) アプリが `127.0.0.1` の小さなサーバーでヘルプを出す、(c) ヘルプはオンラインを使い、ローカルの写しは無検索、のどれかになる。(b) は新しい作りで、セキュリティの判断が要る。

---

## 4. 公開（GitHub Pages）

出典: https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages 、https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits

### 4.1 必要なもの

- 公開リポジトリ（無料のプランで Pages を使うには公開が要る。**推測**。文書にはその記述が見つからなかった。このリポジトリが公開かは、`Cargo.toml` の `repository` と CLAUDE.md の公開の記述から公開とみられるが**未確認**）。
- ワークフロー: `actions/configure-pages@v5`、`actions/upload-pages-artifact@v4`、`actions/deploy-pages@v4`（文書の例の版。使うときは固定の版を確かめる）。ジョブの `permissions` は `pages: write`、`id-token: write`（と `contents: read`）。デプロイのジョブは `environment: github-pages` と、`needs: <build>`。
- **リポジトリの設定（持ち主がやること）**: Settings > Pages > Build and deployment > Source を **GitHub Actions** にする（文書は「カスタムワークフローを有効にする」と書く。選択肢の名前は私の記憶で、**未確認**）。これは AI が変えられない（CLAUDE.md「判断してほしいこと」のリリースの扱い）。
- 環境の保護: `github-pages` という環境が自動で作られ、デプロイを許すブランチが決まる（既定の中身は**未確認**）。Settings > Environments > github-pages で、「Deployment branches」に `main` だけが入っていることを確かめる。PR のブランチから公開されないようにする。必須のレビュアーは付けても付けなくてもよい（付けると公開のたびに承認が要る）。
- main は GitHub で保護されていて直接 push できない（CLAUDE.md）。公開は main への push で動かし、PR の CI ではビルドと検査だけ（`deploy` のジョブに `if: github.ref == 'refs/heads/main'`）。

### 4.2 URL

- 既定: `https://<アカウント>.github.io/<リポジトリ名>/`。このリポジトリなら `https://stofu5678.github.io/matsu_editor/`（`Cargo.toml` の `repository = "https://github.com/stofu5678/matsu_editor"` から。**推測**。アカウントが変わる、独自ドメインにする、は別の判断）。
- mdBook の `book.toml` に `site-url = "/matsu_editor/"` を書く（404 ページのため）。ページのリンクは相対なので、URL が変わっても壊れにくい。
- 独自ドメインは、持ち主の判断（DNS と HTTPS の設定）。

### 4.3 制限

- 公開するサイトの大きさ 1 GB まで。ソースのリポジトリは 1 GB が目安。
- 帯域 月 100 GB（ゆるい制限）。
- ビルド 1 時間に 10 回（ゆるい制限）。カスタムワークフローで作るなら対象外。
- デプロイは 10 分でタイムアウト。
- 商業の利用（オンラインの商売など）には使わない決まり。MATSU は当たらない（**推測**）。
- Pages に置いた文書は誰でも読める。パスワードなどを入れない。
- 見積もり: サクラの HTML 767 枚で 5.8 MB。MATSU のヘルプは 10〜30 MB に収まる見込み（画像次第。**推測**）。

### 4.4 このリポジトリの CI に対して

- `.github/workflows/ci.yml` の `changes` ジョブ（34 行〜）は、PR の変更ファイルが `\.md$` か `^docs/` だけなら `code=false` にして、macOS・Windows のテストを飛ばす。つまり `docs/` の下か `.md` なら軽く通る。`help/` の下に `.md` 以外（`book.toml`、`.css`、`.hbs`、`.js`、画像）を置くと、**重い全件のテストが走る**。
  - 案 A: `docs/help/` に置く。`^docs/` に当たるので `ci.yml` の変更はいらない。`typos` も同じように通る（`_typos.toml` は `docs/` を除外していない。日本語は通るが、サンプルの英語には注意）。
  - 案 B: 最上位の `help/` に置いて、`changes` の正規表現に `^help/` を足す。
- 新しいジョブが要る: `help`（mdBook のビルド、Pagefind、リンクの検査、サンプルの検査）。必須のチェックにするなら `.claude/handoff/set-required-checks.sh` の流れで登録する（持ち主の作業）。ジョブは Linux だけで、20 分の枠（型 E）に収まる。
- 公開用のワークフロー（`.github/workflows/help.yml` のような名前）は、main への push で動く。リリースごとのパッケージ（`package.yml`）に `help/` のアーカイブを足す場合は、`package.yml` と `package-check.yml` の変更になる。
- `leftover_names.rs` は古い名前（`ume`）の混入を調べる。ヘルプの文章が `ume` を含まなければ関係ない（「ドキュメント」「resume」のような単語の一部は除外される）。

---

## 5. マクロとスクリプトのサンプル

### 5.1 すでにあるもの

- `crates/matsu-plugin/samples/`（16 ファイル）:
  - `sample.js` `sample.mjs`（Node.js / QuickJS）、`sample.ts`（TypeScript）、`sample.py`（Python）、`sample.rb`（Ruby）、`sample.php`（PHP）、`sample.ps1`（PowerShell）、`sample.pl`（Perl）、`sample.el`（Emacs Lisp）、`sample.applescript`（macOS）
  - 同梱の処理系: `sample.upy`（MicroPython）、`sample.mruby`（mruby）
  - データベース: `sample_db.py`、`sample_db.mjs`、`sample_db.php`
  - `samples/plugins/`（プラグインの例: `HelloCommands`、`HelloOutline` ほか）
  - どれも、サクラのマクロの API（`Editor.InsText` など）で書いてある。コメントは英語で、文字列の出力は日本語。
- テスト（動かしていない。ファイルを読んだだけ）:
  - `crates/matsu-plugin/src/tests.rs`: `the_sample_macros_run`（798 行〜、9 言語の `sample.*` を動かす。処理系がなければ飛ばす）、`the_database_sample_macros_run`（829 行〜、`sample_db.*`）
  - `crates/matsu-script/tests/{lua,quickjs,micropython,mruby,plugin_host,commands}.rs`: 同梱の処理系ごとのテスト。`quickjs.rs:359` は `samples/sample.js`、`micropython.rs:332` は `sample.upy`（`include_str!`）、`mruby.rs:513` は `sample.mruby` を読む
  - `crates/matsu-ui/src/plugin_commands/tests.rs` ほか 5 か所が `samples/plugins` を読む
  - `docs/adr/0025-script-languages.md` の記述: CI は Linux x64 だけ `require_interpreters: '1'` で、Node.js・Python・Ruby・PHP がないと失敗する。ほかの OS では処理系がなければ飛ばす
- サクラとの比較（oracle）: `crates/matsu/tests/sakura_oracle/cases/` に `.mac` が 729 件（`sakura-oracle.yml` で動く。実物のサクラの結果と比べる）。これは「キーマクロ / サクラ形式のマクロ」の動く例の宝庫で、ヘルプのサンプルの元にできる。
- キーマクロ（`.mac`）: oracle の `.mac`、`crates/matsu-commands/src/key_macro*`。
- サクラ側のサンプル: `sakura/help/macro/source/sample/`（19 ページ、`SampleGrep.html` など。JScript と PPA の例。写すなら GFDL の対象。5.3 で）。

### 5.2 サンプルを動くまま保つ方法（提案。実行はしていない）

1. **サンプルのファイルは今の `crates/matsu-plugin/samples/` を唯一の置き場にする**。ヘルプのページは、mdBook の `{{#include ../../crates/matsu-plugin/samples/sample.py}}`（行の範囲、`ANCHOR:` 付き）で引く。ヘルプにコピーを置かない。コピーが古くなる問題が起きない。コードを変えると、同じ PR でテストが落ちる。
2. 新しいサンプル（キーマクロ、Lua、複数の言語のレシピ）は、同じ `samples/` に足し、`the_sample_macros_run` の表に行を足す。1 サンプル 1 テスト。
3. ヘルプの「入力 → 出力」の例は、テストの期待値と同じ文字列を使う。ヘルプに書いた出力をテストが比べる（例: 各サンプルの先頭に `# expect:` コメントを置いてテストが読む）。
4. oracle の `.mac` は、サクラが同じ結果を返すことの証拠になるので、ヘルプのキーマクロの例にそのまま使い、「サクラでも同じ結果」と書ける。
5. ヘルプの検査（新しい `crates/matsu/tests/help_check.rs`。`leftover_names.rs` と同じ形）:
   - ヘルプに書いたコマンド名（`F_...`、マクロの関数名）がコードの表（`crates/matsu-commands/src/sakura/functions.rs`）にある
   - ページ内のリンクとアンカーが切れていない（mdBook の出力を調べる、または `lychee` などのリンク検査（未確認）。`include` の失敗は mdBook がビルドで報告する）
   - 見出しの ID を変えたら、コードの中の URL（`help.rs`、将来のダイアログのヘルプボタン）が壊れるので、それを引く URL の一覧を 1 か所（`help.rs`）に置き、検査で見る
6. 処理系がない環境での扱い: 今のテストの扱い（Linux x64 は必須、ほかは飛ばす）を引き継ぐ。ヘルプの CI のジョブは Linux x64 で、ここで全言語の処理系を使える（PowerShell、Perl、Emacs などは Ubuntu のイメージにある。ADR-0025 の記述。AppleScript は macOS だけ）。
7. 処理系の版が変わって出力が変わったときは、サンプルの出力を直す。ヘルプには出力の例を短くして、全部の出力を載せない。

### 5.3 サクラのサンプルを MATSU に直す

- サクラの `sample/*.html`（19 ページ）は JScript と PPA。MATSU の言語では QuickJS（`.mjs`）に当たる。文を写すと GFDL の対象になる。サンプルのコードそのもの（短いプログラム）は著作物かが微妙（**推測**）。確実なのは、同じ関数を使う例を MATSU が自分で書く（上の `samples/` を使う）。
- ADR-0025 の残課題「VBScript から書き換える手引き」: サクラの利用者が持つマクロの多くが `.vbs`。VBScript の関数と MATSU の言語（Python や JavaScript）の対応表を作る。これは MATSU 独自のページ。

---

## 6. 仕事の大きさ

### 6.1 ページの見積もり（おおよそ）

「写す」はサクラの文を GFDL の注記つきで MATSU 向けに直す。「書き直す」はサクラの文を見ずに、MATSU の動きから書く（ライセンスは自分で決める）。「新しい」は MATSU だけの機能。

| まとまり | 写す | 書き直す | 新しい | 計 |
|---|---|---|---|---|
| 概要・基本的な使い方（サクラの 35） | 20 | 15（起動、設定の置き場所、インストール） | | 35 |
| コマンド（メニューごと、サクラの約 320） | 約 250（動きの説明） | 約 70（MATSU で違うもの、印刷、プラグイン、外部ヘルプなど） | | 約 320 |
| 共通設定・タイプ別設定のダイアログ（上の 320 に含まれる 29 + α） | 含む | 灰色の項目の説明を直す | | |
| 仕様等（9） | 6（改行コード、文字コード、ExpandParameter、単語） | 3（コマンドラインオプション、メタ文字列） | | 9 |
| 正規表現（`HLP000089`、1） | | 1（Onigmo + bregonig 互換を MATSU で確かめた内容） | | 1 |
| マクロの使い方・概要（サクラの約 18） | | 18（言語が違うので書き直し） | | 18 |
| マクロの関数（329） | **写さない（権利が未確認）** | 329 を表から作る（名前・引数・戻り値はコードの事実。説明は書き直し） | | 329 |
| マクロのサンプル（19 + MATSU） | | 19（`samples/` から） | 10 | 29 |
| プラグイン（サクラの 10） | | 10（ADR-0026 の作りで） | | 10 |
| MATSU だけの機能（2.4 の表） | | | 約 25 | 25 |
| 「サクラとの違い」「VBScript から書き換える手引き」 | | 2 | | 2 |
| **計** | **約 280** | **約 480** | **約 35** | **約 800** |

- マクロの関数 329 ページは、1 関数 1 ページではなく、グループごとに 1 ページに表で並べると 30 ページほどに減らせる（サクラの 1 関数 1 ページの作りに合わせる必要はない。サクラ相当の「引ける」ことは、検索で足りる。**案**）。この見積もりは 1 関数 1 ページの場合で、まとめると全体は約 500 に減る。
- 「写す」を GFDL にする判断が出ない場合は、全部が「書き直す」になる（作る量は増える）。

### 6.2 並列の分け方（1 PR 約 1 つ）

PR 0 は土台で、先に入れる。そのあとは、メニューや領域ごとに独立に書ける。

| PR | 内容 | ページ数 |
|---|---|---|
| 0 | 土台: `book.toml`、テーマ、`SUMMARY.md` の枠、CI の検査、ライセンスの表示のページ、`samples/` の `include` の例、ページの先頭の注記の型 | 5 |
| 1 | 概要・インストール・起動・`matsu.ini`・サクラとの違い | 15 |
| 2 | 基本的な使い方（選択、矩形、タブバー、入力補完ほか） | 25 |
| 3 | コマンド: ファイル | 27 |
| 4 | コマンド: 編集（元に戻す〜削除系） | 40 |
| 5 | コマンド: 編集の続きと変換 | 40 |
| 6 | コマンド: 検索（Grep、タグジャンプ、差分、アウトライン） | 45 |
| 7 | コマンド: ツール（キーマクロ、外部コマンド、データベース） | 15 |
| 8 | コマンド: 設定（共通設定 21 ページ） | 40 |
| 9 | コマンド: 設定（タイプ別設定ほか）とウィンドウ | 55 |
| 10 | コマンド: ヘルプとメニューに入ってないコマンド | 40 |
| 11 | 仕様等（改行、文字コード、ExpandParameter、コマンドラインオプション）と正規表現 | 10 |
| 12 | マクロ: 使い方、キーマクロ、サンプル（言語ごと）、VBScript の書き換えの手引き | 30 |
| 13〜16 | マクロの関数（カーソル・選択・ブロック、編集・変更・挿入・クリップボード、ファイル・ウィンドウ・モード・設定、検索・カスタム・その他・サポート）: 表を作るスクリプトで 4 つに分ける | 各 80 ページ前後（表にまとめれば各 8 ページ） |
| 17 | プラグイン（ADR-0026）とスクリプトの言語 | 15 |
| 18 | MATSU だけの機能（シェル、プレビュー、巨大ファイル、自動保存、データベース、エンコード・デコード） | 20 |
| 19 | アプリの配線: F1 などのヘルプのコマンド、ダイアログのヘルプボタン、パッケージへの同梱（下の ADR 案） | コード |
| 20 | 公開のワークフロー（持ち主が Pages の設定を変えたあと） | CI |

- 1〜12 と 13〜18 は、PR 0 の後、互いに独立。コマンド章の PR は、同じメニュー内の依存がない。
- 変換の自動化: サクラの HTML から Markdown への変換スクリプトを `tools/` に置くと、「写す」の PR が速い（写したあと MATSU の動きで直すところに時間を使う）。変換の元は Shift_JIS でなく UTF-8（BOM 付き）の HTML（`HLP000100.html` の `charset=UTF-8` を見た）。
- 担当（エージェント）が、サクラの動きを MATSU で確かめて書く。ヘルプの内容の正しさは、コードとテストで確かめる（5.2）。

### 6.3 まだ確かめていないこと

- サクラのヘルプの全ページの内容（文の量、画像の依存）は、章の構成と数を調べただけで、中身は読んでいない。
- GFDL の使い方の法的な確認（弁護士ではない）。
- マクロのヘルプの元の S_MACPPA.KHP の条件。
- Pagefind の `file://` の動き、順位の質、ビルドの時間（大きなサイトで）。
- Hugo・MkDocs・Jekyll・Docusaurus の実測（入れていない）。
- GitHub Pages の環境の既定の保護の中身。
- ページの `#アンカー` が `open_with_system` で渡るか。
