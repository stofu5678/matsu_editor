# 下調べ: 右クリックのワンライナー（ADR-0029）

調査日: 2026-11-13。読み取り専用。他エディタは文書（マニュアル、ヘルプ、公式 docs、README）だけを読み、ソースコードは読んでいない。Windows の実験は、この PC（Windows on Arm64、日本語ロケール）の WSL2 から相互運用（interop）で動かした。

## 先頭の要約（判断に効く事実）

1. 近い既存例は VS Code の拡張 2 つで、どちらも settings.json に「名前 + コマンド 1 行」の一覧を持つ。Filter Text（`filterText.commandList`）と Edit with Shell（`favoriteCommands`、シェルと引数も設定できる）。Kate の External Tools は、一覧、入力（stdin）、出力先（置換・挿入・新規文書・クリップボード・パネル）を設定でき、設計がいちばん近い。
2. 出力先の選択肢は、TextMate・Kate・BBEdit のどれも「置換 / 挿入 / 新規文書 / パネル / クリップボード / ツールチップ」から選ぶ形。非ゼロ終了時の扱いは BBEdit だけ文書で確認できた（何もせずエラー表示。選択範囲を空で置き換えない）。
3. Windows PowerShell 5.1 の既定は罠が多い。実測で、`$input` は起動時のコンソール入力コードページ（この PC では CP932）で読み、あとから `[Console]::InputEncoding` を UTF-8 にしても効かない。`[Console]::In.ReadToEnd()` はあとからの設定が効き、UTF-8 の往復が正確（絵文字も含む）。`$OutputEncoding` は US-ASCII だった。
4. pwsh と powershell.exe の `-Command -` は、スクリプトを stdin から読む。テキストを stdin で渡す設計と衝突するので、1 行は必ず `-Command "<文字列>"` か `-EncodedCommand` で渡す（公式 docs）。
5. PowerShell は、出力の行ごとに CRLF を付け、末尾に改行がない入力（`abc`）でも `ABC\r\n` を出す（実測、5.1）。`[Console]::Out.Write` なら入力どおり（LF のまま、末尾の改行は足さない）。cmd の `sort`・`more` も CRLF を付ける。`findstr /n .` は LF のまま。
6. PowerShell の終了コードは粗い。`-Command` は最後のコマンドが外部プログラムで失敗すると 1 に丸める（docs と実測で一致）。元のコードを残すには `exit $LASTEXITCODE` を足す。途中で外部コマンドが失敗しても、最後が成功なら 0。
7. 起動時間（中央値 5 回、interop 込み）: cmd 29 ms、powershell.exe 5.1 `-NoProfile` 157 ms、sh 2 ms、bash 3 ms、bash -l 8 ms。この PC には PowerShell のプロファイルがなく、`-NoProfile` の差は測れなかった。pwsh 7 はこの PC に入っておらず未測定。
8. 大きな入力で、この環境の cmd + 外部プログラムが固まった（3 MB 以上で `findstr /n .` と `more` が 14 秒たっても終わらない。2 MB は 0.06 秒）。原因は未確認（推測: WSL interop の中継）。ただし、入力を全部書き終えてから出力を読む実装は、パイプの詰まりで固まりうる、という一般的な注意はそのまま当てはまる。別スレッドで同時に読むこと、タイムアウト、キャンセルが要る。
9. シェルの設定が「コマンドライン全体」のとき（`pwsh.exe -NoLogo`、`cmd.exe /K ...`）に「1 行を実行する」フラグを足す方法は、他エディタでは別の設定に分けている（Vim は `shell` と `shellcmdflag`、Edit with Shell は `shell.*` と `shellArgs.*`）。MATSU では ADR で決める必要がある。

---

## 1. 他のエディタはどうしているか

### 1.1 比較表

