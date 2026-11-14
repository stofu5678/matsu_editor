# PowerShell

拡張子は `.ps1` です。利用者が入れた PowerShell で動かします。

## 処理系

| 項目 | 内容 |
|---|---|
| 起動する名前 | `pwsh`（PowerShell 7）。Windows で `pwsh.exe` が `PATH` になければ、`powershell`（Windows PowerShell 5.1） |
| 版 | 7 と 5.1 のどちらでも動きます |
| 起動の時間 | ほかの外部の言語より、1 桁遅い（Linux ARM64 の PowerShell 7 で、690〜980 ミリ秒と測りました） |
| プラグイン | 動かせません（動かせるのは QuickJS、Node.js、Python です） |

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

## 根拠

- 起動の引数、中身を渡すこと、処理系の選び方: `crates/matsu-plugin/src/session.rs` の `Language::PowerShell`、`POWERSHELL_BOOTSTRAP`、`powershell_interpreter`、`start`
- スコープ、`$MacroPath`、`$MacroDir`、`Get-EditorErrorCode`、エラーの形、出力の文字コード: `crates/matsu-plugin/src/boot/boot.ps1`
- 起動の時間: `docs/adr/0025-script-languages.md`（「PowerShell」）
- サンプル: `crates/matsu-plugin/samples/sample.ps1`
- 試験: `crates/matsu-plugin/src/tests.rs`
