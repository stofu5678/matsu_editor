# シェル

シェル（端末）を、エディタの文書とは別のタブで動かします。PowerShell、zsh、bash などの対話的なプログラムを、MATSU Editor の中で使えます。

## 開き方

- メニュー: 「ウィンドウ」の「シェルのタブを開く」
- 機能名: `F_SHELL_TAB`
- マクロ: `ShellTab()`
- 既定のキー: なし。キー割り当ての設定で割り当てられます

実行するたびに、新しいシェルのタブが、いま見ているタブの隣に開きます。

シェルは、開いた時点で見ていた文書のあるフォルダーで始まります。ファイルのない文書や、フォルダーが消えていたときは、ホームフォルダーで始まります。文字の大きさ、書体、色は、エディタの文書と同じです。

開いただけでは、シェルに何も送りません。

## どのシェルが動くか

共通設定の「シェル」ページの「コマンドライン」が空のときは、既定のシェルが動きます。

| 環境 | 既定のシェル |
|---|---|
| Windows | `pwsh.exe`（PowerShell 7）が見つかればそれ。なければ Windows PowerShell |
| macOS | ログインシェル（`/usr/bin/login` 経由） |
| Linux | 環境変数 `SHELL` のシェル。なければ利用者の登録にあるシェル |

シェルには、MATSU Editor の環境変数に `TERM=xterm-256color` と `COLORTERM=truecolor` を足して渡します。MATSU Editor 自身の環境は変えません。

> `MATSU_` で始まる環境変数も、シェルの中で見えます。秘密（トークン、パスワード、API キー）を `MATSU_` の変数に入れないでください。

## 使い方

### キー

シェルのタブでは、キーはほとんどシェルの中のプログラムに送られます。

| 操作 | キー |
|---|---|
| 貼り付け | `Ctrl+V`（macOS は `Cmd+V`）。Windows と Linux では `Ctrl+Shift+V` も使えます |
| コピー | `Ctrl+Shift+C`（macOS は `Cmd+C`） |
| 選んでいればコピー、選んでいなければ中断 | `Ctrl+C`（Windows と Linux） |
| 履歴を 1 行ずつさかのぼる | `Ctrl+Shift+↑` / `Ctrl+Shift+↓` |
| 履歴を 1 ページずつさかのぼる | `Ctrl+Shift+PageUp` / `Ctrl+Shift+PageDown` |
| いちばん古い行 / 現在の画面 | `Ctrl+Shift+Home` / `Ctrl+Shift+End` |
| タブを閉じる（macOS のみ） | `Cmd+W` |

次の 2 つも覚えておいてください。

- `Alt` と文字のキーは、メニューを開かず、プログラムに渡ります。メニューは、メニューバーをマウスで選んで使います。
- 「ウィンドウ」グループの機能（次のウィンドウ、タブを閉じる、など）のキーは、シェルを見ているときもウィンドウが受け取ります。ただし `F5`（再描画）と、画面の分割のキーはプログラムに渡します。

日本語の入力（IME）や、デッドキーも使えます。macOS では、設定で「Option を Meta として使う」を選ぶと、`Option` と文字のキーで、ESC に続けて文字を送れます。

### 選択とコピー

- ドラッグで、セルを選びます。
- ダブルクリックで単語、トリプルクリックで行を選びます。
- コピーすると、選択は解けます。クリップボードには、OS の改行コードで入ります。

### 貼り付け

貼り付けるとき、MATSU Editor は次のようにします。

- 改行とタブ以外の制御文字を取り除きます。ESC も取り除くので、貼った文字列に命令を紛れ込ませることはできません。
- プログラムが「括弧付き貼り付け」を求めているときは、貼った文字列を印で囲んで渡します。
- 求めていないときに複数行を貼ると、改行が Enter として働いてしまいます。そのため、貼る前に確認を出します。

### マウス

- プログラムがマウスを使っているとき（エディタ、`less` など）は、クリックやホイールをプログラムに送ります。`Shift` を押しながら操作すると、MATSU Editor が受け取り、選択やスクロールをします。
- 使っていないときは、ホイールで履歴をさかのぼります。
- プログラムが画面を切り替えている間（代替画面）は、プログラムが求めていれば、ホイールを矢印キーとして送ります。

### リンク

