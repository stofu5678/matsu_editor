# マクロと外部コマンドの関数

マクロや外部コマンドを動かす関数と、マクロだけが使う補助の関数（出力、ステータスバー、取り消し、スクロール）です。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `ExecExternalMacro` | 文字列（マクロのファイル（省略するとダイアログで選ぶ））、文字列（言語（拡張子の書き方。あれば、1 つ目はファイルでなくコード）） | なし | マクロのファイルを実行する。ファイルは、利用者がダイアログで選ぶこともできる。 | する | 言語に空の文字列を渡すと、言語の指定なしとして扱う。 |
| `ExecCommand` | 文字列（コマンドライン）、整数（オプション（下の「ExecCommand のオプション」））、文字列（フォルダー（オプションの 0x200 のとき使う）） | なし | 外部のコマンドを実行する。 | する | コマンドラインの `$F` と `$$` を展開する。マクロから呼んだときは、ダイアログが覚えている設定は変わらない。マクロは、コマンドが終わるまで待つ。 |
| `ExecCommandDialog` | なし | なし | 外部コマンド実行のダイアログを開き、頼まれた内容を実行する。 | しない | 記録なし |
| `TraceOut` | 文字列（出力する文字列）、整数（オプション（0x01 `ExpandParameter` を通す、0x02 行末の改行を付けない）） | なし | マクロの出力のパネルに文字列を書く。パネルは自動で開く。改行は、0x02 がなければ付く。 | しない | サクラは専用のウィンドウに書く。MATSU Editor は、そのウィンドウの下のパネルに書き足す。0x01 のとき、展開に使えるのは 2047 文字まで。 |
| `StatusMsg` | 文字列（表示する文字列）、整数（オプション（下位 2 ビットが、0〜3 の表示のしかた）） | なし | ステータスバーにメッセージを出す。 | しない | 次の編集かキャレットの移動で消える。ステータスバーが非表示だと、オプション 0 は出す場所がなく、オプション 1 のメッセージボックスだけが見える。 |
| `MsgBeep` | 整数（種類（-1、0〜4）） | なし | OS の警告音を鳴らす。 | しない | 種類を区別せず、OS の警告音 1 つだけが鳴る。 |
| `CommitUndoBuffer` | なし | なし | 作りかけの取り消しの 1 ステップを終え、新しいステップを始める。ブロックの深さは変えない。 | しない | 記録なし |
| `AddRefUndoBuffer` | なし | なし | これ以降のコマンドを、`SetUndoBuffer` で閉じるまで、取り消しの 1 ステップにまとめる。 | しない | マクロが閉じ忘れても、マクロの終わりでウィンドウが閉じる。 |
| `SetUndoBuffer` | なし | なし | 取り消しのブロックから 1 つ抜ける。`AddRefUndoBuffer` と同じ回数だけ呼ぶと、1 ステップが決まる。 | しない | ブロックがないときは、何もしない。 |
| `AppendUndoBufferCursor` | なし | なし | キャレットの位置を、作っている取り消しのステップに足す。 | しない | このステップは、文書を変更済みにしない。 |
| `ClipboardEmpty` | なし | なし | クリップボードを空にする。そのあとの貼り付けは、テキストがなく、ビープ音が鳴る。 | しない | 記録なし |
| `SetViewTop` | 整数（行（1 から数える）） | なし | その行が、ビューの先頭に来るように、スクロールする。キャレットは動かさない。 | しない | 引数が空、または数字で始まらないときは、エラーになる。ほかのペインは、自動でついてくる。 |
| `SetViewLeft` | 整数（桁（1 から数える）） | なし | その桁が、ビューの左端に来るように、スクロールする。キャレットは動かさない。 | しない | 引数が空、または数字で始まらないときは、エラーになる。範囲は、直前の描画の、いちばん幅の広い行の右端まで。 |


## ExecCommand のオプション {#execcommand-options}

2 つ目の引数は、次の値を足して作ります。省略すると 0 で、コマンドの出力は取り込みません。

| 値 | 意味 |
|---|---|
| 0x01 | 標準出力（と標準エラー）を取り込む |
| 0x02 | 取り込んだ出力を、編集中のウィンドウに入れる。なければ出力のパネルに出す |
| 0x04 | 選択（なければ文書全体）を、標準入力へ送る |
| 0x08 | 出力が Unicode。0x80 は UTF-8。どちらもなければ Shift_JIS |
| 0x10 | 入力が Unicode。0x100 は UTF-8。どちらもなければ Shift_JIS |
| 0x20 | コマンドライン、日付、終了コードを書く |
| 0x40 | それらを書かない |
| 0x200 | 3 つ目の引数のフォルダーで実行する |

## 出力のパネルとビュー

- `TraceOut` が書く出力は、マクロを動かしたウィンドウの、テキストの下のパネルに出ます。`ActivateWinOutput` は、そのパネルを出します。
- 取り消しの関数は、組で使います。`AddRefUndoBuffer` で始め、`SetUndoBuffer` で終えると、その間の編集が取り消しの 1 ステップになります。

## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`ExecExternalMacro`、`ExecCommand`、`ExecCommandDialog`、`TraceOut`、`StatusMsg`、`MsgBeep`、`CommitUndoBuffer` から `SetViewLeft`）
- サクラ: `sakura_core/macro/CMacro.cpp`、`sakura_core/cmd/CViewCommander_Macro.cpp`、`CViewCommander_Window.cpp`、`sakura_core/view/CEditView_ExecCmd.cpp`
- MATSU のコード（`crates/matsu-commands/src/commands/macro_only/`）: `addrefundobuffer.rs`、`appendundobuffercursor.rs`、`clipboardempty.rs`、`commitundobuffer.rs`、`setundobuffer.rs`
- MATSU のコード（`crates/matsu-ui/src/ui_commands/`）: `macros/exec_ext_macro.rs`、`macros/execmd.rs`、`macros/execmd_dialog.rs`、`macro_only/traceout.rs`、`statusmsg.rs`、`msgbeep.rs`、`setviewtop.rs`、`setviewleft.rs`