| エディタ | コマンドの置き場 | 入力 | 出力 | 非ゼロ終了 / stderr | シェル | 文字コード | undo | 末尾改行 |
|---|---|---|---|---|---|---|---|---|
| Vim `:{range}!cmd` | 保存しない（履歴のみ。`:!` の履歴） | 行範囲（ビジュアル選択、`!!` は行数指定、`!{motion}`）。文字単位の選択は行に切り上がる | 範囲を置換のみ | 'shellredir' の既定が `>%s 2>&1` なので stderr は出力に混ざる（sh 系、zsh、bash、fish、pwsh、cmd）。終了コードの扱いは未確認 | 'shell'（既定 `$SHELL` か `sh`、Win32 は `cmd.exe`）+ 'shellcmdflag'（既定 `-c`。Win32 で `powershell` を含むと `-Command`、`sh` を含まなければ `/c`） | 未確認（'encoding' に従うと推測） | 「これらのコマンドは undo できる」と文書にある | 未確認（行単位なので、行末改行は付く前提と推測） |
| Emacs `M-|` | 保存しない（ミニバッファ履歴） | リージョン（`C-x h` で全体） | 既定はバッファ `*Shell Command Output*`。数値引数付きだとリージョンを削除して出力で置換 | 既定で stderr は stdout に混ざる。`shell-command-default-error-buffer` に名前を入れると別バッファへ。終了コードの扱いは未確認 | `shell-file-name`（既定は起動時の `SHELL`。リモートなら `/bin/sh`） | `C-x RET c` で直前に指定 | 未確認 | 未確認 |
| TextMate | バンドルのコマンド（`~/Library/Application Support/TextMate/Bundles` 等）。入力・出力を項目で保存 | 選択範囲 / 文書。選択なしのときは予備の単位（文字・単語・行・スコープ・文書）を指定できる | 選択・文書の置換、テキストまたはスニペットとして挿入、ツールチップ、HTML 表示、新規文書、破棄 | bash の `exit_discard` など、スクリプトが終了の仕方で結果の使い方を選ぶ。stderr は未確認 | bash か先頭の shebang で指定したインタープリタ | 未確認 | 未確認 | 未確認 |
| BBEdit Text Filters | 「Text Filters」フォルダ（Application Support の中）。「Run Unix Command」パネルから保存できる（15.0） | 選択範囲、なければ文書全体（BBEdit 10 マニュアル、検索結果の要約。原文は直接読んでいない） | 標準出力が選択範囲か文書を置き換える | 非ゼロ終了なら、何もせずエラーを表示する（12.6.1 のリリースノート、確認済み） | 未確認 | 未確認 | 未確認 | 未確認 |
| CotEditor | Scripts フォルダの中のスクリプトファイル（ワンライナー一覧ではない） | UNIX スクリプトの入力キーワードで選ぶ（`CurrentLine` を 7.0.0 で追加。ほかのキーワードの正確な綴りは未確認） | 出力キーワードで選ぶ（`ReplaceCurrentLine` を追加。ほかは未確認） | 標準エラーはコンソールパネルに出す（4.0.0 で修正）。終了コードは未確認 | スクリプトの shebang | UNIX スクリプトは常に UTF-8 として読む（4.0.0） | 未確認 | 未確認 |
| Kate Text Filter | 履歴（コンボボックス）。登録機能なし | 選択範囲 | 選択範囲を置換、またはクリップボードへコピー | 「STDOUT と STDERR を混ぜる」を入れると混ぜてエラーは出さない。入れないと STDERR を受動メッセージで表示 | 「シェルコマンド」と書かれている（種類は未確認） | 未確認 | 未確認 | 未確認 |
| Kate External Tools | 設定に一覧（名前、実行ファイル、引数、入力、作業フォルダなど）。デフォルトのツールもある | 「Input」欄（stdin）に `%{Document:Selection:Text}` などの変数を書く | 無視 / カーソル位置に挿入 / 選択範囲を置換 / 文書を置換 / 文書の末尾に追加 / 新規文書 / クリップボード / パネル | 未確認 | 実行ファイルと引数を直接起動（シェルを介さない。シェルが要るなら `sh -c` を自分で書く。標準ツールの例に `konsole -e sh -c "..."` がある） | 未確認 | 未確認 | 未確認 |
| EmEditor 外部ツール | 設定の外部ツール一覧 | 文書全体 / なし（選択範囲は文書に記載なし） | 文書を置換、アウトプットバー、標準エラーの破棄など | 終了コードは文書に記載なし | 未確認 | 文字コードを指定できる（UTF-8 など） | 未確認 | 未確認 |
| サクラエディタ 外部コマンド実行 | ダイアログの履歴（コマンド一覧の保存はない） | 選択範囲、なければ編集中の内容全体 | アウトプットウィンドウ、または編集ウィンドウ（選択ありなら置換、選択なし + 標準入力なしならカーソル位置に挿入、選択なし + 標準入力ありなら全体を置換） | 本調査の対象外（既存実装あり） | `cmd.exe /C` 相当（既存） | SJIS / Unicode / UTF-8 を入力と出力で別に選べる | 既存で 1 回の undo | 未確認 |
| JetBrains External Tools | 設定（Tools | External Tools）に、プログラム、引数、作業フォルダの一覧 | マクロで引数に渡す（`$FilePath$` など）。選択範囲を stdin に送る欄は文書にない | 「Open console for tool output」でコンソールに stdout と stderr を出す。出力の置換はない | プログラムを直接起動（シェルを介さない） | 文書に記載なし | 該当なし | 該当なし |
| VS Code 本体 | 標準機能なし（検索した範囲では見つからなかった。未確認） | | | | | | | |
| VS Code 拡張 Filter Text (yhirose) | settings.json の `filterText.commandList`（name, description, command） | 選択範囲（なければカーソル位置に結果を挿入） | 選択範囲を置換 / 一時エディタに表示 | 未確認 | 未確認 | 未確認 | 未確認 | 未確認 |
| VS Code 拡張 Edit with Shell (ryu1kn) | settings.json の `favoriteCommands`（id, command）。クイックコマンド 1〜5 に割り当て | 選択範囲（`ES_SELECTED` 環境変数にも入る）。`processEntireTextIfNoneSelected` で、選択なしなら全文 | 選択範囲を置換、またはカーソル位置に挿入 | 未確認 | 既定は Linux/macOS が `/bin/sh -c`、Windows が `cmd.exe`。`shell.*` と `shellArgs.*` で変更（pwsh も指定可と書いてある） | 未確認 | 未確認 | 未確認 |

### 1.2 出典

- Vim: https://vimhelp.org/change.txt.html （`:{range}!`、`!{motion}`、'shellredir'、`:keepmarks`）、https://vimhelp.org/options.txt.html （'shell'、'shellcmdflag'、'shellredir' の既定値）、https://vimhelp.org/various.txt.html （`:!` のシェル組み立て）。undo の記述は、change.txt の該当節を要約した WebFetch の結果による（curl で取った本文では grep していない）。
- Emacs: https://www.gnu.org/software/emacs/manual/html_node/emacs/Single-Shell.html （`M-|`、数値引数、`shell-file-name`、`shell-command-default-error-buffer`、`C-x RET c`）、https://www.gnu.org/software/emacs/manual/html_node/elisp/Synchronous-Processes.html （`call-shell-region` の記述）。
- TextMate: https://macromates.com/manual/en/commands （Command Input / Command Output / `exit_*` 関数 / bash か shebang）、https://macromates.com/manual/en/bundles （バンドルの置き場）。「Filter Through Command」という名前のダイアログの説明は、このマニュアルのページからは見つからなかった（未確認）。
- BBEdit: 12.6.1 のリリースノート https://www.barebones.com/support/bbedit/notes-12.6.1.html に、"If a Unix text filter returns a nonzero exit code, BBEdit will do nothing and report an error, instead of replacing the selection (or document contents) with nothing." とある。15.0 のリリースノート https://www.barebones.com/support/bbedit/notes-15.0.html に「Run Unix Command」パネルから Text Filters / Scripts フォルダへ保存できるとある。選択範囲 / 文書全体と stdin の記述は、BBEdit 10 マニュアル（https://pine.barebones.com/manual/BBEdit_10_User_Manual.pdf）を検索結果の要約で読んだもので、原文は開けなかった（未確認）。
- CotEditor: リリースノート https://coteditor.com/releasenotes/7.0.0-beta.en （`CurrentLine` と `ReplaceCurrentLine`）、https://coteditor.com/releasenotes/7.0.2.en （大きな入出力をパイプで扱う信頼性の改善）、https://coteditor.com/releasenotes/4.0.0-beta.en （標準エラーのコンソール表示、UTF-8 固定、同じ標準入力が繰り返し送られる問題の修正）。スクリプトのヘッダーの書き方（入出力の指定方法）はアプリ内ヘルプにあり、web からは読めなかった（未確認）。
- Kate Text Filter: https://docs.kde.org/trunk_kf6/en/kate/kate/kate-application-plugin-textfilter.html 。Kate External Tools: https://docs.kde.org/trunk_kf6/en/kate/kate/kate-application-plugin-external-tools.html 。
- EmEditor: https://www.emeditor.com/text-editor-features/extensibility/external-tools/ （入力は文書全体または None、出力は文書を置換 / アウトプットバー、標準エラーは Discard にできる、エンコーディングで UTF-8 を選ぶ例、作業フォルダ `$(Dir)`）。選択範囲、終了コードについては記載なし。
- サクラ: 上流の `help/sakura/res/HLP000103.html`。「標準入力に送る: 範囲選択中であれば、選択範囲のみを渡します」、文字コードは SJIS / Unicode / UTF-8、出力先の動作、「カレントディレクトリ」の項目。
- JetBrains: https://www.jetbrains.com/help/idea/settings-tools-external-tools.html （Program, Arguments, Working directory, Open console for tool output など。stdin や選択範囲の置換の欄はない）。
- VS Code 拡張: https://raw.githubusercontent.com/yhirose/vscode-filtertext/master/README.md 、https://raw.githubusercontent.com/ryu1kn/vscode-edit-with-shell/master/README.md （どちらも README のみ読んだ）。

