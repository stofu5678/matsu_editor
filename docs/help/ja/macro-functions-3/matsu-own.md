# MATSU Editor だけの関数

サクラにはなく、MATSU Editor が足した関数です。プレビューのタブ、シェルのタブ、ワンライナー、生成 AI への送信を動かします。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `PreviewTab` | なし | なし | 文書を、プレビューのタブに表示する。 | する | MATSU Editor だけの関数。サクラにはない。 |
| `ShellTab` | なし | なし | シェルを、タブの中で開く。 | する | MATSU Editor だけの関数。サクラにはない。開くだけで、シェルへは何も送らない。テキストを端末へ送る関数は、ない。 |
| `OneLiner0` から `OneLiner49` | なし | なし | 設定の `[OneLiner]` の、番号のスロットに登録したワンライナーを実行する。選択（なければ文書全体）を標準入力へ送り、出力を、選択の置き換え、次の行、出力のパネルのどれかに送る。取り消しは 1 ステップで戻る。 | する | MATSU Editor だけの関数。サクラにはない。登録のないスロットは、「ワンライナー N は登録されていません。」と知らせるだけで、マクロは続く。 |
| `AiPrompt0` から `AiPrompt49` | なし | なし | 設定の `[AI]` の、番号の送り先に、選択（なければ文書全体）を、そのままプロンプトとして標準入力で送る（モード a）。確認の窓が出る（送り先が「次から聞かない」にしていなければ）。答えは、置き換え、次の行、ログのタブだけのどれかに入る。 | する | MATSU Editor だけの関数。サクラにはない。登録のない番号は、「生成 AI の送り先 N は登録されていません。」と知らせるだけで、マクロは続く。選択が空で文書も空のときは、何も送らず、状態バーで知らせる。 |
| `AiAsk` | なし | なし | 「指示をつけて送る」の窓（モード b）を開く関数 `F_AI_ASK` を呼ぶ。窓は、まだなく、何も送らず、状態バーに「指示をつけて送る（モード b）は、まだ使えません。」と知らせる。キーマクロに記録した `AiAsk()` を再生しても、窓が開くだけで、指示は記録に入らない（再生のたびに、人が入れる）。窓ができても、指示の文字列をマクロから渡す手段は、作らない（指示は人が入れる）。 | する | MATSU Editor だけの関数。サクラにはない。 |
| `OneLinerDialog` | なし | なし | 「ワンライナーを入力して実行」のダイアログを開く。履歴と「登録...」がある。 | しない | MATSU Editor だけの関数。サクラにはない。 |


## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`PreviewTab`、`ShellTab`、`OneLiner0` から `OneLiner49`、`OneLinerDialog`、`AiPrompt0` から `AiPrompt49`、`AiAsk`）。この 6 種は、MATSU Editor が `xtask/src/matsu_extras.rs` で足したものです
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/preview_tab.rs`、`shell_tab.rs`、`crates/matsu-ui/src/ui_commands/oneliner/`、`crates/matsu-ui/src/oneliner/flow.rs`（`run_slot`）、`crates/matsu-ui/src/ui_commands/ai/`、`crates/matsu-ui/src/ai_send/`（`run_prompt`）
- 登録した送り先の一覧（`F_AI_PROMPT_LIST`）と設定（`F_AI_SETTINGS`）は、メニューの項目で、マクロはありません。「そのまま送る」の項目は、関数 `F_AI_PROMPT_n` を呼ぶので、`AiPrompt0` から `AiPrompt49` と同じです
- 決定: ADR-0024（プレビュー）、ADR-0028（シェル）、ADR-0029（ワンライナー）、ADR-0035（生成 AI に送る）
