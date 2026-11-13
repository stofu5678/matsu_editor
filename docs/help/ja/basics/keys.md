# キーで動く・選ぶ

マツエディタの既定のキー割り当ては、サクラエディタの既定の割り当てを元に作っています。このページには、動く・選ぶ・消す・入れるといった基本の操作に使うキーをまとめます。ほかの機能のキーは、各機能のページの「キー」の欄にあります。

キーの一覧は、「ヘルプ」メニューの「キー割り当て一覧をコピー」（`Ctrl+Q`）でクリップボードに写せます。「ヘルプ」メニューの「コマンド一覧」（`Shift+F1`）では、機能を名前で選んで実行できます。

## 動く

| キー | 動き | 機能 |
|---|---|---|
| `←` `→` `↑` `↓` | 1 文字、1 行動く | `F_LEFT` `F_RIGHT` `F_UP` `F_DOWN` |
| `Ctrl+←` `Ctrl+→` | 単語の左端、右端へ動く | `F_WORDLEFT` `F_WORDRIGHT` |
| `Home` `End` | 行頭、行末へ動く（折り返し単位） | `F_GOLINETOP` `F_GOLINEEND` |
| `Ctrl+Home` `Ctrl+End` | ファイルの先頭、最後へ動く | `F_GOFILETOP` `F_GOFILEEND` |
| `PgUp` `PgDn` | 1 ページ上、下へ動く | `F_1PageUp` `F_1PageDown` |
| `Ctrl+PgUp` `Ctrl+PgDn` | 半ページ上、下へ動く | `F_HalfPageUp` `F_HalfPageDown` |
| `Ctrl+↑` `Ctrl+↓` | キャレットは動かさず、テキストを 1 行ずつスクロールする | `F_WndScrollDown` `F_WndScrollUp` |
| `Ctrl+H` | キャレットのある行を、ウィンドウの中央へ寄せる | `F_CURLINECENTER` |
| `Ctrl+[` `Ctrl+]` | 対応する括弧へ動く | `F_BRACKETPAIR` |
| `Ctrl+J` | 行番号を指定して動く | `F_JUMP_DIALOG` |

そのほかの移動（段落、ブックマーク、タグジャンプ、ジャンプ履歴など）は、「コマンド: 編集」と「コマンド: 検索」の章にあります。

## 選ぶ

| キー | 動き | 機能 |
|---|---|---|
| `Shift` + 動くキー | 動いた範囲を選ぶ | `F_LEFT_SEL` など（`_SEL` の付く機能） |
| `Ctrl+Shift+←` `Ctrl+Shift+→` | 単語単位で選びながら動く | `F_WORDLEFT_SEL` `F_WORDRIGHT_SEL` |
| `Ctrl+A` | すべて選ぶ | `F_SELECTALL` |
| `Ctrl+W` | キャレットのある単語を選ぶ | `F_SELECTWORD` |
| `F6` | 範囲選択を始める（または終える） | `F_BEGIN_SEL` |
| `Esc` | 選択と、各種のモードをやめる | `F_CANCEL_MODE` |

`F6` で範囲選択を始めると、そのあとの動くキーが、`Shift` を押さなくても選択を広げます。もう一度 `F6` を押すと、選択を残したまま、通常の動きに戻ります。選択を解くような動き（選択なしの移動、文字の入力、`Esc`）をしても、選択の「ロック」は終わります。

## 矩形で選ぶ

| キー | 動き | 機能 |
|---|---|---|
| `Alt` + 動くキー | 矩形で選びながら動く | `F_LEFT_BOX` など（`_BOX` の付く機能） |
| `Shift+F6` | 矩形選択を始める（それまでの選択は捨てられ、キャレットの位置から新しい四角が始まる） | `F_BEGIN_BOX` |

詳しくは「[矩形選択と矩形の編集](box-select.md)」にあります。

## 消す・写す・貼る

| キー | 動き | 機能 |
|---|---|---|
| `BkSp` | キャレットの前の 1 文字を消す | `F_DELETE_BACK` |
| `Del` | キャレットの位置の 1 文字を消す | `F_DELETE` |
| `Ctrl+BkSp` `Ctrl+Del` | 単語の左端まで、右端まで消す | `F_WordDeleteToStart` `F_WordDeleteToEnd` |
| `Ctrl+X` | 切り取る | `F_CUT` |
| `Ctrl+C` | コピーする | `F_COPY` |
| `Ctrl+V` | 貼り付ける | `F_PASTE` |
| `Shift+Del` `Ctrl+Ins` `Shift+Ins` | それぞれ、切り取り、コピー、貼り付け | `F_CUT` `F_COPY` `F_PASTE` |
| `Shift+F9` | 矩形として貼り付ける | `F_PASTEBOX` |
| `Ctrl+Z` | 元に戻す | `F_UNDO` |
| `Ctrl+Y` | やり直す | `F_REDO` |

## 入れる

| キー | 動き | 機能 |
|---|---|---|
| `Ins` | 挿入モードと上書きモードを切り替える | `F_CHGMOD_INS` |
| `Tab` | 複数の行を選んでいるときは、各行の頭に Tab を入れる。それ以外は、Tab を入れる | `F_INDENT_TAB` |
| `Shift+Tab` | 選んだ行の頭の Tab（または空白）を取る。選択がないときは、Tab を入れる | `F_UNINDENT_TAB` |
| `Space` | 複数の行を選んでいるときは、各行の頭に空白を入れる。それ以外は、空白を入れる | `F_INDENT_SPACE` |
| `Shift+Space` | 選んだ行の頭の空白を 1 つ取る。選択がないときは、空白を入れる | `F_UNINDENT_SPACE` |
| `Ctrl+Space` `Ctrl+/` | 入力補完 | `F_HOKAN` |
| `Alt+;` `Alt+:` | 日付、時刻を入れる | `F_INS_DATE` `F_INS_TIME` |