### 1.3 短い注記

- 「登録した一覧をメニューから選ぶ」型は、VS Code 拡張 2 つ、Kate External Tools、TextMate（バンドル）、BBEdit（フォルダ）。うち「設定ファイルに名前とコマンド 1 行」だけで済むのは VS Code 拡張の 2 つ。
- 出力先に「パネル」を持つのは Kate External Tools（Display in Pane）、EmEditor（アウトプットバー）、JetBrains（コンソール）、サクラ（アウトプットウィンドウ）。「次の行に挿入」に当たるものは、TextMate の「Insert as Text」、Kate の「Insert at Cursor Position」「Append to Current Document」、Edit with Shell の挿入。文書上、「選択範囲の次の行」ちょうどの挿入は見つからなかった。
- 選択なしのときの扱いは、TextMate が予備の単位を選べる、BBEdit が文書全体、Edit with Shell が設定で全文にできる、Filter Text が挿入のみ、サクラが全体、と分かれる。今回の機能（選択なしは文書全体）は BBEdit とサクラに合う。
- Kate Text Filter は stderr を「混ぜる / 受動メッセージで見せる」の 2 択にしている。MATSU で出力を置換にするなら、stderr を混ぜると結果にエラー文が入る。

---

## 2. 各シェルで 1 行を stdin つきで実行する（公式 docs）

| シェル | 1 行の渡し方 | 起動ファイル | stdin の行き先 | 注意 |
|---|---|---|---|---|
| sh（POSIX） | `sh -c 'cmd'` | 非対話・非ログインなら読まない（`ENV` は対話シェルのみ。未確認） | コマンドの stdin にそのまま届く | |
| bash | `bash -c 'cmd'` | 非対話で起動すると `BASH_ENV` が指す文字列があればそのファイルを読む。`--login` を付けると `/etc/profile` と `~/.bash_profile`（なければ `~/.bash_login`、`~/.profile`）を読む。`--norc` で `~/.bashrc` を読まない | そのまま | 手元の WSL では `bash -c` 3 ms、`bash -l -c` 8 ms |
| zsh | `zsh -c 'cmd'` | `/etc/zshenv` と `$ZDOTDIR/.zshenv` は常に読む（`-f` なしの場合）。ログインシェルなら `.zprofile`、対話なら `.zshrc`、ログインなら最後に `.zlogin`。つまり `-c` だけでは `.zprofile` も `.zshrc` も読まず、`-l -c`（ログイン）なら `.zprofile` と `.zlogin` を読む（`.zshrc` は対話でないので読まない） | そのまま | macOS で PATH を `.zprofile` などで足しているなら、`-c` だけでは通らない。`-l -c` が要る。ただしログイン用ファイルが重い・出力を出す人もいる。WSL に zsh がなく、実測はしていない |
| fish | `fish -c 'cmd'`（`-C` で `-c` の前にコマンドを足せる） | `-c` でも設定ファイル（config.fish）を読む。`-N` または `--no-config` で読まない。`-l` でログインシェル扱い | そのまま（WSL に fish がなく実測していない） | |
| PowerShell 7（pwsh） | `pwsh -NoProfile -Command "<文字列>"`、または `-EncodedCommand`（UTF-16LE の Base64）、`-CommandWithArgs` | `-NoProfile` でプロファイルを読まない。`-NoLogo`、`-NonInteractive` も付けられる | `-Command` の文字列の中で `$input`（パイプラインへの入力）、または `[Console]::In` で読む。`-Command -` と `-File -` は「スクリプトを stdin から読む」意味になるので、テキスト用の stdin とは両立しない | `-Command` は最後のパラメータでなければならない（後ろの引数がすべてコマンドの一部になる）。終了コードは、最後のコマンドの `$?` が真なら 0、偽なら 1。外部プログラムの 2 以上は 1 に丸められるので、`exit $LASTEXITCODE` を足す |
| Windows PowerShell 5.1 | `powershell.exe -NoProfile -Command "<文字列>"`、`-EncodedCommand`（UTF-16LE の Base64） | 同上 | 同上 | 5.1 の docs も `-Command -` は stdin 読み込み |
| cmd.exe | `cmd.exe /D /S /C "<文字列>"` | `/D` で AutoRun（レジストリ）を読まない | 子プロセスの stdin にそのまま届く | `/S` は `/C` と併用すると、最初と最後の `"` を取り除いて残りはそのまま渡す。`/S` なしでは、引用符を残す条件が細かい（引用符が 1 組だけ、特殊文字を含まない、空白を含む、実行ファイル名、のすべてを満たすとき）ので、`/S` を付けて全体を `"` で囲むのが安全 |

