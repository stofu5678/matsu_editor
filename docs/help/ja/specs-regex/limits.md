# 上限と制限

MATSU Editor のコードが決めている、数や大きさの上限です。多くは、サクラエディタの上限に合わせた値です。文字数は、書いてあるものだけ UTF-16 の単位で数えます。

## 大きさに上限がないもの

- ファイルの大きさ: 上限はありません。実際の上限はメモリです（ADR-0012）。メモリに載らないファイルは、読み込みの失敗になります。`usize` が 32 ビットの環境では、4 GiB を超えるファイルは開けません。巨大なファイルを範囲ごとに読む方法は、「MATSU Editor だけの機能」の章にあります。
- 正規表現の照合: 件数にも、スタックにも、上限はありません（[検索の仕組みと上限](search-rules.md)）。

## 文書タイプと表示

| 項目 | 上限 |
|---|---|
| 文書タイプの数 | 60 |
| 文書タイプの名前、拡張子の並び | それぞれ 63 文字まで |
| 折り返し桁数 | 10 〜 10240 |
| TAB の幅 | 64 まで |
| 字間 | 0 〜 64 |
| 行間 | -128 〜 128 |
| 行番号の幅 | 2 〜 11 |
| 縦線（指定桁の線） | 200 本 |
| 正規表現キーワード | 100 個 |
| キーワードセット | 100 組。名前は 32 文字、1 語は 63 文字まで |
| すべてのキーワードの合計 | 15000 語 |
| インデントの判定に見る行数 | 先頭の 256 行 |

## 履歴と一覧

| 項目 | 上限 |
|---|---|
| 最近使ったファイル | 36 |
| 最近使ったフォルダー | 36 |
| 除外するファイル（履歴に残さない） | 36 |
| 検索条件、置換後の文字列 | それぞれ 30（1 件は 260 文字より短いもの） |
| Grep のファイル、フォルダー、除外ファイル、除外フォルダー | それぞれ 30（1 件は 512 文字より短いもの） |
| タグジャンプのキーワード | 30 |
| 外部コマンドの履歴、フォルダーの履歴 | それぞれ 32（コマンドは 1024 文字、フォルダーは 260 文字より短いもの） |
| 移動履歴（`F_JUMPHIST_*`） | 30 |
| ワンライナーの履歴 | 20 |

## 登録できるもの

| 項目 | 上限 |
|---|---|
| 登録マクロ | 50。名前は 63 文字まで、ファイルのパスは 260 文字まで |
| ワンライナー | 50。名前は 63 文字まで、コマンドは 1023 文字まで |
| カスタムメニュー | 25（右クリックメニューを含む）。名前は 32 文字まで、1 つに 48 項目まで |
| メインメニュー | 全体で 500 行、メニューバーの最上位は 20、名前は 40 文字まで |
| プラグイン | 40。ID は 63 文字まで |
| プラグインのコマンド、オプション | 1 つの接続点につき 49 まで、オプションは 99 まで |
| 印刷設定 | 8 組。名前は 32 文字まで、フォント名は 31 文字まで。ヘッダーとフッターの 1 つは 99 文字まで |
| ファイルツリーの項目 | 設定に 20、プロジェクトファイルから 10000 |

## 実行と補完

| 項目 | 上限 |
|---|---|
| 外部コマンド実行 | 展開後のコマンドは UTF-16 の 1024 単位まで |
| 入力補完の候補 | 文書から 1024 個まで。1 語は 1020 文字まで |
| マクロの出力パネル | 5000 行（古い行から消える）。描くのは最後の 300 行 |
| シェルのタブのスクロール | 既定 5000 行、最大 50000 行 |
| 差分の比較 | 片方が、巨大ファイルの設定の大きさと 256 MiB の、小さい方まで |
| 背景画像 | 縦横とも 8192 ピクセルまで。ファイルは 256 MiB まで |
| 1 回の起動の引き渡し | 4 MiB まで（2 つ目の起動が、動いているプロセスに渡す内容） |

## 根拠

- MATSU のコード（定数の定義。ファイルと定数の名前）:
  - `crates/matsu-config/src/settings/types.rs`: `MAX_TYPES`、`MAX_LINE_KETAS`、`MIN_LINE_KETAS`、`COLUMN_SPACE_MAX`、`LINE_SPACE_MAX`、`LINE_NUM_WIDTH_RANGE`、`MAX_TYPES_NAME`、`MAX_TYPES_EXTS`
  - `crates/matsu-config/src/settings/type_rules.rs`: `MAX_VERT_LINES`、`MAX_REGEX_KEYWORDS`
  - `crates/matsu-config/src/settings/keywords.rs`、`crates/matsu-ui/src/common_settings/pages/keyword/entry.rs`: キーワード
  - `crates/matsu-core/src/indent_style.rs`: `TAB_WIDTH_MAX`、`MAX_LINES_TO_CHECK`
  - `crates/matsu-config/src/history/mod.rs`、`settings/search_keys.rs`、`settings/exec_history.rs`、`crates/matsu-core/src/jump_history.rs`、`settings/oneliner.rs`: 履歴
  - `crates/matsu-config/src/settings/macros.rs`、`oneliner.rs`、`custom_menu.rs`、`main_menu.rs`、`plugins.rs`、`plugins/def.rs`、`print.rs`、`crates/matsu-print/src/setting.rs`、`settings/file_tree.rs`: 登録できるもの
  - `crates/matsu-core/src/exec.rs`、`crates/matsu-commands/src/text/complete.rs`、`crates/matsu-ui/src/macros/output.rs`、`crates/matsu-config/src/terminal.rs`、`crates/matsu-ui/src/diff/run.rs`、`back_image/mod.rs`、`crates/matsu-platform/src/instance/request.rs`: 実行と補完
- ADR: `docs/adr/0012-file-size-limit.md`、`docs/adr/0020-huge-files.md`

名前や文字列の長さは、「〜より短い」とコードに書いてあるものを、「〜文字まで」に直して書いています。
