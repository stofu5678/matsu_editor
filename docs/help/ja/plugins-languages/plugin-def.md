# plugin.def の書き方

`plugin.def` は、プラグインのフォルダーに置く定義ファイルです。名前、動く場面（ジャック）、コマンド、オプションを書きます。形式は INI です。サクラエディタの `plugin.def` と同じ形で読みます。サクラエディタ用に書かれた定義ファイルも、読めます。

MATSU Editor は、`plugin.def` を書き換えません。

## 文字コード

- 先頭に BOM があれば UTF-8 として読みます。
- BOM がなければ Shift_JIS として読みます。

日本語を書くときは、BOM 付きの UTF-8 で保存してください。ハンドラのスクリプトの文字コードは別で、UTF-8 だけです（[ハンドラの中で使えるもの](plugin-api.md)）。

## 言語ごとの上書き

`plugin.def` のあるフォルダーの `local` フォルダーに、`plugin_ja_JP.def` を置くと、次の項目を上書きできます。

- `[Plugin]` の `Name`、`Description`、`Url`
- `[Plug]` と `[Command]` の `.Label`
- `[Option]` の `.Label` と `.Select`
- `[String]` の `S[n]`

上書きできるのは、書いた項目だけです。書かなかった項目は、`plugin.def` のままです。読むファイルは `plugin_ja_JP.def` に決まっています。表示の言語を変える設定は、ありません。

## 書き例

```
; An example of plugin.def (UTF-8 with a BOM).
[Plugin]
Id=jp.example.hello
Name=Hello
Description=Inserts a greeting.
Type=wsh
Author=Example
Version=1.0.0
Url=https://example.com/

[Plug]
DocumentOpen=opened.js
DocumentOpen.Label=Say hello when a file is opened

[Command]
C[1]=insert.js
C[1].Label=Insert a greeting

[Option]
O[1].Section=Hello
O[1].Key=Greeting
O[1].Label=Greeting
O[1].Type=Str
O[1].Default=Hello

[Wsh]
UseCache=0
```

## `[Plugin]`

| キー | 内容 |
|---|---|
| `Id` | プラグインの ID。必須。使える文字の規則は[プラグインを入れる](plugin-install.md)にあります |
| `Name` | 一覧に出す名前。なければ空です。コマンドの副メニューの名前には、なければフォルダー名を使います |
| `Description` | 一覧の下に出す説明 |
| `Type` | `wsh` か `script`。大文字と小文字は区別しません。どちらも同じ扱いです。`script` は MATSU Editor が足した名前で、Windows のスクリプトを思わせない名前にするためのものです。`dll` は動かしません |
| `Author` | 作者。確認の画面に出ます |
| `Version` | 版。確認の画面に出ます |
| `Url` | 配布元。「配布元」のボタンで開きます |

## `[Plug]`

ジャックにハンドラを結びます。

```
[Plug]
Outline=outline.js
Outline.Label=Markdown headings
Outline[1]=other.js
Outline[1].Label=Another outline
```

