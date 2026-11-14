# ブラウズ

今のファイルを、OS が決めている既定のプログラムで開きます。

- 機能名: `F_BROWSE`
- メニュー: 「ファイル」の「ブラウズ」
- 既定のキー: `Ctrl+B`
- マクロ: `Browse()`

## 動き

OS の「ファイルを開く」の動きを、今のファイルに対して行います。たとえば、HTML ファイルなら、ブラウザーで開きます。Windows では `ShellExecute`、macOS では `open`、Linux では `xdg-open` と同じです。

ファイルがない文書（無題）では、「ファイルがないので使えない」と出て、何も開きません。

編集中の内容は、ディスクに保存したものと違うことがあります。ブラウズで開くのは、ディスクにあるファイルです。見たい内容を保存してから使います。

## サクラエディタとの違い

ファイルがない文書で、サクラエディタは音を鳴らします。MATSU Editor は、メッセージを出します。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_BROWSE`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/browse.rs`、`crates/matsu-ui/src/file_ops/external.rs` の `document_file`（メッセージ）
