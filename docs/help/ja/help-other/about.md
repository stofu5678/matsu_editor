# バージョン情報

マツエディタの版や作り、ライセンスを見る箱を開きます。

- 機能名: `F_ABOUT`
- メニュー: 「ヘルプ」→「バージョン情報」
- 既定のキー: `Ctrl+Shift+F1`
- マクロ: `About()`（引数なし。キーマクロには記録されません）

## 箱に出るもの

- アイコンと、「MATSU Editor Ver. 版の番号」
- リポジトリの URL（押すと、既定のブラウザで開く）
- 「Build:」: ビルドの種類（`release` か `debug`）、OS、CPU（例: `release, windows aarch64`）
- 「Last Modified:」: 実行ファイルを最後に書き換えた日時（分かるときだけ）
- 説明の文: ライセンス、サクラエディタを元に Rust で作り直したこと、一部は独自の仕様であること、コードの一部を zlib License の下で移植していること、動く OS

## ボタンと操作

| 操作 | 動き |
|---|---|
| 「コピー」 | 版の番号、リポジトリ、ビルドの行を、クリップボードに入れる |
| 「OK」、`Enter`、`Space`、`Esc` | 箱を閉じる |

- 箱が開いている間は、キーはすべて箱が受け取ります。文書には届きません。
- 箱がすでに開いているウィンドウで、もう一度この機能を動かしても、箱は 1 つのままです。

「コピー」で入る文字は、次の形です。版の番号、ビルド、日時は例で、実際はそのときの値が入ります。「Last Modified:」の行は、日時が分かるときだけ付きます。

```
MATSU Editor Ver. 0.1.0
(GitURL https://github.com/stofu5678/matsu_editor)

      Build: release, windows aarch64
      Last Modified: 2026/10/16 12:00:00
```

## サクラエディタとの違い

サクラエディタの箱にある、サクラエディタ自身の謝辞とビルドの番号は、マツエディタの箱にありません。説明の文は、マツエディタ用に書いたものです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp` の `CViewCommander::Command_ABOUT`、`sakura_core/dlg/CDlgAbout.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/support/about.rs`、`crates/matsu-ui/src/support/about.rs`（`AboutInfo`、`AboutDialog`）
- 試験: `crates/matsu-ui/src/support/about.rs` の `tests` モジュール
- 既定のキーとマクロ名: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F1` の行）、`crates/matsu-commands/src/sakura/macro_functions.rs`（`About`）