出典:
- PowerShell 7: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pwsh?view=powershell-7.5 （`-Command`、`-Command -`、終了コード、`-EncodedCommand`、`-NoProfile`、`-NonInteractive`）。
- Windows PowerShell 5.1: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_powershell_exe?view=powershell-5.1 。
- cmd: https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/cmd （`/s`、`/d`、引用符の規則、特殊文字は `^` か引用符で）。
- zsh: https://zsh.sourceforge.io/Doc/Release/Files.html （起動ファイルの順序）。
- bash: https://www.gnu.org/software/bash/manual/html_node/Bash-Startup-Files.html （非対話、`--login`、`BASH_ENV`、`--norc`）。
- fish: https://fishshell.com/docs/current/cmds/fish.html （`-c`、`-C`、`-N`、`-l`）。
- PowerShell の文字コード: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding?view=powershell-7.5 （PowerShell 6 以降は出力を既定で utf8NoBOM）、https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_preference_variables?view=powershell-7.5 （`$OutputEncoding` は既定 UTF8Encoding、「ネイティブアプリへパイプで渡すときの方式」）。

### 引用符の罠

- PowerShell 7.3 以降は `$PSNativeCommandArgumentPassing`（Windows の既定は `Windows`）。cmd.exe、cscript.exe、find.exe、sqlcmd.exe、`.bat`/`.cmd` などには旧来の動作を使う。docs は「バッチファイルへは生のコマンドライン文字列が cmd.exe に渡る。信頼できない入力は別の方法で渡せ」と警告している（上の pref 文書）。
- `pwsh -Command` に複雑な引用符を渡すなら `-EncodedCommand` が公式の逃げ道。5.1 でも実測で動いた（下の 3.8）。
- Windows 上で子プロセスを `CreateProcess` で起動する場合、コマンドライン全体が 1 本の文字列になるので、`cmd.exe` と他のシェルとで引用符の解釈が違う。今回の WSL 経由の実験では、WSL 側が引数を組み直すので、cmd の引用符の挙動は判断できなかった（`'"findstr /n "h""'` が壊れたが、WSL の引数変換の影響と推測。未確認）。MATSU は Windows のネイティブから起動するので、Sandbox で別途確かめる必要がある。
- ワンライナーを cmd.exe に渡すとき、`%` は cmd が展開する。`&`、`|`、`^` も特別。実測（WSL 経由）で `echo 100%PATH:~0,0%^&x` が `100&x` になり、`%PATH:~0,0%` が空に展開され `^&` が `&` になった。
- sh 系は、1 行にシングルクォートが入ると組み立てが難しい（MATSU は 1 行を丸ごと `-c` の引数に渡すので、組み立て不要で問題は起きない。罠はユーザーが書くコマンドの側）。

---

## 3. この PC での実験（Windows on Arm64、WSL2 から）

環境: `uname -m` は aarch64。cmd.exe のコードページは 932（`chcp` の出力）。powershell.exe は `5.1.26100.9974`。`pwsh.exe` は探したが、見つからなかった（`/mnt/c/Program Files/PowerShell/7/`、`WindowsApps`、`where pwsh` のいずれにもない）。したがって pwsh 7 の実測は、すべて未確認。pwsh 7 については docs の記述だけで書く。zsh と fish も WSL に入っていない。

入力はすべて `printf 'いろは\nabc\nあ😀\n'`（UTF-8、LF）。バイト列は `e3 81 84 e3 82 8d e3 81 af 0a 61 62 63 0a e3 81 82 f0 9f 98 80 0a`。実行は `cd /mnt/c/Windows` から（UNC のカレントでは cmd が動かないため）。

### 3.1 各 Windows シェルの既定の挙動（Windows PowerShell 5.1）

起動直後の設定（`... -Command '[Console]::InputEncoding.CodePage; [Console]::OutputEncoding.CodePage; $OutputEncoding.CodePage'`、stdin を `/dev/null`）: 932、932、20127（US-ASCII）。

```
printf "$IN" | powershell.exe -NoProfile -Command '$input | Sort-Object' | od -c
→ a b c \r \n 343 201 201 E 202 215 343 201 257 \r \n 343 201 202 360 237 230 200 \r \n
```
- 3 行目（`あ😀`）は戻ったが、1 行目は壊れた（`い` の `e3 81 84` が `e3 81 81`、`ろ` の `e3 82 8d` が `45 82 8d`）。`$input` は CP932 として読み、CP932 として書き出すので、UTF-8 のバイトがおおむねそのまま通り抜けるが、CP932 として不正なバイト列と行区切りの `0x0a` が組み合わさると壊れる。ソートも壊れた文字列に対して行われている。
- `$input | ForEach-Object { $_.ToUpper() }` は `e3 81 81 45 82 6c e3 81 af`（`82 8d` が `82 6c` に変わった。大文字化が壊れたバイトを書き換える）。
- 各行に CRLF が付く（入力は LF）。BOM なし。

`$input | sort.exe`: `3f 3f 3f 3f 0d 0a ...`（`?` だらけ。`$OutputEncoding` が US-ASCII のため、PowerShell が子プロセスに渡す時点で `?` になった）。

### 3.2 UTF-8 を設定した場合（5.1）

設定: `$u=New-Object Text.UTF8Encoding $false; [Console]::InputEncoding=$u; [Console]::OutputEncoding=$u; $OutputEncoding=$u;`

