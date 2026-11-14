# コピー

選択している文字をクリップボードに写します。

## 呼び出し方

- メニュー: 「編集」→「コピー」
- キー: `Ctrl+C`、`F8`、`Ctrl+Ins`。macOS では `Cmd+C` も使えます
- マクロ: `Copy()`
- 機能名: `F_COPY`

## 動き

- 選択があるときは、その範囲を写します。
- 選択がないときは、カーソルのある行を写し、「行コピー」の印を付けます（共通設定「編集」の「選択なしでコピーを可能にする」と「ラインモード貼り付けを可能にする」が入っているとき。どちらも既定で入っています）。行コピーは、貼り付けると行頭に入ります（[貼り付け](paste.md)）。
- コピーしたあとも、選択は残ります（「コピーしたら選択解除」が切れているとき。既定は切れています）。ただし選択の固定は外れるので、次にカーソルを動かすと選択は解けます。
- 矩形選択のときは、矩形の範囲を写します。これを貼り付けると、矩形として入ります（[貼り付け](paste.md)）。

## 改行コードを決めてコピーする仲間

- [改行コードを決めてコピー](copy-eol.md)
- [折り返し位置に改行をつけてコピー](copy-add-crlf.md)
- [行をまとめてコピー](copy-lines.md)

## 関連

[切り取り](cut.md)、[貼り付け](paste.md)

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `CViewCommander::Command_COPY`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/copy.rs`
- 試験: `crates/matsu-commands/src/commands/clipboard/tests.rs` の `copy_then_paste_round_trips_through_the_clipboard`
- 設定の項目名: `crates/matsu-ui/src/common_settings/pages/edit.rs`（`bEnableNoSelectCopy`、`bEnableLineModePaste`、`bCopyAndDisablSelection`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F8`、`Ins`、`C` の行）
