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

`F_PROFILEMGR` は、プロファイル（名前をつけた設定のまとまり）を選ぶ画面を開きます。起動のとき `-PROFMGR` を付けても、同じ画面が開きます。画面はサクラエディタの「プロファイルマネージャ」と同じ作りで、別のウィンドウです。

| 部品 | 動き |
|---|---|
| 一覧 | 先頭の「(default)」は、名前のないプロファイル（設定ファイルを置く場所そのもの）です。起動時に使うデフォルトの行には、名前のあとに `*` が付きます。上下キー、Home、End と、クリックで選びます |
| 起動(S) | 選んだプロファイルで、新しい MATSU Editor を始めます。プロファイルはプロセスごとに決まるので、新しいプロセスで始まります（`-PROF=名前` を付けた起動と同じ）。画面は閉じます |
| 閉じる(X) | 何もせずに閉じます（Esc とウィンドウの閉じるボタンも同じ）。起動のときに開いていたなら、起動をやめます |
| 新規作成(N) | 名前を入力して、一覧の終わりに足します。同じ名前の「ファイル」があると、作れません |
| 名前変更(R) | 選んだ名前を変えます。そのプロファイルのフォルダーがあれば、フォルダーも新しい名前に移します（大文字と小文字だけの変更もできます）。使用中のプロファイルは、変えられません。移せなければ、OS の理由つきで「名前の変更に失敗しました」と出ます。「(default)」には使えません |
| 削除(D) | 選んだ名前を一覧から消します。プロファイルのフォルダーと、その中の設定は消さずに、そのまま残します。使用中のプロファイルは、消せません。「(default)」には使えません |
| デフォルト設定(F)、デフォルト解除(C) | 選んだ行を、起動時に使うデフォルトにします。解除は、デフォルトを決めていない状態に戻します |
| デフォルト設定にして起動(E) | 起動(S) を押したとき、選んだ行をデフォルトにします |
| ヘルプ(H)、F1 | このページを開きます |

名前に使えない文字は、`/` `\` `*` `?` `<` `>` `&` `|` `:` `"` `'` とタブと、`..` です。一覧にある名前（「(default)」を含む）と、大文字と小文字の違いだけの名前も、使えません。名前の長さは 259 文字までです。

一覧は、変えるたびに `matsu_prof.ini` に書かれます。書くときは、`matsu_prof.ini.lock` をロックして（ほかのウィンドウや起動のときの画面が同時に書かないように。数秒待っても取れなければ、箱で知らせます）、ファイルを読み直して、その 1 つの変更だけを足します。ほかのウィンドウや、起動のときの画面が変えた内容を、古い一覧で上書きすることはありません。書けなかったときは、箱で知らせて、何も変えません。名前変更は、一覧を先に書いてからフォルダーを移し、移せなければ、そのときの一覧の、その行の名前だけを元に戻します。

「使用中」は、名前のあるプロファイルで動いている MATSU Editor が、そのプロファイルのフォルダーの `in-use-プロセス番号.lock`（プロセスごとに 1 つ）をロックしていること。同じプロファイルで複数の MATSU Editor が動いているときは、どれか 1 つでも動いていれば使用中ですです。ロックはプロセスのものなので、MATSU Editor が終われば、異常終了しても、OS が放します（残ったファイルは何も押さえません。同じ番号のプロセスが、また使います）。macOS と Linux のロックは、ロックを使うプログラム（MATSU Editor）にだけ効きます。

プロファイルのフォルダーは、`-PROF=名前` で起動した MATSU Editor が `matsu.ini` を探す場所と同じです。Windows で実行ファイルの隣に `matsu.ini` を置いているときは、`名前\matsu.ini` が実行ファイルの隣にあれば、そこです。なければ `%APPDATA%\matsu-editor\名前` です。起動のときに画面が開くのは、次のとおりです。

- `-PROFMGR` を付けたとき。
- `-PROF=` がなく、`matsu_prof.ini` があって、そのデフォルトが決まっていない（`nDefaultIndex` が -1、または名前の数より大きい）とき。

`matsu_prof.ini` がないとき、または、デフォルトが決まっているときは、画面を出さずに、そのデフォルトのプロファイル（なければ名前のないプロファイル）で起動します。

`matsu_prof.ini` の場所と形式は、[設定ファイル matsu.ini](../intro/settings-file.md) にあります。サクラエディタとの違いは、「(default)」をデフォルトにした場合も、画面を出さずに起動する点です（サクラエディタは、毎回画面を出します）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILESAVE`、`Command_OPEN_FOLDER_IN_EXPLORER`、`Command_OPEN_COMMAND_PROMPT`、`Command_OPEN_POWERSHELL`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/` の `file_save_quiet.rs`、`open_folder_in_explorer.rs`、`open_command_prompt.rs`、`open_command_prompt_as_admin.rs`、`open_powershell.rs`、`open_powershell_as_admin.rs`。`crates/matsu-ui/src/file_ops/external.rs`（ファイルがないとき、エラーの文）、`crates/matsu-ui/src/file_ops/save.rs`（`SaveRequest::quiet`）、`crates/matsu-platform/src/shell/mod.rs`
- プロファイルマネージャ: サクラのソースは `sakura_core/dlg/CDlgProfileMgr.cpp`、`sakura_core/cmd/CViewCommander_File.cpp` の `Command_PROFILEMGR`、`sakura_core/_main/CProcessFactory.cpp` の `ProfileSelect`。MATSU のコードは `crates/matsu-config/src/profile_manager.rs`（ファイルと名前の規則、起動の判断）、`crates/matsu-ui/src/profile_manager/`（画面、起動）、`crates/matsu-ui/src/ui_commands/file/profilemgr.rs`
- 試験: `crates/matsu-ui/src/file_ops/external.rs` の `the_folder_of_a_file`、`errors_are_told_in_japanese`、`crates/matsu-config/src/profile_manager/tests.rs`、`crates/matsu-ui/src/profile_manager/tests.rs`