| やり方 | 結果 |
|---|---|
| `$u...; $input \| Sort-Object` | 入力が文字化けしたまま（`0xe7 0xb8 0xba 0xe3 0x83 0xbb ...`）。`$input` は起動時に決まった CP932 で読むので、あとから `InputEncoding` を変えても効かない |
| `$u...; [Console]::In.ReadToEnd()` | 入力どおり（`e3 81 84 ... f0 9f 98 80 0a`）。そのあと PowerShell が `\r\n` を 1 つ足す |
| `$u...; $t=[Console]::In.ReadToEnd(); $t -split "\`r?\`n" \| ? {$_} \| Sort-Object` | UTF-8 で正しい（`abc`、`あ😀`、`いろは` の順。コードポイント順）。各行 CRLF |
| `$u...; $t=[Console]::In.ReadToEnd(); [Console]::Out.Write($t.ToUpper())` | `いろは\nABC\nあ😀\n`（LF のまま、改行を足さない、正確） |
| `$u...; ... $t \| findstr /n .` | UTF-8 のまま行番号付きで出る（`1:いろは` など）。LF のまま |
| `$u...; ... $t \| sort.exe` | 日本語が壊れる（sort.exe は CP932 として扱う） |
| `$u...; ... $t \| findstr あ` | 空（コマンドライン引数の `あ` が CP932 で渡され、UTF-8 の入力と一致しない） |
| `-Command '$r=New-Object IO.StreamReader([Console]::OpenStandardInput(),(New-Object Text.UTF8Encoding $false)); $r.ReadToEnd()'` | 読み取りは正しいが、出力が `[Console]::OutputEncoding`（既定 CP932）で書かれ、`82 a2 82 eb 82 cd 0a 61 62 63 0a 82 a0 3f 3f 0a`（絵文字は `??`） |

ラッパーとして使える形（1 行を `& { ... }` に包んで、`$input` に UTF-8 で読んだ行を流す）:
```
$u=New-Object Text.UTF8Encoding $false;[Console]::InputEncoding=$u;[Console]::OutputEncoding=$u;$OutputEncoding=$u;$t=[Console]::In.ReadToEnd();$t -split "`r?`n" | & { $input | Sort-Object }
→ UTF-8 で正しい（先頭に空行が入る。`-split` が末尾の空要素を作るため。`? {$_}` で落とせるが、空行のある入力が壊れる）
```
このとき、ユーザーの 1 行は `$input` を使って書ける。ただし行単位に分けるので、「テキスト全体を 1 つの文字列として扱いたい」ワンライナーは `$input` ではなく `$t` が要る。ワンライナーの書き方を MATSU が決める必要がある（下の 4 章）。

- 入力に BOM（`ef bb bf`）があると、`ReadToEnd()` は BOM をそのまま文字として返す（`printf '\xef\xbb\xbfabc\n' | ... Out.Write($t)` が `ef bb bf 61 62 63 0a`）。
- `[Console]::OutputEncoding=[Text.Encoding]::UTF8` と書いても、実測で出力に BOM は付かなかった（`a\r\n` のみ）。BOM が付くのは、ファイルへ書くときの `Out-File -Encoding utf8` 側（5.1 の docs による。実測していない）。

### 3.3 cmd.exe

```
printf "$IN" | cmd.exe /D /S /C "sort"
→ a b c \r \n 343 201 201 E 202 215 343 201 257 \r \n 343 201 202 360 237 230 200 \r \n   （1 行目が壊れる。CP932 として並べ替えた）
printf "$IN" | cmd.exe /D /S /C "chcp 65001 >nul & sort"   （同じ結果。sort は stdin のコードページを使わないようだ）
printf "$IN" | cmd.exe /D /S /C "chcp 65001 >nul & more"
→ a b c \r \n \r \n   （日本語の行が消えた）
printf "$IN" | cmd.exe /D /S /C "more"
→ ...壊れた1行目 \r\n abc \r\n あ😀 \r\n \r\n   （LF が CRLF に変わり、末尾に空行が付く）
printf "$IN" | cmd.exe /D /S /C "findstr /n ."          （どちらの chcp でも）
→ 1:いろは\n2:abc\n3:あ😀\n   （UTF-8 のバイトがそのまま通る。LF のまま）
```
- つまり cmd 側は、コマンドごとに挙動が違う。`findstr` はバイトを素通しする。`sort` と `more` はコードページに依存する（`chcp 65001` を先に実行しても `sort` は直らず、`more` は日本語行が消える）。`chcp` が子プロセスのコンソールコードページを変えても、その効果がパイプ入出力に及ぶかは外部プログラム次第。
- 入力が `abc`（末尾改行なし）の `sort` は `abc\r\n` を返す（改行が付く）。
- cmd の終了コード: `cmd /D /S /C "exit /b 7"` は 7。`findstr zzz & echo after` は `after` を出して 0（最後の `echo` の結果）。
- 文書: cmd の `/U` は出力を Unicode にする（cmd docs の `/u`）。`/A` は ANSI。今回は試していない（未確認）。

### 3.4 PowerShell の外部プログラム

`$input | sort.exe`、`$t | sort.exe`（UTF-8 設定あり）のどちらも日本語が壊れる（3.1、3.2）。`findstr` は UTF-8 設定ありだとバイトが通る（3.2）。外部プログラムを使うワンライナーでは、PowerShell の `$OutputEncoding` と、外部プログラムが想定するコードページの両方が合う必要がある。

### 3.5 末尾改行

- PowerShell 5.1: 入力 `abc`（改行なし）で `$input | % { $_.ToUpper() }` は `ABC\r\n`（改行が付く）。`[Console]::Out.Write("x")` は `x` のみ。
- cmd `sort`: `abc` → `abc\r\n`。
- WSL の `bash -c 'tr a-z A-Z'`: `abc` → `ABC`（改行なし。tr は足さない）。
- WSL の `sort`: 入力 `いろは\nabc\nあ😀\n` → `abc\nあ😀\nいろは\n`（LF のまま）。`sh -c sort` と `bash -c sort` で同じ。

### 3.6 stderr と終了コード（5.1）

```
printf abc | powershell.exe -NoProfile -Command 'Write-Error boom; "ok"; exit 3'   → rc=3、出力に装飾付きのエラー文
   Write-Error boom; "ok"; exit 3 : boom
       + CategoryInfo          : NotSpecified: (:) [Write-Error], WriteErrorException
       + FullyQualifiedErrorId : Microsoft.PowerShell.Commands.WriteErrorException
   ok
printf abc | ... -Command 'cmd /c exit 5'          → rc=1（元のコード 5 は 1 に丸められる）
printf abc | ... -Command 'cmd /c exit 5; "x"'     → rc=0（最後が成功）
printf abc | ... -Command 'nonexist'               → rc=1、日本語のエラー文（CP932）+ CategoryInfo 等
```
- エラー文は stderr に出る（上は `2>&1` で見ている）。エラーのテキストは複数行の装飾付きで、日本語ロケールでは CP932 のメッセージが出る。出力先を置換にしたとき、stderr を混ぜると、結果にこの装飾が入る。
- `-Command` の終了コードの丸めは、pwsh 7 の docs と同じ記述（`exit $LASTEXITCODE` を足す）。

