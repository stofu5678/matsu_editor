# インデント（TAB と SPACE）

行頭に TAB や空白を足したり、取ったりする 4 つの操作です。メニューは「編集」→「高度な操作」にあります。

| 機能名 | メニュー | キー | マクロ |
|---|---|---|---|
| `F_INDENT_TAB` | TABインデント | `Tab` | `IndentTab()` |
| `F_UNINDENT_TAB` | 逆TABインデント | `Shift+Tab` | `UnindentTab()` |
| `F_INDENT_SPACE` | SPACEインデント | `Space` | `IndentSpace()` |
| `F_UNINDENT_SPACE` | 逆SPACEインデント | `Shift+Space` | `UnindentSpace()` |

既定のキーの割り当てでは、`Space` が `F_INDENT_SPACE`、`Shift+Space` が `F_UNINDENT_SPACE` です。選択がないときの `Space` は、空白を 1 つ打つだけです。

## TABインデント / SPACEインデント

- 選択がない、または選択が 1 行の中に収まるときは、選択の代わりに TAB（または空白）を 1 つ打ちます。
- 選択が複数の行にわたるときは、文字のある行ごとに、行頭へ TAB（または空白）を足します。折り返しの途中の行には足しません。足したあとは、対象の行全体が、先頭から最終行の次の行の先頭まで選ばれます。
- 矩形選択のときは、矩形の左端に、TAB（または空白）を入れます。矩形のところに文字がない行には入りません。
- 幅 0 の矩形のときの「SPACEインデント」は、まず矩形を右端まで広げるだけで、空白は入れません。もう一度使うと入ります。
- 幅が 1 桁の矩形で、全角文字がちょうどそこに収まる行は、そのままにします。
- 「ソフトタブ」（タイプ別設定で、TAB の代わりに空白を入れる設定）のときは、TAB の代わりに、次の TAB 位置までの空白を入れます。桁は表示桁で数えます（全角は 2 桁）。入れる場所ごとに、空白の数を別に数えます。上書きモードでは、打った文字と同じに、カーソルの文字を置き換えます。

## 逆TABインデント / 逆SPACEインデント

| 機能 | 選択がないとき | 選択があるとき | 矩形選択 |
|---|---|---|---|
| 逆TABインデント | TAB を 1 つ打つ（ステータスバーにメッセージが出る） | 行頭の TAB を 1 つ取る。TAB がなければ、行頭の空白を、TAB の幅まで取る | 警告音のみ |
| 逆SPACEインデント | 空白を 1 つ打つ（同じメッセージが出る） | 行頭が空白の行から、空白を 1 つ取る | 中身のある矩形なら警告音のみ |

- 選択が 1 行の中でも、「選択があるとき」の動きをします。
- 折り返しの途中の行からは取りません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander_Edit_advanced.cpp` の `Command_INDENT`、`Command_UNINDENT`
- MATSU のコード: `crates/matsu-commands/src/commands/edit/indent_tab.rs`、`unindent_tab.rs`、`indent_space.rs`、`unindent_space.rs`（先頭のコメント）
- 試験: 上の 4 つのファイルの `tests` モジュール
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`Tab`、`Space` の行）
