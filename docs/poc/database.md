# データベースの操作: 調べたこと

[ADR-0023](../adr/0023-database.md) の下調べ。2026-10-24 に調べた。コードは書いていない。出典のない事実は、調整役が確かめていない一般的な知識で、「未確認」と付けた。

## 1. サクラの 2 つの機能は、何をするか

出典はサクラのソース（zlib License）。

| 機能 | ID（マクロ名） | 既定のキー | ソース |
|---|---|---|---|
| SQL*Plusで実行 | `F_PLSQL_COMPILE_ON_SQLPLUS`（`ExecSQLPLUS`） | F10（`func/CKeyBind.cpp` の `_SQL_RUN`） | `cmd/CViewCommander_File.cpp` の `Command_PLSQL_COMPILE_ON_SQLPLUS` |
| SQL*Plusをアクティブ表示 | `F_ACTIVATE_SQLPLUS`（`ActivateSQLPLUS`） | Shift+F11 | 同じファイルの `Command_ACTIVATE_SQLPLUS` |

「SQL*Plusで実行」の手順:

1. `FindWindow(L"SqlplusWClass", L"Oracle SQL*Plus")` で、起動中の SQL*Plus のウィンドウを探す。なければ「起動されていません」と出して終わる
2. 文書が変更されていれば、保存するか確かめる（はい: 保存、いいえ・キャンセル: 中止）
3. 文書にファイル名がなければ、ビープ音とエラーで終わる（メニューの有効・無効もファイル名の有無で決まる。`func/Funccode.cpp` の `IsFuncEnable`）
4. クリップボードに `@<ファイルのパス>\r\n` を入れる（パスに空白があれば `@"<パス>"`）
5. そのウィンドウを前面に出し、`WM_COMMAND`（コマンド ID 201。SQL*Plus のメニューの「貼り付け」とみられるが、ソースに名前はなく未確認）を送る。3 秒で応答がなければエラー

つまり、サクラは SQL*Plus を子プロセスとして動かさない。利用者が先に起動した Windows の GUI 版 SQL*Plus（`sqlplusw.exe`。クラス名 `SqlplusWClass`）に、ファイル名を貼り付けさせている。結果は SQL*Plus のウィンドウに出て、エディタには戻らない。

ヘルプ（`help/sakura/res/HLP000027.html`）の動作確認の版は「SQL*Plus: Release 3.3.2.0.2」で、GUI 版の古い版である。

「アクティブ表示」は、同じウィンドウを探して前面に出すだけ（なければ「起動されていません」）。

### 今も動くか

今の配布物だけでは揃わない。Oracle は 11g から、Windows の GUI 版 SQL*Plus（`sqlplusw.exe`）を同梱しなくなったとされる。コマンドライン版の `sqlplus.exe` だけが残る。出典は、Oracle の公式な廃止告知ではなく、利用者の報告とベンダーの記事である（11g から、という線は複数で一致するが、公式の文書では確かめていない）:

