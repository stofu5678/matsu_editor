# ファイルの場所とシェル、保存、プロファイル

メニューにない、ファイル操作系の機能です。1 つの表にまとめました。

| 機能名 | 表示名 | キー | マクロ |
|---|---|---|---|
| `F_FILESAVE_QUIET` | （名前なし） | なし | なし |
| `F_OPEN_FOLDER_IN_EXPLORER` | ファイルの場所を開く | なし | なし |
| `F_OPEN_COMMAND_PROMPT` | コマンドプロンプトを開く | なし | なし |
| `F_OPEN_COMMAND_PROMPT_AS_ADMIN` | 管理者としてコマンドプロンプトを開く | なし | なし |
| `F_OPEN_POWERSHELL` | PowerShellを開く | なし | なし |
| `F_OPEN_POWERSHELL_AS_ADMIN` | 管理者としてPowerShellを開く | なし | なし |
| `F_PROFILEMGR` | プロファイルマネージャ | なし | なし |

どれも、既定のキーも、マクロの名前もありません。キー割り当てやカスタムメニュー、ツールバーに割り当てて使います。`F_FILESAVE_QUIET` 以外は、「コマンド一覧」の「ファイル操作系」のグループにあります（[コマンド一覧](command-list.md)）。`F_FILESAVE_QUIET` は、その一覧にありません。

同じグループにある `F_FILESAVEAS`（マクロ `FileSaveAs`）と `F_FILEOPEN_DROPDOWN` は、「コマンド: ファイル」の章にある、保存と開くのページで説明しています。

## 保存（静かに）

`F_FILESAVE_QUIET`: 上書き保存と同じ手順で保存します。ただし、ファイルのない文書では、名前を聞かずに、何もしません。

- ファイルのある文書では、「上書き保存」と同じです。
- ファイルのない文書では、保存の箱を出さず、何も保存しません。

## ファイルの場所を開く

`F_OPEN_FOLDER_IN_EXPLORER`: OS のファイル管理アプリで、この文書のファイルを見せます。

| OS | 動き |
|---|---|
| Windows | エクスプローラーが開き、ファイルが選ばれる |
| macOS | Finder が開き、ファイルが選ばれる |
| Linux | デスクトップのファイル管理アプリを開く。それができないときは、フォルダを `xdg-open` で開く |

## シェルを開く

フォルダは、この文書のファイルのあるフォルダです。

| 機能 | Windows | macOS | Linux |
|---|---|---|---|
| コマンドプロンプトを開く | 環境変数 `COMSPEC` の `cmd.exe` を、そのフォルダで開く | 「ターミナル」（Terminal）を、そのフォルダで開く | 入っているターミナルのうち最初に見つかったものを、そのフォルダで開く（`x-terminal-emulator`、大きなデスクトップのターミナル、`xterm` の順） |
| 管理者としてコマンドプロンプトを開く | 管理者の権限で、同じように開く | 使えない（ステータスバーに知らせる） | 使えない（同じ） |
| PowerShellを開く | `powershell.exe` を、そのフォルダで開く（`-NoExit` で開いたままにする） | `pwsh` が入っていれば、「ターミナル」の中で、そのフォルダで開く。入っていなければ、使えない | `pwsh` が入っていれば、上のターミナルの中で、そのフォルダで開く。入っていなければ、使えない |
| 管理者としてPowerShellを開く | 管理者の権限で、同じように開く | 使えない | 使えない |

- ネットワークのフォルダ（`\\サーバー\共有` の形）のファイルでは、Windows のコマンドプロンプトは開けません。
- PowerShell は、フォルダに `[` や `*` があっても、その名前のフォルダへ移ります（`-LiteralPath` を使います）。

## ファイルがないとき

「ファイルの場所を開く」と、シェルを開く 4 つの機能は、ファイルのない文書（新規のまま）では、何も開きません。ステータスバーに「ファイルがないので使えない」と出ます。マクロは、エラーにならずに、そのまま続きます。サクラエディタは、同じときにビープ音を鳴らします。

開けなかったときは、ステータスバーに、使えない理由か「開けなかった: …」と出ます。

## プロファイルマネージャ

`F_PROFILEMGR` は、まだ動きません。動かすと、ステータスバーに「F_PROFILEMGR はまだ使えない」と出ます。設定ファイル `matsu.ini` の置き場所の決め方に、ユーザーの判断が要るためです。起動のときに `-PROF=名前` で設定ファイルを選ぶ機能は、すでにあります（「仕様等と正規表現」の章の、コマンドラインの説明を見てください）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILESAVE`、`Command_OPEN_FOLDER_IN_EXPLORER`、`Command_OPEN_COMMAND_PROMPT`、`Command_OPEN_POWERSHELL`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/` の `file_save_quiet.rs`、`open_folder_in_explorer.rs`、`open_command_prompt.rs`、`open_command_prompt_as_admin.rs`、`open_powershell.rs`、`open_powershell_as_admin.rs`。`crates/matsu-ui/src/file_ops/external.rs`（ファイルがないとき、エラーの文）、`crates/matsu-ui/src/file_ops/save.rs`（`SaveRequest::quiet`）、`crates/matsu-platform/src/shell/mod.rs`
- 未対応: `crates/matsu-ui/src/view/mod.rs` の `run_function`（コマンドがない機能の知らせ）、`docs/sakura-gaps.md` のプロファイルの行
- 試験: `crates/matsu-ui/src/file_ops/external.rs` の `the_folder_of_a_file`、`errors_are_told_in_japanese`、`crates/matsu-ui/src/view/tests.rs` の `a_function_without_a_command_says_so`
