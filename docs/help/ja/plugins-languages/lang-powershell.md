# PowerShell

拡張子は `.ps1` です。利用者が入れた PowerShell で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `pwsh`（PowerShell 7）。Windows で `pwsh.exe` が `PATH` になければ、`powershell`（Windows PowerShell 5.1） |
| 版 | 7 と 5.1 のどちらでも動きます |
| 起動の時間 | ほかの外部の言語より、1 桁遅い（Linux ARM64 の PowerShell 7 で、690〜980 ミリ秒と測りました） |
| プラグイン | 動かせます。下の「プラグインとして動かすとき」 |

## 動き方

- `-NoLogo -NoProfile -NonInteractive -Command` で起動します。利用者のプロファイルは、読み込みません。出力が変わったり、入力待ちになったりするのを避けるためです。
- スクリプトのファイルは、PowerShell に渡しません。MATSU Editor が読んで、中身を渡します。実行ポリシー（Windows のクライアントの既定は Restricted）と、Windows PowerShell 5.1 が BOM のないファイルを ANSI として読むことの、どちらにも影響されないためです。
- 渡された中身は、スクリプトブロックにして、グローバルのスコープで動かします。変数と関数は、コンソールで打ったときのように、グローバルに残ります。
- `$Editor` と、エラーの番号を読む `Get-EditorErrorCode` は、最初から使えます。呼び方は、`$Editor.InsText("文字")` です。
- ファイルがないので、`$PSScriptRoot` と `$PSCommandPath` は、空です。スクリプトのパスは `$MacroPath`、あるフォルダーは `$MacroDir` で読みます。
- `Write-Host` と `Write-Output` などの出力は、UTF-8 です。色のコードは付きません（PowerShell 7.2 以降）。表の形の出力も、スクリプトが終わる前に出ます。

## 標準ライブラリ

利用者の PowerShell のコマンドレットとモジュールが、そのまま使えます。

## 文字コード

- スクリプトは UTF-8 で保存します（BOM は取り除きます）。
- 出力は UTF-8 です。コンソールの文字コードにかかわらず、そうなります。

## エラー

`Editor` の呼び出しが失敗すると、例外になります。PowerShell は、メソッドが投げた例外を包むので、番号は `Get-EditorErrorCode $_` で読みます。エラーでなければ `$null` です。

```powershell
try {
  $Editor.GetLineStr('not a number')   # a bad argument
} catch {
  Write-Output ("code: " + (Get-EditorErrorCode $_))
}
```

受けなかったエラーは、`スクリプト名:行: メッセージ` の形で、標準エラーに出ます。構文のエラーも同じ形です。スクリプトは、終了コード 1 で終わります。

## プラグインとして動かすとき

ハンドラの拡張子が `.ps1` のとき、利用者の PowerShell が動かします。使う PowerShell は、マクロと同じです（`pwsh`。Windows で `pwsh.exe` が `PATH` になければ、`powershell`）。

- ハンドラの文面は、MATSU Editor が呼び出しのたびに渡します（同意のときに確かめた文面です）。ファイルは PowerShell に渡さないので、実行ポリシーに止められません。文面は、スクリプトブロックにして、その呼び出しだけの新しいスコープで動かします。
- `$Editor`、`$Plugin`、`$Outline`、`$Complement`、`$Indent` は、最初からあります。メソッドで呼びます（`$Plugin.GetOption('節', 'キー')`。ジャックの名前は `$Plugin.Jack`、引数は `$Plugin.Args`、プラグインの ID は `$Plugin.Id`、フォルダーは `$Plugin.Dir`）。`$Indent.GetChar()` は、`SmartIndent` のときだけ、打った文字を返します（ほかの場面では空の文字列です）（[ハンドラの中で使えるもの](plugin-api.md)）。
- ハンドラの変数と関数は、その呼び出しの中だけのものです。MATSU Editor の側の変数は書き換えられません（同じ名前の変数を作っても、ハンドラの中の別のものになります）。プロセスが残っている（`UseCache=1` か `Resident=1`）間、次の呼び出しに引き継ぎたい値は、`$global:count = [int]$global:count + 1` のように、`$global:` をつけて読み書きします。
- `return` は、そのハンドラを終えるだけです。`exit` も、そのハンドラを終えるだけで、失敗ではありません（MATSU Editor が、文面の中の `exit` を、ハンドラだけを終える形に書き換えます。行番号は変わりません。ハンドラ自身の `try`/`catch` で包んだ `exit` は、その `catch` に入ります）。
- 別のファイルを読むときは、`. (Join-Path $Plugin.Dir 'lib/word.ps1')` のように、`.` で読みます。`Import-Module` も使えます。読むファイルは、エディターがフォルダーを確かめた直後に、ディスクから読みます（Python、Ruby、PHP が import するファイルを読むのと同じです）。`.` で読んだ関数は、その呼び出しの中だけのものです。
- エラーが出ると、その呼び出しは失敗になります。メッセージは、種類（`RuntimeException` など。構文の誤りは `ParseError`、`Editor` の失敗は `EditorError 番号:`）、理由、`ハンドラ名:行` です。行は、ハンドラのファイルのものです。プロセスは残り、次の呼び出しを受けます。
- `$Plugin.GetOption('節', 'キー')` などの、オブジェクトの関数は、同じ綴りの関数です。引数の最後の `$null` は、省略として扱います。
- 出力は、呼び出しごとに、答えの前に書き出します。

## 根拠

- 起動の引数、中身を渡すこと、処理系の選び方: `crates/matsu-plugin/src/session.rs` の `Language::PowerShell`、`POWERSHELL_BOOTSTRAP`、`powershell_interpreter`、`start`
- プラグインの待ち受け、スコープ、`exit`、失敗の文面: `crates/matsu-plugin/src/boot/boot.ps1` の `Start-MatsuPlugin`、`Invoke-MatsuHandler`、`Convert-MatsuExit`。試験は `crates/matsu-plugin/src/external_plugin_tests/powershell_perl.rs`、サンプルは `crates/matsu-plugin/samples/plugins/HelloPowerShell`
- スコープ、`$MacroPath`、`$MacroDir`、`Get-EditorErrorCode`、エラーの形、出力の文字コード: `crates/matsu-plugin/src/boot/boot.ps1`
- 起動の時間: `docs/adr/0025-script-languages.md`（「PowerShell」）
- サンプル: `crates/matsu-plugin/samples/sample.ps1`
- 試験: `crates/matsu-plugin/src/tests.rs`
