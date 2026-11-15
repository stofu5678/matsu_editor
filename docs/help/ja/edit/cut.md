# 切り取り

選択している文字をクリップボードに写し、文書から消します。

## 呼び出し方

- メニュー: 「編集」→「切り取り」
- キー: `Ctrl+X`、`F7`、`Shift+Del`。macOS では `Cmd+X` も使えます
- マクロ: `Cut()`
- 機能名: `F_CUT`

## 動き

- 選択があるときは、その範囲を写して消します。
- 選択がないときは、カーソルのある行を切り取ります（「行切り取り」と同じです）。これは共通設定「編集」の「選択なしでコピーを可能にする」が入っているとき（既定）の動きです。切った行は「行コピー」として覚えられ、貼り付けると行頭に入ります（[貼り付け](paste.md)）。

## 関連

[コピー](copy.md)、[貼り付け](paste.md)、[行の切り取りと削除](line-cut-delete.md)

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `CViewCommander::Command_CUT`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/cut.rs`
- 設定の項目名: `crates/matsu-ui/src/common_settings/pages/edit.rs`（「選択なしでコピーを可能にする」、`bEnableNoSelectCopy`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F7`、`Del`、`X` の行）
