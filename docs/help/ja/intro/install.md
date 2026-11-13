# インストールと起動

MATSU Editor の配布物は、GitHub のリリース（<https://github.com/stofu5678/matsu_editor/releases>）にあります。署名はまだしていません。そのため、初めて開くときに OS が警告を出すことがあります。

ファイル名の `<版>` には、版の番号（`v0.9.2` など）が入ります。最新の途中経過を入れた `nightly` という名前のものもあります。

## Windows

次のどちらかを使います。`<対象>` は、x64 の PC なら `windows-x64`、ARM64 の PC なら `windows-arm64` です。

| ファイル | 内容 |
|---|---|
| `matsu-<版>-<対象>-setup.exe` | インストーラ |
| `matsu-<版>-<対象>.zip` | 展開するだけで使える一式 |

### インストーラを使う

1. `matsu-<版>-<対象>-setup.exe` を実行します
2. 画面の指示に従います。画面の言語は、Windows の表示言語に合わせます。日本語の Windows では日本語、そのほかでは英語です

インストーラの動きは次のとおりです。

- 既定では、自分だけが使う形で入れます（`%LOCALAPPDATA%\Programs\MATSU Editor`）。管理者の権限は要りません。最初のダイアログで「全利用者」を選ぶと、`Program Files` に入れます
- スタートメニューと、Windows の「アプリ」（アンインストール）に項目を作ります
- 古い版の上に入れると、更新になります。x64 と ARM64 のどちらも、同じ製品として扱います
- 選べる項目があります

| 項目 | 名前 | 既定 | 内容 |
|---|---|---|---|
| プログラムから開く | `openwith` | オン | 「プログラムから開く」の一覧に `matsu.exe` を出します |
| `.txt` の関連付け | `associatetxt` | オフ | `.txt` を開くプログラムとして登録し、「既定のアプリ」に出します。既定のアプリを決めるのは利用者です。インストーラは奪いません |
| PATH に追加 | `addtopath` | オフ | インストール先のフォルダーを PATH に足します。アンインストールで取り除きます |

- 利用者のデータには触れません。インストール、更新、アンインストールで、設定のフォルダー（`%APPDATA%\matsu-editor`）の `matsu.ini` などを作らず、書き換えず、消しません。「設定も消す」という選択肢もありません
- アンインストールは、インストーラが入れたファイルだけを消します

コマンドラインで動かすこともできます。

```text
matsu-<版>-<対象>-setup.exe /VERYSILENT /SUPPRESSMSGBOXES
matsu-<版>-<対象>-setup.exe /CURRENTUSER
matsu-<版>-<対象>-setup.exe /ALLUSERS
matsu-<版>-<対象>-setup.exe /TASKS="openwith,associatetxt,addtopath"
matsu-<版>-<対象>-setup.exe /LANG=ja
```

| 指定 | 意味 |
|---|---|
| `/VERYSILENT /SUPPRESSMSGBOXES` | 画面を出さずに入れます |
| `/CURRENTUSER`、`/ALLUSERS` | 入れる形を決めます |
| `/TASKS=...` | 選べる項目を名前で決めます |
| `/LANG=ja`、`/LANG=en` | 画面の言語を決めます |

### zip を使う

1. zip を好きな場所に展開します
2. 中の `matsu.exe` を実行します

zip には、実行ファイル、マクロを動かす `matsu-script.exe`、ライセンスの文書、アイコンの絵、短い `README.txt` が入っています。`matsu-script.exe` は `matsu.exe` と同じフォルダーに置いたままにします。`matsu.exe` は、自分の隣にあるものを使います。

署名がないので、Windows の SmartScreen が警告を出すことがあります。

## macOS

macOS は Apple シリコン（ARM64）だけです。次のどちらかを使います。

| ファイル | 内容 |
|---|---|
| `matsu-<版>-macos-arm64.dmg` | ディスクイメージ |
| `matsu-<版>-macos-arm64.tar.gz` | アーカイブ |

### dmg を使う

1. `.dmg` を開きます。「MATSU Editor」というボリュームに、`MATSU Editor.app` と「Applications」へのリンクが入っています
2. `MATSU Editor.app` を「Applications」にドラッグします
3. 「Applications」の `MATSU Editor.app` を開きます

`.dmg` は何も設定せず、設定のフォルダーにも触れません。

### 初めて開くとき

アプリは署名が簡易的で、公証（Apple の確認）を受けていません。初めて開くとき、macOS が「開発元を確認できない」と出すことがあります。そのときは、次のどちらかをします。

- アプリを Control キーを押しながらクリックし、「開く」を選ぶ
- 「システム設定」の「プライバシーとセキュリティ」で「このまま開く」を選ぶ

「壊れている」と出るときは、ブラウザーが付けた隔離の印が原因です。ターミナルで、次のコマンドで外します。

