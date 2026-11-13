# 文字カウント方法

ステータスバーに出す「選択の文字数」を、文字の数で数えるか、バイトの数で数えるかを切り替えます。いまのウィンドウだけの切り替えです。

- 機能名: `F_SELECT_COUNT_MODE`
- メニュー: 「設定」→「文字カウント方法」
- 既定のキー: なし
- マクロ: `SelectCountMode(mode)`（キーマクロに記録されます）

## 引数

| `mode` | 動き |
|---|---|
| 0 | 文字単位とバイト単位を替えます。引数を省いたときも 0 です |
| 1 | 文字単位にします |
| 2 | バイト単位にします |

ほかの数は、何も変えません。メニューには、いまの方法にチェックが付きます。

## 共通設定との関係

この機能は、共通設定を書き換えません。ファイルに残す既定の方法は、「共通設定」の [ステータスバー](page-status-bar.md) ページの「選択文字数を文字単位ではなくバイト単位で表示する」で決めます。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Settings.cpp` の `Command_SELECT_COUNT_MODE`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/settings/select_count_mode.rs`、`crates/matsu-ui/src/menu_state.rs`
- マクロ名: `crates/matsu-commands/src/sakura/macro_functions.rs`