文字を入れるときの動きは、「[文字入力と IME](typing.md)」にあります。入力補完は「[入力補完](completion.md)」にあります。

## ウィンドウと画面

| キー | 動き | 機能 |
|---|---|---|
| `Ctrl+Tab` `Ctrl+Shift+Tab` | 次、前のウィンドウ（タブ）へ移る | `F_NEXTWINDOW` `F_PREVWINDOW` |
| `F4` `Shift+F4` | 上下、左右に画面を分割する（もう一度押すと解除） | `F_SPLIT_V` `F_SPLIT_H` |
| `Ctrl+1` `Ctrl+2` `Ctrl+3` | ツールバー、ファンクションキーのバー、ステータスバーの出し入れ | `F_SHOWTOOLBAR` `F_SHOWFUNCKEY` `F_SHOWSTATUSBAR` |
| `Ctrl+ホイール` | フォントを拡大、縮小する | `F_SETFONTSIZEUP` `F_SETFONTSIZEDOWN` |

## 自分のキー割り当てにする

共通設定の「キー割り当て」で、機能ごとにキーを割り当てなおせます。割り当ては、`Shift`、`Ctrl`、`Alt` の組み合わせ 8 通りごとに持てます。割り当てた内容は、設定ファイル `matsu.ini` に保存されます。

また、マウスのクリックやホイールの動きにも、同じ表の中で機能を割り当てられます（「[マウスで動く・選ぶ](mouse.md)」を見てください）。

## macOS でのキー

macOS では、サクラエディタの `Ctrl` を、そのまま `Control` として読みます（`Control+S` で保存するなど、Windows と同じです）。`Alt` は `Option` キーです。`Cmd` には、サクラエディタが何も割り当てていないので、macOS のほとんどのアプリにある基本のショートカットだけを割り当てています。

| キー | 動き |
|---|---|
| `Cmd+C` `Cmd+V` `Cmd+X` | コピー、貼り付け、切り取り |
| `Cmd+Z` `Cmd+Shift+Z` | 元に戻す、やり直す |
| `Cmd+A` | すべて選択 |
| `Cmd+S` `Cmd+O` `Cmd+N` `Cmd+P` | 保存、開く、新規作成、印刷 |
| `Cmd+F` `Cmd+G` `Cmd+Shift+G` | 検索、次を検索、前を検索 |
| `Cmd+W` `Cmd+Q` `Cmd+,` | 閉じる、全終了、共通設定 |
| `Cmd+←` `Cmd+→` | 行頭、行末へ動く（`Shift` を足すと選択） |
| `Cmd+↑` `Cmd+↓` | ファイルの先頭、末尾へ動く（`Shift` を足すと選択） |
| `Cmd+BkSp` | 行頭まで削除 |
| `Option+BkSp` `Option+Del` | 単語単位で削除 |

`Cmd+H`、`Cmd+Option+H`、`Cmd+M` は、macOS とメニューに任せます。ほかの `Cmd` の組み合わせは、何もしません。

キーの読み替えは、設定ファイルの `[MATSU]` セクションの `nMacKeyLayout` で変えられます。

| 値 | 意味 |
|---|---|
| `0`（既定。項目がないときも） | 上の標準の読み替え |
| `1` | すべて `Control`。`Cmd` は何もしません。Windows のサクラエディタと同じ手の動きになります |
| `2` | サクラエディタの `Ctrl` を `Cmd` として読みます（`Ctrl+S` は `Cmd+S`）。`Cmd` が macOS の標準キーと重なるものは、macOS の意味にします |

macOS のキーボードでファンクションキー（`F1` から `F12`）を押すには、`fn` キーが要ることがあります。

## JIS 配列と US 配列

サクラエディタの割り当ては、Windows の仮想キーコードで決まっていて、キー名は日本語配列（JIS）のキーの位置に沿っています。マツエディタは、キーに印字されている文字で割り当てを探します。US 配列では、`^` のキーの割り当て（例: `Ctrl+^` の `F_COPYTAG`）は、`=` のキーで使います。記号のキーは、同じ文字が両方の配列にあるものは同じ動きになり、それ以外は近い位置のキーになります。実際のキーの位置で決められないのは、GPUI（画面を作る部品）がキーの位置の情報を渡さないためです。

## サクラエディタとの違い

- Windows と Linux では、サクラエディタの割り当てをそのまま使います。
- macOS の `Cmd` の割り当て、`Option+BkSp`、`Option+Del`、`nMacKeyLayout` は、マツエディタだけのものです。
- 記号のキーの位置は、上の「JIS 配列と US 配列」のとおり、近似です。

## 根拠

- 既定の割り当て: `crates/matsu-commands/src/sakura/key_bindings.rs`（サクラの `CKeyBind.cpp` の `KeyDataInit` から生成した表）。機能の名前は `crates/matsu-commands/src/sakura/functions.rs`
- キーの対応: `crates/matsu-ui/src/keys.rs`（`chord_of`、`single`）、`crates/matsu-commands/src/keymap/`
- macOS: `docs/adr/0008-commands-and-keybindings.md`（「macOS でのキー」）、`crates/matsu-ui/src/settings/mac_key_layout.rs`（`MacKeyLayout`）
- 範囲選択の開始: `crates/matsu-commands/src/commands/select/begin_sel.rs`
- サクラのソース: `sakura_core/func/CKeyBind.cpp`（`KeyDataInit`）、`sakura_core/cmd/CViewCommander_Select.cpp`（`Command_BEGIN_SELECT`）