```sh
xattr -dr com.apple.quarantine "MATSU Editor.app"
```

### ターミナルから起動する

```sh
"/Applications/MATSU Editor.app/Contents/MacOS/matsu" file.txt
```

## Linux

x64 と ARM64 があります。`<対象>` は、`linux-x64` か `linux-arm64` です。

| ファイル | 内容 |
|---|---|
| `matsu-<版>-<対象>.deb` | Debian、Ubuntu 系のパッケージ |
| `matsu-<版>-<対象>.rpm` | Fedora 系のパッケージ |
| `matsu-<版>-<対象>.tar.gz` | 展開するだけで使える一式 |

### パッケージを使う

```sh
# Debian、Ubuntu
sudo apt install ./matsu-<版>-<対象>.deb

# Fedora
sudo dnf install ./matsu-<版>-<対象>.rpm
```

- プログラムは `/usr/lib/matsu-editor/` に入り、`/usr/bin/matsu` がそこへのリンクになります
- アプリケーションの一覧用の項目（`.desktop`）が入ります。テキストファイルの「このアプリで開く」に出ますが、既定のアプリは奪いません
- インストール・削除のときに、設定のフォルダー（`~/.config/matsu-editor`）には触れません
- `.deb` は、作った環境（Ubuntu 24.04）の glibc などを要求します。それより古い配布版には入りません
- 描画には Vulkan を使います

### tar.gz を使う

1. 好きな場所に展開します
2. 中の `matsu` を実行します

`matsu-script` は `matsu` と同じフォルダーに置いたままにします。

## 起動のしかた

アイコンから起動するほか、ターミナルからも起動できます。

```sh
matsu [--separate-process] [サクラエディタのオプション] [--] [path...]
```

- ファイルを付けないと、空のウィンドウが 1 つ開きます
- ファイルを付けると、そのファイルを開きます
- `--`: これ以降は、`-` で始まる名前でもファイルとして扱います
- `--separate-process`: 動いている MATSU Editor を探さず、独立したプロセスとして動かします（試験や開発のためのものです）

MATSU Editor は、使う設定ファイル（`matsu.ini`）1 つにつき、1 つのプロセスで動きます。`matsu` をもう一度起動すると、開くファイルと作業フォルダーを、動いている MATSU Editor に渡して、すぐに終わります。もう開いているファイルは、そのウィンドウが前に出ます。

サクラエディタのコマンドラインオプションも使えます。名前の大文字小文字は区別しません。値は `=` か `:` に続けて書きます。

| オプション | 意味 |
|---|---|
| `-X=` `-Y=` `-VX=` `-VY=` | 開いた直後のキャレットと表示の位置 |
| `-CODE=` | 文字コード（サクラエディタの番号） |
| `-TYPE=` | 拡張子でタイプ別設定を決める |
| `-R` | ビューモード |
| `-SX=` `-SY=` `-WX=` `-WY=` | 新しいウィンドウの大きさと位置 |
| `-GREPMODE` `-GREPDLG` `-GKEY=` `-GREPR=` `-GFILE=` `-GFOLDER=` `-GOPT=` `-GCODE=` | Grep |
| `-M=` | 読み込み後に実行するマクロのファイル |
| `-PROF=` | 設定のプロファイル（設定ファイルのサブフォルダー）。[設定ファイル matsu.ini](settings-file.md) を見てください |
| `-@=` | レスポンスファイル |

`-NOWIN` `-GROUP=` `-DEBUGMODE` `-PROFMGR` は、受け付けて何もしません。

### macOS の PATH

Finder や Dock から起動したアプリは、シェルを通らないので、`PATH` が最小になります。MATSU Editor は、起動したときにログインシェルを裏で 1 回動かして `PATH` を読み取り、外部コマンドやマクロの言語を探すときに使います。10 秒で読めなかったときは、起動したときの `PATH` のまま動きます。ターミナルから起動したときは、ターミナルの `PATH` をそのまま使います。

## 根拠

- 配布物の名前と中身: `docs/release.md`（「アセットの名前」と「Windows のインストーラ」）、`.github/workflows/package.yml`（`README.txt` を作る部分）
- Windows のインストーラの定義: `packaging/windows/matsu.iss`
- Linux のパッケージ: `scripts/package-linux.sh`、`packaging/linux/`
- macOS のアプリと dmg: `scripts/package-mac-app.sh`、`scripts/package-mac-dmg.sh`
- 起動のしかた、コマンドラインオプション、macOS の PATH: `README.md` の「起動のしかた」、`crates/matsu/src/main.rs`、`crates/matsu-ui/src/command_line.rs`
- 1 つのプロセスで動くこと: `docs/adr/0016-single-instance.md`
- インストーラと配布物の決めごと: `docs/adr/0032-install.md`
