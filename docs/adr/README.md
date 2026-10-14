# 決定事項（ADR）

| 番号 | 内容 | 状態 |
|---|---|---|
| [0001](0001-project-approach.md) | プロジェクトの方針と進め方 | 決定（設定は 0010、順序は 0012 で置き換え） |
| [0002](0002-target-platforms.md) | 対象プラットフォーム | 決定（初期開発の対象は 0009） |
| [0003](0003-language-and-license.md) | 言語とライセンス | 決定 |
| [0004](0004-ui-framework.md) | UI フレームワーク | 決定（Linux の GPU での確認は残っている） |
| [0005](0005-text-buffer-and-large-files.md) | テキストバッファと巨大ファイル | 決定（細部は Phase 1・6 で詰める。当面は 2GB まで: 0012） |
| [0006](0006-regex-engine.md) | 正規表現エンジン | 決定 |
| [0007](0007-macros-and-plugins.md) | マクロとプラグイン | 提案中（Phase 3 の前に確定。キーマクロ以外は JSON-RPC。初期は JavaScript と Python、第 2 弾に Emacs Lisp） |
| [0008](0008-commands-and-keybindings.md) | コマンドとキー割り当て | 決定（キー割り当てのファイルは 0010） |
| [0009](0009-initial-development-target.md) | 初期開発の対象（Windows ARM64 を優先） | 決定 |
| [0010](0010-sakura-compatible-settings.md) | 設定ファイルをサクラと互換にする | 決定 |
| [0011](0011-printing.md) | 印刷 | 決定（細部は Phase 4 で詰める） |
| [0012](0012-file-size-limit.md) | 当面は 2GB までのファイルを扱う | 決定 |
| [0013](0013-module-structure.md) | モジュールの構造 | 決定（細部は試作で詰める） |
| [0014](0014-development-process.md) | 開発の進め方とバージョンの付け方 | 決定（ループのルールは Phase 5 の前に書く） |

新しい決定は、次の番号のファイルを足して記録する。決定を変えるときは、元の ADR を書き換えずに新しい ADR を足し、元の ADR の状態を「置き換え済み」にする。