### 3.7 起動時間（trivial な 1 行、5 回の中央値。WSL の interop 込み）

| コマンド | 5 回（ms） | 中央値 |
|---|---|---|
| `cmd.exe /D /S /C "echo 1"` | 29 29 29 33 34 | 29 ms |
| `powershell.exe -NoProfile -Command 1` | 152 154 157 160 162 | 157 ms |
| `powershell.exe -Command 1`（プロファイルあり指定） | 141 150 156 159 163 | 156 ms |
| `powershell.exe -NoProfile -NonInteractive -Command 1` | 147 151 153 153 156 | 153 ms |
| WSL `sh -c :` | 2 2 2 2 2 | 2 ms |
| WSL `bash -c :` | 2 3 3 3 3 | 3 ms |
| WSL `bash -l -c :` | 8 8 8 9 15 | 8 ms |
| WSL `bash --norc -c :` | 2 3 3 3 3 | 3 ms |

- この PC には Windows PowerShell のプロファイル（`$PROFILE.CurrentUserAllHosts` は `False`）が存在せず、`-NoProfile` の有無の差は出ない。プロファイルが重い利用者では差が出る（推測）。実行ポリシーは `RemoteSigned`。
- pwsh 7 は未測定。一般に Windows PowerShell 5.1 より起動が遅いことが多いが、確かめていない（推測）。
- interop の起動コストが入っているので、Windows のネイティブ起動での数字は、これより小さいはず（推測）。macOS の zsh は測っていない。

### 3.8 引用符、`-EncodedCommand`、stdin を閉じた場合

- `-EncodedCommand`（`printf '$input | % { "[" + $_ + "]" }' | iconv -f utf-8 -t utf-16le | base64 -w0`）は、stdin のテキスト（`a\nb\n`）を `$input` で受け取って動いた（`[a]\r\n[b]\r\n`）。引用符の問題が出ない。
- stdin を `/dev/null` にした `$input | % {$_}; "done"` は、すぐに `done` を出して終わる。
- stdin が何も来ない（書き手が 8 秒眠る）状態で cmd の `sort` を走らせると、タイムアウト（`timeout 5`）まで待ち続けた（rc=124）。つまり、入力を閉じない限り、読む側のコマンドは終わらない。エディタが stdin を閉じることが必須。

### 3.9 大きな入力（重要な注意）

| 条件 | 結果 |
|---|---|
| `cat 2MB \| cmd /D /S /C 'findstr /n .'` | 0.05〜0.06 秒、出力 2108894 バイト（正常） |
| 同 3MB、4MB、5MB、10MB、20MB、50MB | いずれも 14 秒（3MB）、15 秒（4MB）、30〜40 秒（5MB 以上）たっても終わらず、`timeout` で切った |
| `cat 5MB \| cmd /D /S /C 'findstr /n .' > ファイル` | 30 秒で終わらない（出力先をファイルにしても同じ） |
| `cat 5MB \| cmd /D /S /C 'more'` | 30 秒で終わらない |
| `cat 5MB \| cmd /D /S /C 'findstr /c:zzz'`（出力なし） | 0.05 秒で終わる |
| `cat 5MB \| powershell.exe -NoProfile -Command '[Console]::In.ReadToEnd().Length'` | 0.23 秒、5000000 |
| `cat 5MB \| powershell.exe -NoProfile -Command '[Console]::Out.Write([Console]::In.ReadToEnd())'` | 0.27 秒、5000000 バイトを返した |
| `cat 2MB \| powershell.exe -NoProfile -Command '$input \| Measure-Object -Line \| % Lines'` | 0.32 秒 |
| WSL `cat 5MB \| sh -c 'cat -n' \| wc -c` | 0.01 秒、5350000 |

- cmd + `findstr`/`more` の 3 MB 以上で固まるのは、入力を読みながら出力も書く外部プログラムを cmd 経由で走らせたときだけだった。PowerShell は「全部読んでから全部書く」形なら 5 MB でも問題がなかった。原因は未確認。WSL の interop の中継（Linux 側のパイプと Windows 側のパイプの間）の問題か、cmd と findstr の組み合わせの問題かは区別できていない（推測: interop 側）。MATSU は Windows のネイティブからパイプで起動するので、Windows Sandbox で同じ実験を繰り返して確かめる必要がある。
- 少なくとも、この環境で分かったこととして、エディタは「入力を書く」と「出力を読む」を同時に行い（別スレッド）、タイムアウトとキャンセルを持つべき。

### 3.10 pwsh 7 と powershell 5.1 の違い（docs のみ、実測はない）

- `$OutputEncoding` の既定: 7 は UTF8Encoding（pref 文書）。5.1 は実測で US-ASCII。
- 出力ファイルの既定エンコーディング: 7 は utf8NoBOM（about_Character_Encoding）。5.1 の cmdlet は Default（ANSI）や Unicode が混在する（同文書の記述）。
- `$PSNativeCommandArgumentPassing`（7.3 以降）。5.1 にはなく、旧来の引数の渡し方。
- `-CommandWithArgs` は 7 にだけある（about_pwsh の記述）。
- 終了コードの丸め、`-Command -` の意味は同じ。
- コンソールの入出力コードページ（`[Console]::InputEncoding` など）の既定が 7 でどうなるかは、docs では確認できなかった（未確認。日本語 Windows では CP932 のままと推測）。

### 3.11 WSL の sh と bash（Linux であって macOS ではない）

- `sh`/`bash`: UTF-8 のバイトが素通しで、改行は入力どおり（LF）。起動は 2〜3 ms（`bash -l` で 8 ms）。
- zsh と fish は入っていないので未測定。
- macOS のログインシェル（zsh）の実測は別途（Mac Sandbox）必要。

---

