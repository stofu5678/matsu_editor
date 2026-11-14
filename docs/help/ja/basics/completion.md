# 入力補完

入力補完は、書きかけの語の頭の文字から、候補の一覧を出して、選んで仕上げる機能です。同じ語を何度も打つときに使います。

## 使い方

1. 語の頭の数文字を打ちます。
2. `Ctrl+Space` か `Ctrl+/` を押します（機能 `F_HOKAN`。「ツール」メニューの「入力補完」）。
3. 候補が出たら、選んで確定します。

押したあとの動きは、候補の数で変わります。

| 状態 | 動き |
|---|---|
| キャレットの前に語がない | 一覧は出ず、ステータスバーに「補完対象がありません」と出る |
| 候補がない | 一覧は出ない |
| 候補が 1 つ | すぐにその候補が入る |
| 候補が複数 | 語の始まりの下に一覧が出て、最初の候補が選ばれている |

## 一覧の中のキー

一覧が出ている間は、キーが先に一覧へ行きます。

| キー | 動き |
|---|---|
| `↑` `↓` `PgUp` `PgDn` `Home` `End` | 候補を選ぶ |
| `Enter`、`→` | 選んだ候補を入れる |
| `Tab` | 選んだ候補を入れる（共通設定の「支援」で、`Tab` で確定する設定をオンにしたとき） |
| `Esc` | 一覧を閉じる |
| 文字、`BkSp` | 一覧は残り、キャレットの前の語に合わせて、候補が絞り込まれる。語がなくなるか、候補がなくなると閉じる |
| ほかのキー | 編集の機能として動く |

マウスのクリック、キャレットの移動、ほかの機能の実行でも、一覧は閉じます。

候補を入れると、キャレットの前の語が候補に置き換わります（「Ap」と「apple」で「apple」になります）。1 回の取り消しで戻ります。

確定に使うキー（`Enter`、`Tab`、`→`）の有効・無効は、共通設定の「支援」で決めます。

## 候補の元

候補は、文書のタイプに合わせて集められます。「タイプ別設定」の「支援」で、次を選びます。

- 単語の一覧のファイル
- 開いている文書の中の語
- 強調キーワード

大文字・小文字を同じに扱うかどうかも、同じ場所で決めます。プラグインも候補の元にできます。プラグインには、一覧を開くときに 1 回だけ問い合わせるので、絞り込みの途中では呼び直しません（サクラエディタとの違い）。

候補にキーワードヘルプの辞書の意味があるときは、一覧の横に出ます。

## 根拠

- マツエディタのコード: `crates/matsu-ui/src/support/completion.rs`、`crates/matsu-ui/src/ui_commands/support/hokan.rs`、`crates/matsu-ui/src/support/prefs.rs`、`crates/matsu-ui/src/plugin_complement.rs`、`crates/matsu-commands/src/text/`（`complete`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`Space` と `/` の `Ctrl` の行）
- 設定の画面: `crates/matsu-ui/src/common_settings/pages/support.rs`、`crates/matsu-ui/src/type_settings/pages/support.rs`
- サクラのソース: `sakura_core/cmd/CViewCommander_Support.cpp`（`Command_HOKAN`）、`sakura_core/view/CEditView_CmdHokan.cpp`、`sakura_core/env/CHokanMgr.cpp`
- 差の記録: `docs/sakura-gaps.md`（補完の絞り込みでプラグインを呼び直さない、の記述）