画面にある `http://` と `https://` のアドレスは、`Ctrl` を押したままクリックすると開きます（macOS は `Cmd`）。開く前に、アドレスを見せて確認します。ほかの種類のアドレス（`file:` など）は開きません。プログラムが OSC 8 で付けたリンクも、同じです。

### タブの見え方

- タブの名前は、プログラムが付けた題です。付けていなければ、シェルのプログラムの名前（`pwsh`、`zsh` など）です。題は、制御文字と、文字の並びを変える文字を取り除き、128 文字までにします。
- シェルが終わると、題の後ろに「(終了)」が付きます。
- ベルが鳴ったとき、そのタブを見ていれば画面が一瞬光ります。見ていなければ、題の前に「●」が付き、OS に注意を求めます（Windows はタスクバーの点滅、macOS は Dock の跳ねる動き）。1 秒以内に続けて鳴ったベルは、1 つに数えます。
- プログラムがクリップボードに書くこと（OSC 52）は許します。クリップボードを読むことは許しません。

### 閉じるとき

動いているシェルのタブ、ウィンドウ、MATSU Editor を閉じると、「シェルが動いています。閉じると、シェルとその中で動いているプログラムが終わります。閉じますか?」と聞きます。終わったシェルのタブは、確認なしで閉じます。

## 設定

共通設定の「シェル」ページ。`matsu.ini` の `[Terminal]` に書かれます。

| 項目 | キー | 意味 |
|---|---|---|
| コマンドライン | `Shell` | 動かすプログラムと引数を、コマンドラインのとおりに書きます。空は既定のシェル。閉じていない引用符があるときは、既定のシェルで始まります。macOS と Linux では、`~` や `$HOME` は展開されません |
| 保持する行数 | `ScrollbackLines` | 画面より上に残す行数。0 から 50000。既定は 5000。タブ 1 枚あたり、1 万行でおよそ 29 MB のメモリを使います |
| Option を Meta として使う | `OptionAsMeta` | macOS だけ。既定はオフ |

コマンドラインと行数は、OK を押したあとに開くタブから使われます。開いているタブは変わりません。

## 記録しないもの

- シェルの出力を、ファイルに残しません。
- 履歴（スクロールバック）を保存しません。
- 「[自動保存](autosave.md)」の対象にもなりません。MATSU Editor を閉じると、シェルの中身は戻りません。
- マクロとプラグインは、シェルに文字を送れません。シェルに文字が届くのは、利用者のキー入力、入力した文字、貼り付けだけです。

## 文字の幅

シェルの中では、全角か半角か決まらない文字（曖昧幅の文字）を、1 桁として扱います。エディタの文書の設定（2 桁）とは合わせません。

## サクラとの違い

サクラエディタには、シェルのタブに当たる機能がありません。MATSU Editor だけの機能です。

## 根拠

- 機能の定義: `xtask/src/matsu_extras.rs` の `F_SHELL_TAB`、`crates/matsu-commands/src/sakura/macro_functions.rs` の `ShellTab`、`crates/matsu-commands/src/sakura/main_menu.rs`
- 開く動き: `crates/matsu-ui/src/ui_commands/window/shell_tab.rs`、`crates/matsu-ui/src/tabs/terminal_tab.rs` の `open_shell`、`crates/matsu-ui/src/terminal/mod.rs` の `start`、`start_directory`、`confirm_close`、`program_name`
- キー: `crates/matsu-ui/src/terminal/input.rs` の `translate_with`、`crates/matsu-ui/src/terminal/view.rs` の `is_window_function` と `NOT_FOR_A_TERMINAL`
- メニュー: `crates/matsu-ui/src/terminal/menus.rs`
- 貼り付け: `crates/matsu-terminal/src/paste.rs` の `encode_paste`、`crates/matsu-ui/src/terminal/view/clipboard.rs`
- マウスとリンク: `crates/matsu-ui/src/terminal/view/mouse.rs`、`crates/matsu-terminal/src/link.rs`
- 題とベルと OSC 52: `crates/matsu-terminal/src/title.rs`、`crates/matsu-ui/src/terminal/view/program.rs`
- 既定のシェルと環境変数: `crates/matsu-terminal/src/shell.rs`
- 設定: `crates/matsu-config/src/terminal.rs`、`crates/matsu-ui/src/common_settings/pages/terminal.rs`
- 決めたこと: `docs/adr/0028-terminal-tab.md`
