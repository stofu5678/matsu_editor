# MATSU Editor だけの関数

サクラにはなく、MATSU Editor が足した関数です。プレビューのタブ、シェルのタブ、ワンライナーを動かします。

## 表

| マクロ名 | 引数 | 戻り値 | 動き | キーマクロに記録 | MATSU での違い |
|---|---|---|---|---|---|
| `PreviewTab` | なし | なし | 文書を、プレビューのタブに表示する。 | する | MATSU Editor だけの関数。サクラにはない。 |
| `ShellTab` | なし | なし | シェルを、タブの中で開く。 | する | MATSU Editor だけの関数。サクラにはない。開くだけで、シェルへは何も送らない。テキストを端末へ送る関数は、ない。 |
| `OneLiner0` から `OneLiner49` | なし | なし | 設定の `[OneLiner]` の、番号のスロットに登録したワンライナーを実行する。選択（なければ文書全体）を標準入力へ送り、出力を、選択の置き換え、次の行、出力のパネルのどれかに送る。取り消しは 1 ステップで戻る。 | する | MATSU Editor だけの関数。サクラにはない。登録のないスロットは、「ワンライナー N は登録されていません。」と知らせるだけで、マクロは続く。 |
| `OneLinerDialog` | なし | なし | 「ワンライナーを入力して実行」のダイアログを開く。履歴と「登録...」がある。 | しない | MATSU Editor だけの関数。サクラにはない。 |


## 根拠

- 表の元: `crates/matsu-commands/src/sakura/macro_functions.rs`（`PreviewTab`、`ShellTab`、`OneLiner0` から `OneLiner49`、`OneLinerDialog`）。この 4 種は、MATSU Editor が `xtask/src/matsu_extras.rs` で足したものです
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/preview_tab.rs`、`shell_tab.rs`、`crates/matsu-ui/src/ui_commands/oneliner/`、`crates/matsu-ui/src/oneliner/flow.rs`（`run_slot`）
- 決定: ADR-0024（プレビュー）、ADR-0028（シェル）、ADR-0029（ワンライナー）
