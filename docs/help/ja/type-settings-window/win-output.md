# アウトプット

マクロが出力した文字（`TraceOut`）を見せる、出力の欄を出します。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > アウトプット |
| 機能名 | `F_WIN_OUTPUT` |
| 既定のキー | なし |
| マクロ | `ActivateWinOutput` |
| キーマクロに記録 | する |

## 動き

サクラは、出力専用のウィンドウを開きます。MATSU Editor に出力専用のウィンドウはありません。マクロの出力は、マクロを動かしたウィンドウの本文の下にある出力の欄に出るので、このコマンドはその欄を表示します（欄に残っている文字はそのままです）。

## サクラエディタとの違い

- 出力専用のウィンドウがない代わりに、マクロを動かしたウィンドウごとの出力の欄を使います。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Window.cpp` の `Command_WIN_OUTPUT`、`CShareData::OpenDebugWindow`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/win_output.rs`、`crates/matsu-ui/src/macros/`（`output`）
