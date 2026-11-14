# ファイル名・パス名のコピー

開いているファイルの名前や場所を、クリップボードに写します。「編集」→「高度な操作」にあります。

| 機能名 | メニュー | キー | マクロ | 写すもの |
|---|---|---|---|---|
| `F_COPYFNAME` | このファイル名をコピー | `Ctrl+-` | `CopyFilename()` | ファイル名 |
| `F_COPYPATH` | このファイルのパス名をコピー | `Ctrl+\` | `CopyPath()` | ファイルのフルパス |
| `F_COPYTAG` | このファイルのパス名とカーソル位置をコピー | `Ctrl+^`（英語配列ではアポストロフィ） | `CopyTag()` | `パス (行,桁): ` の形 |

## 動き

- ファイルに保存していない文書（無題）では、警告音が鳴るだけで、何も写しません。
- パス名とカーソル位置は、`パス (行,桁): ` の形で写します。行と桁は、カーソルの位置を 1 から数えます。桁は UTF-16 の単位で数えます（サクラエディタの `wchar_t` に合わせています）。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Clipboard.cpp` の `Command_COPYFILENAME`、`Command_COPYPATH`、`Command_COPYTAG`
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/copy_file_name.rs`、`copy_path.rs`、`copy_tag.rs`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`
