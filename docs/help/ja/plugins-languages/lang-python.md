# Python

拡張子は `.py` です。利用者が入れた Python で動かします。MicroPython の `.upy` とは別です（[MicroPython](lang-micropython.md)）。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | Windows は `python`、そのほかは `python3`（`PATH` から探します） |
| 版 | 下限を確かめません。試した版は 3.13 と 3.15 |
| プラグイン | 動かせません（今は QuickJS だけです） |

Windows で `python` が Microsoft Store の案内だけのときは、終了コード 9009 で終わります。このとき、「Python が見つかりません。インストールを確かめてください」と出ます。

## 動き方

- Python は、`-u`（出力をためない）を付けて起動します。`Editor` の呼び出しと、出力の順番が合います。
- スクリプトは、`runpy.run_path` で、`__main__` として動きます。行番号は、ファイルのものです。
- `Editor` と `EditorError` は、スクリプトのグローバルにあります。`import` は要りません。
- モジュールを探す場所の先頭は、スクリプトのあるフォルダーです。隣に置いたモジュールを、`import` できます。
- 環境変数 `PYTHONUTF8=1` と `PYTHONDONTWRITEBYTECODE=1` を付けて動かします。

## 標準ライブラリ

利用者の Python の標準ライブラリと、入れてあるパッケージが、そのまま使えます。

## 文字コード

- スクリプトは UTF-8 で保存します。
- `PYTHONUTF8=1` で、標準出力は UTF-8 です。Windows の Python は、何も指定しないと、パイプへの出力が CP932 などになるためです。Python 3.15 は、指定がなくても UTF-8 です。

## エラー

`EditorError` は `Exception` を継承します。`code` と、メッセージを持ちます。

受けなかった例外は、Python のトレースバックとして、標準エラーに出ます。スクリプトは、終了コード 1 で終わります。

## 根拠

- 起動の引数と環境変数: `crates/matsu-plugin/src/session.rs` の `start`、`Language::default_interpreter`
- `runpy`、`sys.path`、`EditorError`: `crates/matsu-plugin/src/boot/boot.py`
- 9009 の案内: `crates/matsu-ui/src/macros/mod.rs` の `on_host_event`
- 文字コードと試した版: `docs/poc/json-rpc.md`（「基本 API」の前の節と「確かめたこと」）
- 試験: `crates/matsu-plugin/src/tests.rs`
