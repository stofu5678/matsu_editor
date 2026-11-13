# イベント

6 つのジャックが、ウィンドウやファイルの節目で動きます。利用者は何も押しません。キー入力のたびには動きません。

| ジャック | 場面 |
|---|---|
| `EditorStart` | ウィンドウができたあと |
| `EditorEnd` | ウィンドウが閉じる前 |
| `DocumentOpen` | ファイルを開いたあと |
| `DocumentClose` | ファイルを閉じる前（変更を保存するかの質問のあと） |
| `DocumentBeforeSave` | 保存の前 |
| `DocumentAfterSave` | 保存のあと |

`plugin.def` の `[Plug]` に、ジャックの名前とハンドラを書きます。

```
[Plug]
DocumentBeforeSave=before_save.js
DocumentAfterSave=after_save.js
```

## 動く順番

- 同じイベントを、複数のプラグインが宣言しているときは、プラグインの一覧の順に、1 つずつ動かします。1 つのプラグインの中では、`[Plug]` に書いた順です。
- 1 つのウィンドウでは、前のイベントが終わってから、次のイベントを動かします。たとえば `EditorStart` が終わってから、最初の `DocumentOpen` が動きます。
- プラグインのコマンドやマクロが、そのウィンドウで動いている間に起きたイベントは、配りません。

## 保存と閉じるの順番

保存では、次の順に動きます。

1. 保存前のマクロ（共通設定の「マクロ」で登録したもの）
2. `DocumentBeforeSave`
3. 名前を付けて保存のときは、元の文書について `DocumentClose`
4. ファイルを書く
5. `DocumentAfterSave`
6. 名前を付けて保存のときは、オープン後のマクロと、新しい文書について `DocumentOpen`

3 から 6 までは、名前を付けて保存のときだけです。

保存前のマクロや `DocumentBeforeSave` が文書を変えたときは、書く前の質問（改行コードの混在と、文字コードで書けない文字）を、もう一度出します。

閉じるときは、`DocumentClose` が動いてから閉じます。ウィンドウごと閉じるときは、そのあとに `EditorEnd` が動きます。プラグインを待っている間に、利用者が文字を打つと、閉じるのをやめます。状態バーに「プラグインを待つ間に文書が変わったので、閉じるのをやめた」と出ます。もう一度閉じると、質問から始まります。

ウィンドウが閉じなかったときは、そのウィンドウに `EditorStart` をもう一度送ります。プラグインが `EditorStart` で初期化しているなら、閉じるのをやめるたびに、その初期化がもう一度動きます。

## 待ちと失敗

- イベントの呼び出しは、5 秒までです（[時間の制限と失敗](plugin-limits.md)）。待っている間も、ウィンドウは使えます。待ちが 0.3 秒を超えると、状態バーに「プラグインを待っています」と出ます。
- ハンドラが例外を投げた、遅すぎた、プロセスが落ちたとき、出力パネルに知らせます。保存も、閉じる操作も、そのまま続けます。プラグインの失敗で、保存や閉じる操作を止めることはありません。
- うまく終わったイベントは、何も言いません。ハンドラが出力したときだけ、出力パネルを開きます。

## `EditorStart` を呼ぶ場面

MATSU Editor は、1 つのプロセスで複数のウィンドウを持ちます。ウィンドウができるたびに、`EditorStart` を呼びます。`EditorEnd` も、ウィンドウごとです。

ファイルを指定して起動したときは、そのファイルを読んでから `EditorStart` を呼びます。Grep を指定して起動したとき（`-GREPMODE`）は、検索が終わってから、`EditorStart` と `DocumentOpen` を呼びます。

## マクロとの関係

マクロ（キーマクロとスクリプト）が、ファイルを保存したり読み込んだりしても、イベントは呼びません。

## サクラエディタとの違い

- サクラエディタは、時間の制限なしに待ちます。MATSU Editor は、5 秒で打ち切って、操作を続けます。
- サクラエディタは、1 つのウィンドウに文書が 1 つです。MATSU Editor はタブがあるので、プラグインを待つ間にタブを開いたり、文字を打ったりできます。その場合の扱いは、上のとおりです。
- マクロが保存や読み込みをしたときは、イベントを呼びません。サクラエディタは、呼びます。
- OS が直接アプリを終わらせる道（Dock からの終了、ログアウト）では、イベントを呼びません。
- `-GREPDLG` で起動したときは、ダイアログのあとにイベントを呼びません。

## 根拠

- 場面、順番、待ち: `crates/matsu-ui/src/plugin_events.rs` の冒頭の説明、`fire`、`fire_and_wait`、`window_closing`、`restart_ended`、`start_after_grep`、`WAITING`、`WAIT_NOTICE_AFTER`
- 保存の順番: `crates/matsu-ui/src/file_ops/save.rs`（`DocumentBeforeSave` から `DocumentOpen` まで）
- 閉じる: `crates/matsu-ui/src/file_ops/close.rs` の `close_event_keeps_the_text`、`close_with_events`
- 制限の時間: `crates/matsu-plugin/src/plugin_host.rs` の `LIMITS`
- 試験: `crates/matsu-ui/src/plugin_events/tests.rs`、`close_path_tests.rs`
- サンプル: `crates/matsu-plugin/samples/plugins/HelloEvents/`
- サクラのソース: `sakura_core/plugin/CJackManager.cpp`、`sakura_core/doc/CDocFileOperation.cpp`（保存と開く）、`sakura_core/window/CEditWnd.cpp`（閉じる）、`sakura_core/CNormalProcess.cpp`（起動）
- `docs/sakura-gaps.md` の「プラグイン」の表（残っていること）