## 4. ADR で決める必要があること（事実と選択肢。推奨はしない）

### 4.1 文字コード（シェルごと）

事実:
- Windows PowerShell 5.1 は、`$input` が起動時のコンソール入力コードページ（CP932）で読む。あとから `InputEncoding` を変えても `$input` には効かない（3.2）。`[Console]::In.ReadToEnd()` は効く。`$OutputEncoding` は US-ASCII で、外部プログラムへのパイプが `?` になる（3.1）。
- pwsh 7 は `$OutputEncoding` が既定で UTF-8（docs）。コンソール入出力のコードページは未確認。
- cmd は `chcp 65001` でも `sort` と `more` は直らない。`findstr` はバイトを素通しする（3.3）。
- sh 系は UTF-8（ロケール依存。`LANG` が C のときの挙動は未確認）。
- サクラの既存実装は SJIS / Unicode / UTF-8 を選べる（HLP000103）。

選択肢:
- A. すべてのシェルへ UTF-8 で渡し、UTF-8 で読む。PowerShell には先頭に設定の文字列を足す（`[Console]::InputEncoding`、`OutputEncoding`、`$OutputEncoding`）。cmd には `chcp 65001 >nul &` を足す（外部プログラム次第で効かない）。
- B. ワンライナーごとに入出力の文字コードを持たせる（サクラの外部コマンド実行に近い。SJIS / UTF-8 / システムの ANSI など）。
- C. シェルごとの既定（Windows のシェルは CP932、ほかは UTF-8）にして、設定で変えられるようにする。
- 付随: 文書が UTF-8 以外のとき、エディタの内部は Unicode なので、送るときに変換する。変換できない文字（CP932 に絵文字がない）が出たときの扱い（エラー、`?`、そのまま）を決める必要がある。
- 付随: 先頭の設定文字列の足し方は「ユーザーの 1 行の前に `;` でつなぐ」形になる。ユーザーの 1 行が `exit` や構文エラーを含むとき、前置きの動作にどう影響するかを決める。
- 付随: PowerShell で `$input` を使えるようにするには、`$input` ではなく `[Console]::In.ReadToEnd()` で読み、`& { ... }` でラップする（3.2）。ユーザーに `$input` を書かせるか、`$t`（全文の文字列）を用意するかを決める。

### 4.2 CRLF と LF

事実:
- PowerShell は出力の行ごとに CRLF を付ける（3.1、3.5）。`[Console]::Out.Write` なら付けない。
- cmd の `more`、`sort` も CRLF にする。`findstr` は LF のまま。
- Linux のシェルは LF のまま。
- エディタの文書の改行コードが CRLF の場合、サクラは（既存実装）どう渡しているかは本調査の対象外。

選択肢:
- 入力: 文書の改行コードのまま渡す / 常に LF に正規化して渡す / シェルごとに（Windows のシェルには CRLF）。
- 出力: そのまま受けて文書の改行コードに揃える / 出力の CRLF と LF の混在を LF に統一してから文書の設定に合わせる。
- 文書の改行コードを維持するか、出力の改行コードに従うか。

### 4.3 末尾改行

事実:
- 入力に末尾改行がなくても、PowerShell と cmd の `sort` は `\r\n` を付ける（3.5）。`tr` は付けない。
- 選択範囲が行の途中までのとき、出力が改行で終わると、次の行が詰まって見える。
- BBEdit、TextMate、Kate の文書には、末尾改行の扱いの記述がなかった（未確認）。

選択肢:
- 出力の末尾改行を、入力の末尾改行の有無に合わせる（入力になければ 1 つ削る）。
- そのまま採用する。
- 設定でワンライナーごとに選ぶ（「末尾の改行を取り除く」）。
- 出力先が「次の行に挿入」のときは、出力の末尾改行の有無に関わらず行として挿入する（必要なら改行を補う）。

### 4.4 stderr と終了コード

事実:
- BBEdit は非ゼロ終了なら何もしないでエラーを出す（12.6.1）。Vim は stderr を出力に混ぜる既定（'shellredir'）。Emacs も既定で混ぜる。Kate Text Filter は選択制。
- PowerShell は `-Command` の終了コードが 0 か 1 に丸められる（外部プログラムの 2 以上が 1 になる）。`exit $LASTEXITCODE` を足せば保てる（docs、実測）。途中の失敗は拾えない。
- PowerShell の stderr は装飾付きの複数行（3.6）。
- cmd の終了コードは最後のコマンドのもの。

選択肢:
- 非ゼロ終了のとき: 何も変更せずにエラーを出力パネルに出す（BBEdit 型）/ 結果を反映してエラーを通知する / 設定で選ぶ。
- stderr: 混ぜる / 分けて、出力パネルかメッセージに出す（Kate 型）。出力先が「置換」のときは、混ぜると結果にエラー文が入るので、分けるのが安全、という考え方がある。
- 終了コードの取り方: PowerShell 用に `; exit $LASTEXITCODE` を自動で足すかどうか。

### 4.5 入力を待ち続けるコマンド、終わらないコマンド

事実:
- stdin を閉じない限り、読む側は終わらない（3.8）。エディタが全部書いたら stdin を閉じる必要がある。
- stdin を読まないコマンドに全文を書き込もうとすると、パイプが詰まって書き込みが固まりうる（一般的な事実。この環境では未再現）。子プロセスが先に終わったとき、書き込み側が `EPIPE` や「パイプが閉じられた」を受け取る。
- 端末を要求するコマンド（`Read-Host` など）。PowerShell は `-NonInteractive` で、対話が必要になるとエラーにして固まらない（docs）。
- 子プロセスが孫プロセスを作って、孫が stdout を握り続けると、エディタが EOF を受け取れない（一般的な事実。未確認）。

選択肢:
- タイムアウトを設ける（既定値を決める）/ キャンセルのボタンを出す / 両方。
- キャンセル時にプロセスツリーごと終了させるか（Windows はジョブオブジェクト、Unix はプロセスグループ。実装の話だが挙動は ADR に影響する）。
- stdin の書き込みと stdout/stderr の読み取りを別スレッドにする（必須と考える）。
- PowerShell に `-NonInteractive` を足す。