- [Oracle Forums: sqlplusw.exe not supplied with 11g](https://forums.oracle.com/ords/apexds/post/sqlplusw-exe-not-supplied-with-11g-1481)
- [Quest (Toad) KB: 「SQL*Plus で実行」が 11g のクライアントで動かない](https://support.quest.com/toad-for-oracle/kb/4228344/the-execute-in-sql-plus-option-is-not-working-with-an-oracle-11g-client)

さらに、`FindWindow` と `SendMessage` は Windows だけの仕組みで、macOS と Linux には当てはめられない。今の MATSU の F10 は「まだ使えない」と出る（`crates/matsu-ui/src/view/tests.rs`）。

サクラは、ウィンドウがあるかだけを見るので、古い GUI 版を別に手に入れた利用者が今の Oracle につなげるかは、確かめていない。ただ、そのまま写しても、現在の配布物だけでは使えず、macOS と Linux では成り立たない。必要なのは、「いま編集中のファイルをデータベースのクライアントに実行させる」という意図を、現在のクライアントで実現することである。

## 2. データベースごとの比較

### 2.1 コマンドラインのクライアント

| DB | クライアント | ライセンス | 入れ方 | ファイルの渡し方 | エラーの行番号 | 認証 |
|---|---|---|---|---|---|---|
| Oracle | SQL*Plus（Oracle のサイトで配布） | Oracle の独自の条件（未確認） | Windows・macOS・Linux 用の配布物（未確認） | `@file`、標準入力 | PL/SQL のコンパイルエラーは `SHOW ERRORS` で行・桁が出る（未確認） | ユーザー名/パスワード、Oracle Wallet、OS 認証（未確認） |
| Oracle | SQLcl | Oracle Free Use Terms and Conditions（[Oracle のブログ](https://blogs.oracle.com/database/post/sqlcl-now-under-the-oracle-free-use-terms-and-conditions-license)）。Java が要る（版は未確認） | zip を展開（未確認） | `@file` | SQL*Plus と同様（未確認） | Wallet など（未確認） |
| MySQL / MariaDB | `mysql`（MariaDB は `mariadb`） | MySQL は GPL-2.0（[mysql-server の LICENSE](https://raw.githubusercontent.com/mysql/mysql-server/trunk/LICENSE)）。MariaDB のサーバーは GPL-2（[MariaDB](https://mariadb.com/kb/en/mariadb-license/)）。クライアントのライブラリは LGPL-2.1 以降（同ページ） | 各 OS のパッケージ、Homebrew など（未確認） | `mysql < file`、`source file` | `ERROR 1064 ... at line N` の形（未確認） | `mysql_config_editor`（`--login-path`）、`~/.my.cnf`（未確認） |
| PostgreSQL | `psql` | PostgreSQL License（[出典](https://www.postgresql.org/about/licence/)。ページは psql を名指ししていない） | 各 OS のパッケージ、Windows 用インストーラー（未確認） | `psql -f file`、標準入力 | `psql:file:LINE: ERROR: ...` の形（未確認） | `~/.pgpass`（Windows は `%APPDATA%\postgresql\pgpass.conf`）、サービスファイル（未確認） |
| SQL Server | `sqlcmd`（Go 版） | MIT（[microsoft/go-sqlcmd](https://github.com/microsoft/go-sqlcmd)） | Windows: `winget install sqlcmd`。macOS: `brew install sqlcmd`。Linux: apt・yum・zypper、Linuxbrew（同ページ） | `-i file`、標準入力（同ページ） | 同ページに記述なし（未確認） | SQL 認証、Windows 統合認証（`-U` を付けない）、Entra ID（`-G`、`--authentication-method`）（同ページ） |
| SQLite | `sqlite3` | パブリックドメイン（[sqlite.org](https://sqlite.org/copyright.html)） | 多くの OS に入っている。Windows は zip を展開（未確認） | `sqlite3 db < file`、`.read file` | `Parse error near line N` の形（未確認） | なし（ファイルへのアクセス権のみ） |
| BigQuery | `bq`（Google Cloud CLI の一部。Python 製） | 未確認 | Google Cloud CLI のインストーラー（未確認） | 標準入力（未確認。[公式ページ](https://docs.cloud.google.com/bigquery/docs/bq-command-line-tool)の例は SQL を引数で渡す） | 未確認 | Google Cloud へのサインインが前提（同ページ）。`gcloud auth login`、サービスアカウント（未確認） |

注意:

- GPL のクライアントを外部のプログラムとして動かすだけなら、MATSU のライセンスには影響しないと考える（取り込まず、同梱もしない）。CLAUDE.md の禁止は「依存」と「コードの写し」についてである。この解釈は、ユーザーの確認を求める（ADR の「決めてほしいこと」）
- 行番号の形式の「未確認」は、実機で確かめてから、エラーの行へ移る正規表現を決める

### 2.2 Rust のドライバー（MATSU が直接つなぐ場合）

crates.io の API で、最新版とライセンスを確かめた（2026-10-24）。

| DB | クレート | 最新版 | ライセンス | 必要なもの・注意 |
|---|---|---|---|---|
| PostgreSQL・MySQL・SQLite | `sqlx` | 0.9.0 | MIT OR Apache-2.0 | 機能で DB を選ぶ。非同期が前提。TLS の選択がある（未確認） |
| PostgreSQL | `tokio-postgres` | 0.7.18 | MIT OR Apache-2.0 | 非同期。TLS は別のクレート（未確認） |
| MySQL | `mysql_async` | 0.37.1 | MIT OR Apache-2.0 | 非同期 |
| SQL Server | `tiberius` | 0.13.0 | MIT/Apache-2.0 | 非同期 |
| SQLite | `rusqlite` | 0.40.2 | MIT | 同梱の SQLite をビルドする機能がある。C コンパイラが要る（未確認） |
| Oracle | `oracle` | 0.6.3 | UPL-1.0/Apache-2.0 | Oracle のクライアントライブラリ（11.2 以降）が実行時に要る（[README](https://github.com/kubo/rust-oracle)）。最後の更新は 2025-01-02。`UPL-1.0` は `deny.toml` の一覧にない。Apache-2.0 の側を選べるが、cargo-deny が「/」をどう扱うかは確かめていない |
| BigQuery | `gcp-bigquery-client` | 0.28.0 | MIT OR Apache-2.0 | REST API を呼ぶ。認証は Google の OAuth。HTTP と TLS の依存が入る（未確認） |

どれも、クレート自身のライセンスは GPL 系ではない。ただし、依存の先（TLS、暗号、HTTP）は調べていない。実装の前に `cargo deny check` で確かめる。バイナリの大きさは測っていない。

直接つなぐと、非同期のランタイム、TLS、各ドライバーが依存に入り、資格情報の扱いと依存の脆弱性の面倒が増える。

## 3. 案

| | A: クライアントを動かす（サクラの 2 機能を一般化） | B: 内蔵のクライアント | C: マクロとプラグイン |
|---|---|---|---|
| 利用者が得るもの | F10 で、いまのファイルを選んだ DB に実行し、結果を出力パネルで見る。エラーの行へ移れる | 接続の一覧、クエリ、結果の表、キャンセル | スクリプトで自由に（Python の各ドライバーなど）。結果の見せ方は自分で作る |
| 作業 | 小さい。外部コマンド実行（`crates/matsu-ui/src/exec/`）に、接続ごとの雛形、保存の確認、エラー行のジャンプを足す | 大きい。ドライバーの依存、非同期、結果の表（非エディタのタブ。ADR-0024 として別に下書き中で、まだ決まっていない）、資格情報の保管、キャンセル、巨大な結果の分割読み | ほぼない。サンプルと、出力パネルに書く API の確認 |
| 依存 | なし | 多い（2.2） | なし |
| 資格情報 | クライアントが持つ。MATSU は預からない | MATSU が預かる（OS のキーチェーン） | スクリプトとその環境が持つ |
| 危険 | 実行するのは利用者が書いた雛形。ファイル名の引用を誤ると危ない | 資格情報、TLS、依存の脆弱性。`/security-review` の対象が広い | ほぼない |
| 欠点 | クライアントを利用者が入れる。結果は文字で、表ではない | 開発と保守が重い。サクラにない機能 | 利用者の手間が大きい |

組み合わせと順番:

1. A を先にやる。サクラの 2 つの機能の意図を、今のクライアントで満たせる
2. C は、サンプルのマクロとして A と並べて載せる（費用がほぼない）
3. B は、A を使ってみて、表の結果が欲しいとユーザーが決めてから、別の ADR で扱う

## 4. 接続の設定の置き場所

- 接続の設定は利用者のデータで、`matsu.ini` の形式と置き場所に関わる（ADR-0010）。ADR-0010 は、サクラにない項目を「サクラが使わない節か別のファイル」に置くことを認めている。この案は別のファイルを選ぶもので、`matsu.ini` の中の独自の節でも足りるかもしれない
- 案: `matsu.ini` の隣に、別のファイル `databases.ini` を置く。接続ごとに、種類、雛形（例: `psql -h db.example.com -d app -f $F`）、文字コード、作業フォルダーを書く
- パスワードは書かない。MATSU は、パスワードを平文でファイルに書かない。認証は、2.1 の各クライアントの仕組み（`~/.pgpass`、`mysql_config_editor`、Wallet、`gcloud auth` など）に任せる。雛形にパスワードを直接書くことは、画面で止める（プロセス一覧とコマンド履歴に残るため）
- 雛形で使える変数は、外部コマンド実行の `$F` などに合わせる

## 5. 色分け

サクラのタイプは `PL/SQL` の 1 つだけ（`sakura_core/types/CType_Sql.cpp`。拡張子 `sql,plsql`、文字列のエスケープは `''` と `""` の重ね書き）。方言ごとに必要になるもの（未確認。実装の前に各方言の文書で確かめる）:

- コメント: `--` と `/* */` が共通。MySQL は `#` も
- 文字列: PostgreSQL のドル引用（`$$ ... $$`）、MySQL のバッククォート、SQL Server の `[識別子]`、BigQuery の三重引用符
- キーワード: 方言ごとに違う（`LIMIT` と `TOP`、`RETURNING` など）

ADR-0017 は、サクラにない言語のタイプを足す前例である。方言のタイプを 5 つ（MySQL・PostgreSQL・SQL Server・SQLite・BigQuery）足すか、「SQL」の 1 つにまとめるかを決める。サクラの PL/SQL のタイプは、番号も中身も変えない。

## 6. セキュリティ

- 外部のクライアントを動かす点は、すでにある外部コマンド実行と同じ。ADR-0018 は外部のプログラムの実行をユーザーが決めることに入れているので、案 A も、実装の前に確認が要る
- SQL インジェクションは MATSU の範囲外（利用者が自分のファイルを自分の DB に実行する）
- MATSU の範囲は、資格情報（平文で保存しない、履歴に残さない）、TLS（クライアント任せなら、クライアントの設定）、雛形の展開（ファイル名の引用）
- 案 B を選んだ場合は、`/security-review` の対象になる（資格情報、ネットワーク、依存）。案 A でも、雛形の展開とコマンド履歴を見直す
