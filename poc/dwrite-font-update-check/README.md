# DirectWrite のフォント一覧の確認の時間を測る

[#23](https://github.com/stofu5678/ume_editor/issues/23) の調査で作った、捨てる前提の再現プログラム。GPUI は使わない。

GPUI の Windows の部分（`gpui_windows` の `DirectWriteTextSystem::new`）は、起動時に `GetSystemFontCollection` を `checkForUpdates = true` で呼ぶ。このプログラムは同じ呼び出しを、同じ版の `windows` クレートで、true と false のそれぞれで測る。

Windows の PowerShell で、このディレクトリで次を実行する。環境（OS の版、CPU、フォントの数など）を出してからビルドし、新しいプロセスで true と false を交互に 10 回ずつ測る。

```powershell
powershell -ExecutionPolicy Bypass -File run.ps1
```
