# 再変換

カーソルのある文字を、IME にもう一度変換させます。

## 呼び出し方

- メニュー: 「編集」→「再変換」
- キー: 既定の割り当てはありません
- マクロ: ありません（サクラエディタにも、この機能のマクロ名はありません）
- 機能名: `F_RECONVERT`

## 動き

| OS | 動き |
|---|---|
| Windows | IMM32 を使います。IME が範囲を広げ、その範囲が選ばれ、IME が変換を始めます。変換の結果が選択範囲に入ります |
| macOS、Linux | プログラムから再変換を始める方法がないため、できません。ステータスバーに、できない理由を出します |

macOS と Linux では、入力メソッドのほうが、必要なときに周りの文字を取りに来ます。そのため、アプリ側から変換を始められません。サクラエディタは IME が対応していないとき何もしませんが、MATSU Editor は、メニューの項目が黙って何もしないよりよいと考え、理由を出します。

Windows の実際の IME での確認は、まだ残っています（CI には IME がありません）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit_advanced.cpp` の `Command_Reconvert`、`sakura_core/view/CEditView_Ime.cpp`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/ime/reconvert.rs`（先頭のコメント）、`crates/matsu-ui/src/reconvert/mod.rs`
- 差と残り: `docs/sakura-gaps.md` の「IME の再変換」の行
