# データベース

編集中の SQL を、データベースのコマンドラインのクライアント（`psql`、`mysql`、`sqlcmd`、`sqlite3`、`sqlplus`、`bq` など）に実行させ、出力をアウトプットのウィンドウに出します。

MATSU Editor がデータベースにつなぐのではありません。利用者が `databases.ini` に書いたコマンドを、外部コマンド実行と同じように動かします。パスワードは、MATSU Editor が預かりません。

## 機能

| メニュー（「ツール」） | 機能名 | マクロ | 既定のキー |
|---|---|---|---|
| データベースで実行 | `F_PLSQL_COMPILE_ON_SQLPLUS` | `ExecSQLPLUS()` | `F10` |
| データベースの出力を表示 | `F_ACTIVATE_SQLPLUS` | `ActivateSQLPLUS()` | `Shift+F11` |
| 接続を選んでデータベースで実行... | `F_PLSQL_COMPILE_ON_DATABASE_CHOICE` | `ExecDatabaseChoice()` | なし |

最初の 2 つは、サクラの Oracle 用の機能（SQL*Plus で実行、SQL*Plus をアクティブ表示）に当たります。機能名とマクロ名、キーはサクラと同じで、メニューの文言だけをデータベース向けに変えています。3 つ目は MATSU Editor だけの機能です。

## 接続を決める: databases.ini

接続の設定は、`matsu.ini` と同じフォルダーの `databases.ini` に書きます。

```ini
[Database]
Default=work

[Connection.work]
Kind=postgresql
Command=psql -h db.example.com -d app -f $F
Encoding=utf-8
Folder=
```

| 項目 | 意味 |
|---|---|
| `[Database]` の `Default` | 「データベースで実行」（`F10`）が使う接続の名前 |
| `Kind` | データベースの種類。`oracle`、`mysql`、`postgresql`、`sqlserver`、`sqlite`、`bigquery`、`other`。エラーの行を探す書き方とパスワードの検査を選ぶだけで、動かすコマンドは `Command` が決めます |
| `Command` | 動かすコマンドライン（雛形） |
| `Encoding` | クライアントの出力の文字コード。`auto`（既定）、`utf-8`、`shift_jis`、`utf-16` |
| `InputEncoding` | 標準入力に送る文字の文字コード。省略すると `Encoding` と同じ（`auto` のときは UTF-8） |
| `Folder` | コマンドを動かすフォルダー。空なら文書のフォルダー |

- 雛形の中の `$F` は、文書のファイル名になります。MATSU Editor が、OS のシェルに合わせて引用符で囲むので、そのまま `-f $F` のように書きます。`$` 自体は `$$` と書きます。
- 雛形に `$F` がないときは、文書の文字列を標準入力に送ります。選んでいる範囲があればそれを、なければ文書全体を送ります。
- `Encoding=auto` のときは、出力を 1 行ずつ、UTF-8 として読めれば UTF-8、読めなければ Shift_JIS（CP932）として読みます。

ファイルがまだないときは、機能を実行すると、「例の入ったファイルを作りますか？」と聞きます。「作る」を選ぶと、6 種類のデータベースの例を書いたファイルができます。例は、実物のクライアントで確かめていません。ホスト名やデータベース名を、自分のものに直してください。作った直後は `Default` が空です。使う接続の名前を書いてください。

### パスワード

パスワードを、`databases.ini` に書かないでください。雛形にパスワードが書かれているように見えると、MATSU Editor は実行しません（`-p…`、`--password=`、`PGPASSWORD=`、`user:pass@` の形などを調べます。すべては見つけられません）。

クライアントが持っている仕組みを使ってください。たとえば `~/.pgpass`、`mysql_config_editor`、Oracle Wallet、`gcloud auth`、Windows の統合認証です。

## データベースで実行（F10）

1. `databases.ini` を読みます。`Default` の接続がないときは、設定の書き方を案内し、「接続を選んでデータベースで実行...」も使えることを知らせます。
2. 雛形に `$F` があるときは、文書がファイルになっている必要があります。
   - 変更があるときは、「保存してから実行しますか」と聞きます。「はい」で保存します。「いいえ」とキャンセルでは、実行しません。
   - 変更のあるファイルなしの文書で「はい」を選ぶと、保存の画面が出ます。変更のない、ファイルなしの文書は、「SQLをファイルに保存しないとデータベースで実行できません。」と知らせて、実行しません。
3. コマンドを動かし、出力をアウトプットのウィンドウに出します。

雛形に `$F` がないときは、手順 2 は要りません。文書を保存しなくても、文字列を送って実行します。

