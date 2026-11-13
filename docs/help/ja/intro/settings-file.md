# 設定ファイル matsu.ini

MATSU Editor の設定は、`matsu.ini` というテキストファイルに保存します。書式は、サクラエディタの `sakura.ini` と同じです。ファイルの名前と置き場所は、サクラエディタのものと取り違えないように変えてあります。MATSU Editor は、`sakura.ini` を読みも書きもしません。

## 置き場所 {#location}

MATSU Editor は、次の場所を上から順に探し、最初に見つかったファイルを使います。どれにもないときは、設定を保存するときに、表の最後の場所へ作ります。

| OS | 探す場所（上から順） |
|---|---|
| Windows | 1. 実行ファイルと同じフォルダーの `matsu.ini`（持ち運び用）<br>2. `%APPDATA%\matsu-editor\matsu.ini` |
| macOS | `~/Library/Application Support/matsu-editor/matsu.ini` |
| Linux | `$XDG_CONFIG_HOME/matsu-editor/matsu.ini`。`XDG_CONFIG_HOME` が空か絶対パスでないときは `~/.config/matsu-editor/matsu.ini` |

Windows では、実行ファイルの隣に `matsu.ini` を置くと、それが使われます。ファイルごと持ち運べます。ただし、インストーラは、インストール先に `matsu.ini` を作りません。作ると、持ち運び用の動きに固定されるためです。

### プロファイル

起動のときに `-PROF=名前` を付けると、上の場所のフォルダーの下に「名前」のサブフォルダーを作り、その中の `matsu.ini` を使います。

```text
matsu -PROF=work
```

Windows では、探す順がプロファイルでも変わりません。実行ファイルのフォルダーの下の `work\matsu.ini` を先に探し、あればそれを使います。なければ `%APPDATA%\matsu-editor\work\matsu.ini` を使います。持ち運び用のプロファイルを使っているときは、`%APPDATA%` 側のファイルを間違えて編集しないようにしてください。保存するときも、見つかったファイルへ書きます。

macOS と Linux では、上の表のフォルダーの下の `work/matsu.ini` です。プロファイルが違えば、設定も、動くプロセスも別になります。

プロファイルを選ぶ画面（プロファイルマネージャ）は、まだありません。

## 同じフォルダーに置かれるもの

`matsu.ini` の隣に、MATSU Editor が使う別のファイルとフォルダーが置かれます。

| 名前 | 内容 |
|---|---|
| `unsaved\` | 保存していない文書の控え。次の起動で戻すために使います |
| `plugins.ini` | プラグインを有効にしてよいと、利用者が答えた記録 |
| `plugins\` | プラグインのフォルダーと、プラグインのオプションの値 |
| `databases.ini` | データベースの接続の設定（パスワードは書きません） |

これらはなくても動きます。必要になったときに作ります。

## ファイルの形

- 文字コードは、新しく作るファイルでは UTF-8（BOM 付き）です。BOM のないファイルは、Shift_JIS として読みます。すでにあるファイルは、今の文字コードのまま書き戻します。Shift_JIS で表せない文字を書くときだけ、UTF-8（BOM 付き）に変わります
- `[節の名前]` の下に `キー=値` を並べます
- `;` か `//` で始まる行は、コメントです
- 同じキーが 2 つあれば、先のものが使われます
- 改行コードは、新しく作るファイルでは OS に合わせます（Windows は CRLF、macOS と Linux は LF）。すでにあるファイルは、今の改行コードのまま書き戻します

MATSU Editor は、読んだだけで使わなかった行（コメント、知らない節やキー）を、書き戻すときに消しません。まだ作っていない機能の設定を失わないためです。

## MATSU Editor だけの項目

サクラエディタにない項目は、サクラエディタが使わない節 `[MATSU]` やキーに置きます。サクラエディタで `matsu.ini` を開いても、壊れません。

| 節 | キー | 内容 |
|---|---|---|
| `[MATSU]` | `nMacKeyLayout` | macOS でキー割り当てをどう読むか。`0` は標準、`1` はすべて Control、`2` は Cmd を Ctrl として扱う |
| `[MATSU]` | `KeepUnsaved`、`KeepUnsavedNoAsk`、`KeepUnsavedMaxMB` | 保存していない文書の控え。残すか、終了時に確認しないか、1 つの文書の上限（MB。`0` は上限なし。既定は 64） |
| `[MainMenu]` | `nMatsuMainMenuVer` | メインメニューに MATSU だけの項目を足すための版 |
| `[CustMenu]` | `nMatsuCustomMenuVer` | 右クリックのメニューに MATSU だけの項目を足すための版 |
| `[Terminal]` | `Shell`、`ScrollbackLines`、`OptionAsMeta` | シェルのタブの設定 |

これらは、設定の画面で変えられるものがほとんどです。手で書き換えるときは、MATSU Editor を終えてからにしてください。

## 前の名前の設定

以前の製品名の設定ファイルがあるときは、起動のときに引き継ぎます。新しい場所に `matsu.ini` がなく、前の名前の設定ファイルが前の場所にあれば、そのフォルダーの中身を新しい場所へコピーします。古いファイルは消さず、書き換えません。引き継いだときは、ステータスバーに知らせます。この仕組みは、1.0 の版で取り除く予定です。

## サクラエディタの設定を使うには

サクラエディタの `sakura.ini` を、そのまま読み込む機能は、まだありません。次の方法で、写し取ってください。

1. `sakura.ini` を、上の「置き場所」のフォルダーへコピーします
2. 名前を `matsu.ini` に変えます

書式が同じなので、MATSU Editor は `sakura.ini` の内容を読めます。ただし、使わない項目や、まだ作っていない機能の項目もあります。MATSU Editor が使わない項目は、書き戻すときも残します。Windows に固有の値（フォントの指定、ウィンドウの位置、パス）は、ほかの OS では読み替えが要ります。この手順での動きは、MATSU Editor が `sakura.ini` と同じ書式を読み書きする（`docs/adr/0010-sakura-compatible-settings.md`）という決めごとにもとづくもので、実際のサクラエディタの設定ファイルでの確認は、このページを書いたときにはしていません。

## 根拠

- 置き場所: `crates/matsu-config/src/location.rs`（`candidate_paths_named`、`write_path`）、試験は同じファイルの `tests`
- 書式と文字コード、改行コード: `crates/matsu-config/src/profile.rs` の先頭の説明、`docs/adr/0010-sakura-compatible-settings.md`
- サクラエディタの `sakura.ini` の置き場所と書式: `sakura_core/_main/CControlProcess.cpp`、`sakura_core/env/CProfile.cpp`
- `-PROF=`: `crates/matsu-config/src/location.rs`（`with_profile`）
- 同じフォルダーのもの: `crates/matsu-config/src/unsaved/store.rs`（`unsaved_dir`）、`crates/matsu-config/src/plugins.rs`、`crates/matsu-config/src/databases.rs`
- MATSU だけの項目: `crates/matsu-ui/src/settings/mac_key_layout.rs`、`crates/matsu-ui/src/unsaved/prefs.rs`、`crates/matsu-config/src/terminal.rs`、`docs/adr/0010-sakura-compatible-settings.md` の「メニューの MATSU の版」
- 前の名前の引き継ぎ: `crates/matsu-config/src/migrate_old_name.rs`、`docs/adr/0021-rename-to-matsu.md`
