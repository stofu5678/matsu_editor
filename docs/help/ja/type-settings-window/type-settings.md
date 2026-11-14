# タイプ別設定...

文書のタイプ（種類）ごとの設定を変えるダイアログを開きます。タイプは「基本」「C/C++」「HTML」のように、ファイルの種類を分ける単位です。色、折り返し、タブ幅、新しい文書の文字コードなどを、タイプごとに決められます。

| 項目 | 内容 |
|---|---|
| メニュー | 設定 > タイプ別設定... |
| 機能名 | `F_OPTION_TYPE` |
| 既定のキー | `Ctrl+5` |
| マクロ | `OptionType` |
| キーマクロに記録 | しない |

いまの文書のタイプの設定が開きます。ほかのタイプの設定を開くには、[タイプ別設定一覧](type-list.md)の「設定変更」を使います。

## ダイアログ

ダイアログには、次の 6 つのページがあります。ページの並びはこのとおりです。

| ページ | 何を決めるか |
|---|---|
| [スクリーン](type-page-screen.md) | 名前と拡張子、折り返し、タブ幅、インデント、アウトライン解析、フォント、禁則処理 |
| [カラー](type-page-color.md) | 色と文字飾り、強調キーワードの選択、コメントと文字列の書き方、縦線 |
| [ウィンドウ](type-page-window.md) | 新しい文書の文字コードと改行コード、IME、行番号、背景画像 |
| [支援](type-page-support.md) | 入力補完、外部ヘルプ、保存時の警告、インデントの検出 |
| [正規表現キーワード](type-page-regex.md) | 正規表現で色を付ける言葉の一覧 |
| [キーワードヘルプ](type-page-keyhelp.md) | キーワードの意味を引く辞書の一覧 |

- 開いたとき、最後に使ったページが前に出ます。初めてのときは「スクリーン」です。
- 「OK」で設定を保存し、すべてのウィンドウにすぐ反映します。変更がなければ何も書きません。「キャンセル」と `Esc` では、何も変わりません。
- ダイアログは、同時に 1 つしか開けません。開いているときにもう一度使うと、そのダイアログが前に出ます。別のタイプの設定を開こうとすると、開いている方を（何も保存せずに）閉じて、頼まれた方を開きます。
- ダイアログは、本文のウィンドウとは別のウィンドウです。開いたままでも、本文のウィンドウを使えます。

## ページを直接開く機能

メニューに載らない次の機能は、タイプ別設定を、決まったページで開きます。いまの文書のタイプが対象です。

| 機能名 | 開くページ |
|---|---|
| `F_TYPE_SCREEN` | スクリーン |
| `F_TYPE_COLOR` | カラー |
| `F_TYPE_WINDOW` | ウィンドウ |
| `F_TYPE_HELPER` | 支援 |
| `F_TYPE_REGEX_KEYWORD` | 正規表現キーワード |
| `F_TYPE_KEYHELP` | キーワードヘルプ |

## 文書のタイプの決まり方

- ファイルを開くとき、タイプの一覧を先頭（基本）から順に調べ、ファイル名が拡張子の欄に合う最初のタイプにします。どれにも合わなければ「基本」です。「テキスト」は他のタイプより前にあるので、`.log` は、TeX に書かれていても「テキスト」になります。
- 拡張子の欄は、空白・`;`・`,` で区切ります。ワイルドカードのない項目（`txt`、`Makefile`）は、ファイル名全体か、最後の `.` より後ろに一致します。`*` と `?` を含む項目（`*.txt`）は、ファイル名全体に当てはめます。大文字と小文字は区別しません。
- 「名前を付けて保存」でファイル名が変わると、タイプも新しい名前から選び直します。ただし、[タイプ別設定一覧](type-list.md)の「一時適用」やマクロの `ChangeType` で選んだタイプは、選び直しません（次にファイルを読み込むまで）。

## 組み込みのタイプ

サクラエディタと同じ次の 17 のタイプが、この順で最初からあります。番号と中身は、サクラエディタと同じです。

