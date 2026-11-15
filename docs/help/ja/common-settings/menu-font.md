# フォント設定...

本文のフォントを選びます。選んだフォントは、すべてのウィンドウの共通のフォントになります。

- 機能名: `F_FONT`
- メニュー: 「設定」→「フォント設定...」
- 既定のキー: `Ctrl+7`
- マクロ: `SelectFont()`（引数なし。キーマクロには記録されません）

## 動き

1. フォントの選択ダイアログが出ます。等幅でないフォントも選べます
2. 選ぶと、フォントの名前と大きさが `matsu.ini` の `[Font]` セクションにすぐ書かれ、すべてのウィンドウが新しいフォントで描き直します
3. 「キャンセル」なら、何も変わりません

- ダイアログは、Windows では OS 標準のフォントの選択（`ChooseFont`）を使います。macOS と Linux では、MATSU Editor の窓（フォントの系統、スタイル、大きさ、見本）を使います
- タイプ別設定で、そのタイプ専用のフォントを選んでいるウィンドウは、そのフォントのままです

## サクラエディタとの違い

- サクラエディタは、ウィンドウが全部閉じるときにファイルへ書きます。MATSU Editor は、選んだときすぐに書きます（ほかのウィンドウが書いた内容を消さないよう、ファイルをロックして書きます）
- サクラエディタと同じく、どのフォントを選んでも「等幅」の扱いにします

## 関連

- 大きさだけを変えるには、`F_SETFONTSIZE` などの機能（Ctrl とホイール）があります
- タブバーのフォントは、[タブバー](page-tab.md) ページで選びます

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Settings.cpp` の `Command_FONT`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/settings/font.rs`、`crates/matsu-ui/src/font_dialog/mod.rs`、`crates/matsu-ui/src/font_dialog/apply.rs`
- マクロ名: `crates/matsu-commands/src/sakura/macro_functions.rs`（`SelectFont`）
