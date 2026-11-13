# タイプ別設定: 支援

入力補完、外部ヘルプ、保存するときの警告、C/C++ のインデントの細かい決め、ファイルを開くときのインデントの検出を決めるページです。キーは、`matsu.ini` の `[Types(N)]` にあります。

## 入力補完機能

入力補完で出す候補の元を決めます。入力補完の使い方は、「コマンド: ツール」の章の「入力補完」にあります。

| 項目 | キー | 動き |
|---|---|---|
| 単語ファイル | `szHokanFile` | 候補にする単語を 1 行に 1 つ書いたファイル。右の「...」で選ぶ |
| 入力補完機能の種類 | `nHokanType` | 「なし」（0）か、補完をするプラグインを選ぶ |
| 英大文字小文字を同一視 | `bHokanLoHiCase` | 候補を探すとき、大文字と小文字を区別しない |
| 候補：編集中のファイル | `bUseHokanByFile` | 開いている文書の単語を、候補にする |
| 候補：強調キーワード | `bUseHokanByKeyword` | このタイプの強調キーワードを、候補にする |

- 「入力補完機能の種類」の一覧には、補完を足すプラグインが、「なし」の後ろに並びます。そのプラグインが使えなくなっても、ファイルの値はそのまま残り、「（使えないプラグイン N）」と出ます。サクラエディタは「なし」を見せて、OK で 0 を書きます。

## 外部ヘルプの設定

| 項目 | キー | 動き |
|---|---|---|
| 外部ヘルプの設定 | `szExtHelp` | このタイプの外部ヘルプのファイル（`*.hlp`、`*.chm`、`*.col`）。空なら、共通設定の設定を使う |
| 外部HTMLヘルプの設定 | `szExtHtmlHelp` | このタイプの外部 HTML ヘルプのファイル（`*.chm`、`*.col`）。空なら、共通設定の設定を使う |
| ビューアを複数起動しない | `bTypeHtmlHelpIsSingle` | HTML ヘルプのビューアを、1 つだけ起動する |

「外部ヘルプ１」と「外部HTMLヘルプ」の機能（`F_EXTHELP1`、`F_EXTHTMLHELP`）は、ここに書いてあれば、共通設定より先にこのタイプのものを使います。

## 保存時の警告

| 項目 | キー | 動き |
|---|---|---|
| 保存時に改行コードの混在を警告する | `bChkEnterAtEnd` | 1 つの文書に、種類の違う改行コードが混ざっていて、保存するときに改行コードを変換しないなら、「いまの改行コードにそろえるか」を聞く。オフにすると聞かない。マクロの `FileSaveAs` は、この設定にかかわらず聞かずに保存する |

## C/C++インデント詳細設定

「スクリーン」ページで、スマートインデントを「C/C++」にしたときの動きです。

| 項目 | キー | 動き |
|---|---|---|
| 文字列を無視する | `bIndentCppStringIgnore` | 文字列や文字定数の中の記号を、インデントの判断に使わない |
| コメントを無視する | `bIndentCppCommentIgnore` | コメントの中の記号を、インデントの判断に使わない |
| Undoバッファを分ける | `bIndentCppUndoSep` | スマートインデントが変えた分を、元に戻す操作の 1 つとして分ける |

## インデントスタイルの検出

| 項目 | キー | 動き |
|---|---|---|
| ファイル読み込み時にインデントスタイルを検出する | `bDetectIndentationStyleOnFileLoad` | ファイルを開いたとき、そのファイルのインデントが、タブか空白か、幅がいくつかを調べて、その文書だけに当てはめる |

検出の規則は次のとおりです。

1. ファイルの最初の 256 行を調べます。1 行は、先頭の部分だけを見ます。
2. ファイルに関係する `.editorconfig` に `indent_style` があれば、その指定を、文字の調べ方より優先します。
3. なければ、行頭の文字で決めます。タブで始まる行が、空白で始まる行の 4 倍より多ければタブ、空白で始まる行がタブの行の 4 倍より多ければ空白とします。差が小さいときは、決めません。
4. 空白と決めたときは、行ごとに「前の行とのインデントの差」を数え、いちばん多い差を幅とします。
5. 決まったことは、その文書の「TAB幅」と「SPACEの挿入」にだけ当てはまります。タイプの設定は、書き換えません。

- 幅が 1〜64 に入らなければ、幅は使いません。
- 読み込みがまだ終わっていない大きなファイルでは、読み込みの終わりに検出します。

## サクラエディタとの違い

- 検出で、`.editorconfig` の `[*.ext]` の形は、拡張子が一致するファイルだけに当てはまります。サクラエディタは、前方一致で比べるので `[*.cpp]` が `.c` のファイルにも当たってしまいます。サクラの不具合として、MATSU Editor は直しています。
- 検出は、行頭の文字を、先頭の部分だけ範囲で読みます。長い行や大きなファイルでも、時間は変わりません。
- 幅の上限（64）を超える `.editorconfig` の値は、取り入れません。サクラエディタは、どんな数でも、そのまま配置に渡します。

## 根拠

- サクラのソース: `sakura_core/typeprop/CPropTypesSupport.cpp`、`IDD_PROP_SUPPORT`（`sakura_core/sakura_rc.rc`）、`sakura_core/parse/DetectIndentationStyle.cpp`、`sakura_core/doc/CDocEditor.cpp` の `OnAfterLoad`、`sakura_core/agent/CSaveAgent.cpp` の `OnCheckSave`
- MATSU のコード: `crates/matsu-ui/src/type_settings/pages/support.rs`、`crates/matsu-ui/src/support/completion.rs`、`crates/matsu-ui/src/support/help.rs`、`crates/matsu-ui/src/file_ops/save.rs`（`warn_mixed_eol`）、`crates/matsu-ui/src/settings/view_settings.rs`（`cpp_string_ignore`、`cpp_comment_ignore`、`cpp_undo_sep`、`detect_indentation`）、`crates/matsu-core/src/indent_style.rs`、`crates/matsu-ui/src/view/settings.rs`
- 試験: `crates/matsu-core/src/indent_style.rs` の試験、`crates/matsu-ui/src/type_settings/tests.rs`