- キーは、ジャックの名前（11 個。[プラグインとは](plugin-concept.md)）。同じジャックに複数のハンドラを書くときは、`ジャック名[1]`、`ジャック名[2]` と続けます。番号は 49 までです。
- 番号は続けて書きます。途中で抜けると、そこから先は読みません。
- `ジャック名.Label` と `ジャック名[n].Label` は、一覧に出す名前です。なければ、ハンドラのファイル名を使います。`Outline`、`SmartIndent`、`Complement` は、タイプ別設定の選択肢の名前になります。
- ハンドラは、プラグインのフォルダーの中のファイルです。パスの区切りは `/` と `\` のどちらでも書けます。

## `[Command]`

コマンドを定義します。

| キー | 内容 |
|---|---|
| `C[1]` から `C[49]` | ハンドラのファイル。1 から続けて書きます。途中で抜けると、そこから先は読みません |
| `C[n].Label` | メニューなどに出す名前。なければハンドラのファイル名 |
| `C[n].Icon` | アイコン（BMP）のファイル名。プラグインのフォルダーからの相対パスで、フォルダーの外は指せません。縦横が同じ大きさの BMP を 16×16 に縮めて、ツールバーとメニューに使います。左上の点の色が透明になります。なかったり、読めなかったり、縦横が違ったりするときは、既定の絵です |

## `[Option]`

利用者が「設定」のダイアログで変えられる値を定義します（[プラグインのオプション](plugin-options.md)）。

| キー | 内容 |
|---|---|
| `O[n].Key` | 値を書くときのキー。必須。`n` は 1 から 99 |
| `O[n].Section` | 値を書くセクション。省くと、前のオプションのセクションを引き継ぎます。最初のオプションで省くと、そのオプションは無視されます |
| `O[n].Label` | ダイアログに出す名前。なければ `Key` |
| `O[n].Type` | `Str`（文字列）、`Dir`（フォルダー）、`Bool`（オンとオフ）、`Int`（整数）、`Sel`（選択）。大文字と小文字は区別しません。ほかの値は `Str` になります |
| `O[n].Default` | 既定の値 |
| `O[n].Select` | `Sel` の選択肢。`\|` で区切ります。`表示:値` と書くと、表示と値を分けられます。`:` がなければ、表示と値は同じです |

`Key` のないオプションは、その番号だけ飛ばして、次の番号を読みます。

## `[String]`

`S[1]` から `S[99]` に、プラグインが使う文字列を書きます。ハンドラは `Plugin.GetString(n)` で読みます。言語ごとの上書きに向いています。

## `[Wsh]`

| キー | 内容 |
|---|---|
| `UseCache` | `1` にすると、呼び出しのあともプロセスを残して、次の呼び出しに使います。既定は `0` で、呼び出しのたびに起動して終わらせます |

`SmartIndent`、`Complement`、`ComplementGlobal` は、キー操作で呼ぶので、プロセスが残っているプラグインだけが使えます。`UseCache=1` にするか、次の `[Matsu]` の `Resident=1` にします。

## `[Matsu]`

MATSU Editor が足した項目です。サクラエディタは、このセクションを読みません。同じプラグインが、サクラエディタでも MATSU Editor でも動けます。

| キー | 内容 |
|---|---|
| `Resident` | `1` にすると、`UseCache=1` と同じにプロセスを残します |
| `SmartIndent.Chars` | 改行のほかに、`SmartIndent` を呼びたい文字。`}` のように並べて書きます（[スマートインデント](plugin-smart-indent.md)） |

## `[DLL]`

`Name` に DLL のファイル名を書く項目です。MATSU Editor は DLL プラグインを動かさないので、読み込みません。

## 上限

| 項目 | 上限 |
|---|---|
| 登録できるプラグイン | 40 個 |
| 1 つのプラグインのコマンド | 49 個 |
| 1 つのジャックのハンドラ | 50 個（番号 0 から 49） |
| 1 つのプラグインのオプション | 99 個 |
| 1 つのプラグインの文字列 | 99 個 |

## サクラエディタとの違い

- `[Matsu]` と `Type=script` は、MATSU Editor が足したものです。
- `[Wsh]` の `UseCache` は、サクラエディタではスクリプトを覚える設定です。MATSU Editor では、プロセスを残す設定です。残したプロセスの中の、ハンドラの実行環境については、[JavaScript（QuickJS）](lang-quickjs.md)にあります。
- `Type=dll` は動かしません。

## 根拠

- MATSU のコード: `crates/matsu-config/src/plugins/def.rs`（`PluginDef`、`read_plugs`、`read_commands`、`read_options`、`read_strings`、`Jack::needs_resident`）、`crates/matsu-config/src/profile.rs` の `Profile::from_bytes`（BOM の有無で文字コードを決める）
- 言語ファイルの名前: `crates/matsu-config/src/plugins.rs` の `DEFAULT_LANGUAGE`、`SearchDirs::load_def`
- オプションの値の形: `crates/matsu-config/src/plugins/options.rs`
- 試験: `crates/matsu-config/src/plugins/tests.rs`
- サンプル: `crates/matsu-plugin/samples/plugins/`（`HelloJs`、`HelloCommands`、`HelloEvents`、`HelloOutline`、`HelloComplement`、`SmartIndentJs`、`HelloVbs`）
- サクラのソース: `sakura_core/plugin/CPlugin.cpp` の `ReadPluginDefCommon`、`ReadPluginDefPlug`、`ReadPluginDefCommand`、`ReadPluginDefOption`、`ReadPluginDefString`。`sakura_core/config/maxdata.h`（上限）
- `docs/adr/0026-resident-plugins.md` の 1 章と 2 章