### 4.6 大きな入力と出力

事実:
- 3.9 の結果。入力・出力の両方が大きいと、この環境の cmd 経由では 3 MB で固まった。
- CotEditor 7.0.2 は「大きな入出力をパイプで扱うときの信頼性を改善」とリリースノートに書いている。同種の問題が実際に起きる、という例。
- プロジェクトのルールで、全文を 1 つの文字列で渡す API を作らない（CLAUDE.md）。ワンライナーの入力は範囲指定で読み出して、チャンクで書き込める。

選択肢:
- 入力と出力の上限サイズ（超えたら実行しない / 警告）。
- 出力をストリームで受けて、一定量を超えたら打ち切る。
- 出力を一旦メモリに全部溜めてから置換する（undo の 1 ステップのため、置換は最後にまとめて行う必要がある）。

### 4.7 作業フォルダ

事実:
- サクラは「カレントディレクトリ」を指定できる（HLP000103）。Kate External Tools は空のとき、現在の文書のパス。EmEditor は `$(Dir)`。Edit with Shell は `currentDirectoryKind`（`currentFile` か `workspaceRoot`、見つからなければ HOME）。JetBrains は Working directory の欄。
- 未保存の文書には「文書のフォルダ」がない。

選択肢:
- 文書のフォルダ（未保存ならホームか、プロジェクトのルートか、エディタの作業フォルダ）。
- ワンライナーごとの設定にする。
- 固定（利用者のホーム）。

### 4.8 環境変数

事実:
- Edit with Shell は選択範囲を `ES_SELECTED` 環境変数に入れる（README）。Kate はテキストを引数に展開する変数（`%{Document:Selection:Text}`）を持つ。今回の機能はテキストをコマンドラインにも環境変数にも入れず stdin だけにする（決定済み）。
- CLAUDE.md の決まり: `MATSU_` で始まる環境変数に秘密を入れない（ADR-0028）。
- シェルタブと同じシェルで動かす決定なので、シェルタブが子プロセスへ渡す環境変数と揃えるか。
- 環境変数で渡す案を採ると、サイズ上限（Windows は 1 変数あたり約 32 KB）と、他プロセスから見える点で、テキストを渡す用途に向かない（一般的な事実。未確認）。

選択肢:
- 何も足さない（エディタ自身の環境を継承する）。
- 文書のパスや選択範囲の位置などを `MATSU_*` の環境変数で渡す（テキストの内容は渡さない）。
- 文書のパス、ファイル名、選択範囲の行番号などを渡すか、一切渡さないか。

### 4.9 シェルの設定が「コマンドライン全体」のとき

事実:
- 「利用者はシェルを `cmd.exe` や `bash` に変えられる」。シェルタブ用の設定なので、`pwsh.exe -NoLogo` や `cmd.exe /K ...` のように引数付きで書かれることがある。
- 他のエディタの対処:
  - Vim: 'shell' に引数を許す（"csh -f" の例が docs にある）。そのうえで 'shellcmdflag' を別の設定にしている（既定 `-c`、Win32 で `powershell` を含むと `-Command`、`sh` を含まないと `/c`）。'shellredir' も別設定（シェルの名前で既定が変わる）。
  - Edit with Shell: `shell.*`（パス）と `shellArgs.*`（引数の配列、既定 `["-c"]`）を別の設定にしている。
  - VS Code のターミナルは `args` を別に持つ（本調査では読んでいない。未確認）。
- 「1 行を実行する」フラグはシェルごとに違う: sh/bash/zsh/fish は `-c`、PowerShell は `-Command`（または `-EncodedCommand`）、cmd は `/D /S /C`。`pwsh.exe -NoLogo` のような設定へ `-Command` を足すのは、`-Command` が最後のパラメータでなければならないので、末尾に足せばよい（pwsh の docs）。cmd の `/K` は「実行して残る」なので、`/C` と置き換えが要る。`/K` と `/C` が両方あるときの扱いは、docs で確認していない（未確認）。
- シェルタブ向けの引数（`-NoExit` や `-Interactive`、`-Login`）が混ざっていると、ワンライナーの実行には不都合なものがある。

選択肢:
- A. シェルの設定は実行ファイルのパスだけにして、引数は別の設定にする（Edit with Shell 型）。ワンライナー専用の引数（1 行を実行するフラグ）は、MATSU が実行ファイル名から判別して足す（`pwsh`、`powershell`、`cmd`、`bash`、`zsh`、`fish`、`sh`。そのほかは `-c`）。
- B. 設定の全文を引数に分けて（シェルの引用符規則で）解析し、先頭の実行ファイル名から判別したフラグを末尾に足す。既存の引数（`-NoLogo`）は残す。`-NoExit` など実行に不都合なものを取り除くかは別に決める。
- C. ワンライナー専用のシェル設定（実行ファイル + 1 行実行用の引数）を別に持つ。
- 実行ファイル名の判別を、拡張子つき・パス付き（`C:\...\pwsh.exe`、`/bin/zsh`）にも通す必要がある。判別できない場合の既定（POSIX の `-c`）を決める。
- ログインシェル（macOS の zsh は `-l -c` が必要になる場合がある。2 章）をどうするかも決める。
- PowerShell の起動フラグとして `-NoProfile` を付けるか（速度、挙動の再現性）、付けないか（利用者のプロファイルの設定、PATH が効く）。シェルタブと同じ環境を保ちたい決定とぶつかる。
- 検索コスト: 見つからない場合のフォールバック（pwsh → powershell 5.1）。

---

## 付録: 実験の限界

- 実験はすべて WSL から Windows の exe を呼ぶ形（interop 経由）で、Windows のネイティブからの起動ではない。stdin の文字コードの扱いとパイプの挙動は、interop の影響がありうる（特に 3.9）。
- pwsh 7、zsh、fish、macOS は実測していない。
- 起動時間は interop の起動コスト込み。
- 本調査では、他エディタのソースコードは読んでいない（文書、README、リリースノートのみ）。
- 一時ファイルは調べた PC の作業用フォルダに置いた（取得した文書のテキストと 2〜50 MB の試験データ）。リポジトリは変更していない（書いたのはこのファイルだけ）。
