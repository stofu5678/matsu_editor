# MicroPython

拡張子は `.upy` です。MATSU Editor に同梱の `matsu-script` が、MicroPython で動かします。利用者が入れるものは、ありません。

`.py` は、利用者が入れた CPython で動きます（[Python](lang-python.md)）。`.mpy` は MicroPython のバイトコードの拡張子なので、使いません。

## 処理系

| 項目 | 内容 |
|---|---|
| 版 | MicroPython 1.29.0 |
| 作り | 公式の embed ポートで作った C のソースを、一緒にビルドしたもの（クレート `matsu-micropython-sys`） |
| ライセンス | MIT。正規表現のエンジン（re1.5）は BSD-3-Clause |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |
| ヒープ | 64 MiB に固定。超えると `MemoryError` |

言語の範囲は、CPython ではなく、MicroPython のものです。CPython にあって MicroPython にない機能は、使えません。

## 標準ライブラリ

`import` できるのは、組み込みのモジュールだけです。

| モジュール | 備考 |
|---|---|
| `sys` | `sys.platform` は `matsu`。`sys.path` と `sys.argv` はありません |
| `json` | |
| `math` | |
| `struct`、`array`、`binascii` | |
| `collections` | `deque` と `OrderedDict` があります |
| `heapq` | |
| `re` | 下の節 |
| `io` | `BytesIO` はあります。ファイルを開くクラスは、ありません |
| `errno`、`gc` | |

ファイルを開くこと、時刻、乱数、`os`、ネットワーク、`asyncio`、`hashlib` は、ありません。`input` と `help` も使えません。

### `re`

MicroPython の小さな `re` です（エンジンは re1.5。UTF-8 のバイトに対して動きます）。CPython の `re` と違う点は、次のとおりです。

- ある: `compile`、`match`、`search`、`sub`、コンパイルしたパターンの `split`、グループ、`span`、`start`、`end`
- ない: 先読みと後読み、名前付きグループ、インラインのフラグ、`re.I` などのフラグ、`{m,n}`（波括弧は、ただの文字になります）、`\b`、`findall`、`finditer`、`fullmatch`、`escape`、モジュールの `split`
- `.` は、改行にも合います。
- `.` と繰り返しは、1 文字ではなく、1 バイトに働きます。日本語の文字の途中で終わる一致は、`UnicodeError` になります。文字そのもの、`|`、範囲は、使えます。

## 文字コード

- スクリプトは UTF-8 で保存します（BOM は取り除きます）。
- 文字列は Unicode です。内部は UTF-8 です。
- 整数の値の浮動小数点数は、整数として渡します。
- 引数の最後の `None` は省略として扱います。

## エラー

`EditorError`（`Exception` を継承）は、`code` と `message` を持ちます。

受けなかった例外は、MicroPython の形で、標準エラーに出ます（`File "名前", line 3, in <module>`）。スクリプトは、終了コード 1 で終わります。エラーの文は、できるだけ CPython と同じです。

## プラグインとして動かすとき

拡張子が `.upy` のハンドラを、同梱の MicroPython が動かします。`.py` は、利用者が入れた Python が動かします（[Python](lang-python.md)）。

- MicroPython は、1 つのプロセスで 1 つしか動かせないので、待ち受けの繰り返しは、Python で書いたもの（`micropython_plugin.py`）が持ちます。ハンドラのファイルごとに、名前空間（グローバルの辞書）を作り、`exec` で文面を動かします。プロセスが残っている（`UseCache=1` か `Resident=1`）間、同じファイルの次の呼び出しは、同じ名前空間を使います。文面は呼び出しのたびに最初から動くので、前の呼び出しの値は、`globals().get('名前', 初めの値)` のようにして読みます。別のファイルの名前空間は、見えません。
- 名前空間には、`Editor`、`EditorError`、`Plugin`、`Outline`、`Complement`（と、`SmartIndent` のとき `Indent`）があります。`import` は要りません。`Plugin.id`、`Plugin.dir`、`Plugin.jack`、`Plugin.args`（呼び出しの引数の辞書。無いときは `None`）があります（[ハンドラの中で使えるもの](plugin-api.md)）。
- 関数は、同じ綴りの関数です（`Plugin.GetOption('A', 'B')`）。引数の最後の `None` は、省略として扱います。`Complement.AddList` は、引数を `str` にして渡します。
- `sys.exit()` は、そのハンドラを終えるだけで、失敗ではありません。
- 例外が出ると、その呼び出しは失敗になります。トレースバックは、ハンドラのファイルの行番号つきです（`File "名前", line 3, in <module>`）。
- 標準エラーへの出力（`print(..., file=sys.stderr)`）は、その呼び出しが終わる前に、エディターに届きます。
- ヒープ（64 MiB）は、すべてのファイルで 1 つです。

## 根拠

- 版、ヒープ、モジュール: `crates/matsu-micropython-sys/vendor/micropython/py/mpconfig.h`（`MICROPY_VERSION_*`）、`crates/matsu-micropython-sys/src/lib.rs`（`HEAP_SIZE`）、`crates/matsu-micropython-sys/embed/mpconfigport.h`（使えるモジュールの設定）
- `re` の違い、`EditorError`、引数の扱い: `crates/matsu-script/src/micropython.rs` の冒頭の説明、`BOOT`
- ライセンス: `crates/matsu-micropython-sys/Cargo.toml`、`THIRD_PARTY_NOTICES.txt`
- プラグイン: `crates/matsu-script/src/micropython_plugin.py`、`crates/matsu-script/src/plugin.rs`
- 試験: `crates/matsu-script/tests/micropython.rs`、`crates/matsu-script/tests/plugin_engines.rs`
- 設計: `docs/poc/json-rpc.md`（「同梱のスクリプトホスト」）