基本、テキスト、C/C++、HTML、PL/SQL、COBOL、Java、アセンブラ、AWK、MS-DOSバッチファイル、Pascal、TeX、Perl、Python、Visual Basic、リッチテキスト、設定ファイル

MATSU Editor は、サクラエディタにない次のタイプを足しています。設定ファイルが自分のタイプに使っていない 17 番より後の番号に、この順で入ります（空きが足りなければ、入らないタイプがあります）。

JavaScript、TypeScript、JSON、CSS、Markdown、Rust、Go、Ruby、シェルスクリプト、YAML、SQL、MySQL、PostgreSQL、SQL Server、SQLite、BigQuery

また、Python のタイプには、組み込み関数（`print` など）を 2 番目の強調キーワードのセットとして足しています。

## 設定ファイルでの置き場所

タイプの設定は、[設定ファイル matsu.ini](../intro/settings-file.md) の `[Types(N)]` に、サクラエディタと同じキーの名前で保存します。`N` は、タイプの一覧での番号（0 が基本）です。タイプの数は `[Other]` の `nTypesCount` です。ページごとのキーは、各ページの表にあります。

次の数の組は、`nInts` という 1 つのキーに、この順で入っています。

`nInts` = 番号, 折り返し桁数, 文字の間隔, TAB幅, 強調キーワード1, 強調キーワード2, 文字列エスケープ, 行番号の単位, 行番号区切り, 英文ワードラップ, 印刷設定の番号, TSVモード

OK で保存するとき、設定ファイルは排他ロックの下で書き換えます。ほかのセクションや、ほかのウィンドウが同時に変えた内容は、消えません。

## 使えない部品

MATSU Editor が使わない機能の部品は、灰色で表示します。ファイルの値は見せますが、書き換えません。灰色の部品は、このダイアログでは 3 つだけで、いずれも「ウィンドウ」ページにあります（IME の 2 つと、文書アイコン。Windows 以外のとき）。

## サクラエディタとの違い

- 保存するファイルは、サクラエディタの `sakura.ini` ではなく `matsu.ini` です。
- ダイアログはモーダルではありません。GPUI にモーダルの子ウィンドウがないためです。
- ダイアログのページには、MATSU Editor が使うプラグインの項目（スマートインデント、アウトライン解析、入力補完）が、サクラエディタと同じように並びます。プラグインが使えなくなっても、設定ファイルの値は変えず、「（使えないプラグイン N）」と表示します。サクラエディタは、一覧の最初の項目を見せて、OK で 0 を書きます。

## 根拠

- サクラのソース: `sakura_core/typeprop/CPropTypes.cpp`（`DoPropertySheet`、`TypePropSheetInfoList`）、`sakura_core/CEditApp.cpp` の `OpenPropertySheetTypes`、`sakura_core/cmd/CViewCommander_Settings.cpp` の `Command_OPTION_TYPE`、`sakura_core/env/CDocTypeManager.cpp` の `GetDocumentTypeOfPath`、`sakura_core/env/CShareData_IO.cpp` の `ShareData_IO_Type_One`
- MATSU のコード: `crates/matsu-ui/src/type_settings/mod.rs`（`open`、`commit`、`PAGE_TITLES`、`page`）、`crates/matsu-ui/src/ui_commands/settings/option_type.rs`、`type_screen.rs` ほか `type_*.rs`、`crates/matsu-config/src/settings/doc_types.rs`（`type_index_for_file_name`）、`crates/matsu-config/src/settings/types.rs`（`read_over`）、`crates/matsu-config/src/settings/builtin_types.rs`、`crates/matsu-config/src/settings/extra_types.rs`、`crates/matsu-ui/src/doc_types/mod.rs`（`follow_file_name`）
- 試験: `crates/matsu-ui/src/type_settings/tests.rs`、`crates/matsu-config/src/settings/doc_types.rs` と `extra_types.rs` の試験
- `docs/adr/0017-extra-document-types.md`（MATSU 独自のタイプ）、`docs/sakura-gaps.md` の「タイプ別設定」の行
