# プラグインとスクリプトの言語

この章は 2 つの話をします。

- **プラグイン**。MATSU Editor が、決まった場面（コマンドを選んだとき、ファイルを保存する前など）に呼び出すスクリプトの集まりです。
- **スクリプトの言語**。マクロとプラグインを書ける言語ごとの、動かすプログラム（処理系）の版、使える標準ライブラリ、文字コード、エラーの出方、時間の制限、止め方です。

マクロの使い方、言語と拡張子の対応表、言語ごとの呼び出しの書き方は、「マクロの使い方」の章にあります。この章では繰り返しません。共通設定の「プラグイン」のページ（ボタンと一覧）は、「コマンド: 設定（共通設定）」の章にあります。

## プラグイン

| ページ | 内容 |
|---|---|
| [プラグインとは](plugin-concept.md) | マクロとの違い、動く場面の一覧、今動かせる言語 |
| [プラグインを入れる](plugin-install.md) | 置き場所、追加、有効にする確認、変更の検出、外す |
| [plugin.def の書き方](plugin-def.md) | 定義ファイルの項目と上限 |
| [ハンドラの中で使えるもの](plugin-api.md) | `Editor` `Plugin` `Outline` `Indent` `Complement` |
| [コマンド](plugin-commands.md) | メニュー・キー・ツールバーから選んで動かす |
| [イベント](plugin-events.md) | 開く・閉じる・保存・ウィンドウの開始と終わり |
| [アウトライン](plugin-outline.md) | アウトライン解析をプラグインに任せる |
| [入力補完](plugin-complement.md) | 補完の候補を足す |
| [スマートインデント](plugin-smart-indent.md) | 改行のあとの字下げをプラグインに任せる |
| [プラグインのオプション](plugin-options.md) | 「設定」ボタンのダイアログと、値の置き場所 |
| [時間の制限と失敗](plugin-limits.md) | 場面ごとの制限、エラーの見え方、停止 |

## スクリプトの言語

| ページ | 内容 |
|---|---|
| [すべての言語に共通のこと](lang-common.md) | 文字コード、出力、エラー、時間の制限、止め方 |
| [Lua](lang-lua.md) | 同梱 |
| [JavaScript（QuickJS）](lang-quickjs.md) | 同梱。プラグインを動かせる唯一の言語 |
| [MicroPython](lang-micropython.md) | 同梱 |
| [mruby](lang-mruby.md) | 同梱 |
| [JavaScript（Node.js）と TypeScript](lang-node.md) | 外部 |
| [Python](lang-python.md) | 外部 |
| [Ruby](lang-ruby.md) | 外部 |
| [PHP](lang-php.md) | 外部 |
| [PowerShell](lang-powershell.md) | 外部 |
| [Perl](lang-perl.md) | 外部 |
| [Emacs Lisp](lang-emacs-lisp.md) | 外部 |
| [AppleScript](lang-applescript.md) | 外部（macOS だけ） |

## 根拠

- `docs/adr/0026-resident-plugins.md`、`docs/adr/0025-script-languages.md`、`docs/adr/0007-macros-and-plugins.md`
- `crates/matsu-plugin/src/lib.rs`（クレートの構成）、`crates/matsu-script/src/lib.rs`
