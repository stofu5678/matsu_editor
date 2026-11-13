# すべて上書き保存

開いているすべてのウィンドウを、順に保存します。

- 機能名: `F_FILESAVEALL`
- メニュー: 「ファイル」の「すべて上書き保存」
- 既定のキー: なし
- マクロ: `FileSaveAll()`

## 動き

どのウィンドウにも、「静かな保存」をします。静かな保存は、[上書き保存](file-save.md)とほぼ同じですが、次のところが違います。

- ファイルがない文書（無題）には、名前を聞かず、何もしません
- 変更がない文書は、何も書きません
- ビューモードの文書は、ファイルを上書きしません

改行コードの混在や、書けない文字の確認が必要なウィンドウは、そのウィンドウの中で聞きます。

## サクラエディタとの違い

動きはサクラエディタと同じです。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_File.cpp` の `Command_FILESAVEALL`
- MATSU のコード: `crates/matsu-ui/src/ui_commands/file/file_save_all.rs`、`crates/matsu-ui/src/file_ops/save.rs` の `save_all`・`SaveRequest::quiet`