## 接続を選んで実行

「接続を選んでデータベースで実行...」を選ぶと、`databases.ini` の接続の一覧（名前、種類、コマンド）が出ます。1 つ選んで OK を押すと、その接続で、`F10` と同じ流れで実行します。

- 接続が 1 つしかなくても、一覧を出します。
- `Default` は変わりません。
- 一覧では、`↑` `↓` `Home` `End` で選び、`Enter` で OK、`Esc` でキャンセルです。行をダブルクリックしても OK になります。

## データベースの出力を表示（Shift+F11）

アウトプットのウィンドウを表示します。サクラの機能は、動いている SQL*Plus のウィンドウを前に出しますが、MATSU Editor が動かすクライアントには、自分のウィンドウがないので、出力を見せるウィンドウを出します。

## 出力のエラー行から移る

アウトプットに出たエラーの行に、SQL の行番号が書かれているときは、その行をクリックして、文書の該当の行へ移れます。クライアントごとの書き方を探します。

| 種類 | 探す書き方の例 |
|---|---|
| PostgreSQL | `psql:ファイル:3: ERROR: ...` |
| MySQL、MariaDB | `ERROR 1064 (42000) at line 3: ...` |
| SQL Server | `Msg 102, Level 15, State 1, Server S, Line 3` |
| SQLite | `Parse error near line 3: ...` |
| BigQuery | `... at [3:5]` |
| Oracle | `ERROR at line 3:`、`3/5 PLS-00103: ...` |
| どれでも | `ファイル:3: メッセージ` |

次のときは、行をクリックしても移りません。

- ファイルのない文書から実行したとき
- 雛形に `$F` がなく、箱型の選択を送ったとき（出力の位置が、文書の位置と合わないため）

雛形に `$F` がなく、選択を送ったときは、行番号を、選択の先頭から数えて文書に直します。

実物のクライアントでは、まだ確かめていません。SQL Server と Oracle は、行番号の数え方がクライアント側の区切り（バッチ、文）ごとなので、文書の行とずれることがあります。

アウトプットに出たものは、データベースの中身など、他人が書いた文字列であることがあります。ネットワーク上の場所（`\\ホスト名\共有` など）と、Windows の装置の名前（`CON`、`NUL` など）は、ファイルの場所として扱いません。

## 色分け

SQL には、色分けのタイプがあります。拡張子 `.sql` は汎用の「SQL」です。MySQL（MariaDB）、PostgreSQL、SQL Server、SQLite、BigQuery の方言のタイプは、タイプの一覧から選ぶか、専用の拡張子（`mysql`、`pgsql`、`psql`、`tsql`、`sqlite3`、`bqsql`）で選びます。サクラの「PL/SQL」は、そのまま残っています。

## サクラとの違い

- サクラは、動いている Windows 版の SQL*Plus（`sqlplusw.exe`）のウィンドウを探し、`@ファイル名` を貼り付けます。MATSU Editor は、コマンドラインのクライアントを動かすので、Windows、macOS、Linux のどこでも使えます。
- サクラには、Oracle 用の機能しかありません。MATSU Editor は、Oracle、MySQL、PostgreSQL、SQL Server、SQLite、BigQuery の雛形を持ち、どのクライアントも呼べます。
- 接続を選んで実行する機能は、サクラにありません。
- 結果は文字の出力です。表にはなりません。

## 根拠

- 機能の定義とメニューの文言: `xtask/src/matsu_extras.rs` の `RELABELS` と `F_PLSQL_COMPILE_ON_DATABASE_CHOICE`、`crates/matsu-commands/src/sakura/macro_functions.rs`、`crates/matsu-commands/src/sakura/key_bindings.rs`（`F10`、`F11`）
- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_PLSQL_COMPILE_ON_SQLPLUS` と `Command_ACTIVATE_SQLPLUS`
- 接続の設定: `crates/matsu-config/src/databases.rs`（`Connection`、`Encoding`、`Kind`、`examples`、`password_in_command`、`create_example_file`）
- 実行の流れ: `crates/matsu-ui/src/database/run.rs`、`crates/matsu-ui/src/database/command.rs`、`crates/matsu-ui/src/database/pick.rs`
- 出力を表示: `crates/matsu-ui/src/ui_commands/file/activate_sqlplus.rs`
- エラー行: `crates/matsu-core/src/sql_error.rs`
- 試験: `crates/matsu-ui/src/database/tests.rs`、`crates/matsu-config/src/databases/tests.rs`
- 決めたこと: `docs/adr/0023-database.md`
