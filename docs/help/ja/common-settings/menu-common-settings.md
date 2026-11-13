# 共通設定...

すべてのウィンドウに共通の設定を、1 つのダイアログで変えます。

- 機能名: `F_OPTION`
- メニュー: 「設定」→「共通設定...」
- 既定のキー: `Ctrl+6`
- マクロ: `OptionCommon()`（引数なし。キーマクロには記録されません）

## ダイアログの使い方

- 開くと、前に使ったページが出ます。初めてのときは「全般」です
- ページの一覧は、[この章の入り口](index.md#共通設定ダイアログのページ) にあります。ページの名前を押すと、そのページに移ります
- `Tab` で部品の間を動き、`Ctrl+Tab` か `Ctrl+PageDown` で次のページ、`Ctrl+Shift+Tab` か `Ctrl+PageUp` で前のページへ移ります
- 部品の名前の後ろの `(&F)` のような印は、`Alt` と組み合わせるアクセスキーです
- `Enter` が「OK」、`Esc` が「キャンセル」です
- 「OK」を押すと、すべてのページの設定が `matsu.ini` に書かれ、開いているすべてのウィンドウが、すぐに新しい設定を使い始めます
- 「キャンセル」は、何も書きません
- 例外として、「全般」ページの「履歴をクリア」は、確認のあとすぐに履歴を消してファイルへ書きます。「キャンセル」では戻りません

## 「未対応」の部品

MATSU Editor が使っていない設定の部品は、灰色で表示され、名前の後ろに「（未対応）」と付きます。ファイルの値を表示しますが、書き換えません。ファイルにある値は、そのまま残ります。

## ページを直接開く機能

ページごとに、そのページを開く機能があります。メニューには出ません。

| 機能名 | 開くページ |
|---|---|
| `F_OPTION_GENERAL` | 全般 |
| `F_OPTION_WINDOW` | ウィンドウ |
| `F_OPTION_MAINMENU` | メインメニュー |
| `F_OPTION_TOOLBAR` | ツールバー |
| `F_OPTION_TAB` | タブバー |
| `F_OPTION_STATUSBAR` | ステータスバー |
| `F_OPTION_EDIT` | 編集 |
| `F_OPTION_FILE` | ファイル |
| `F_OPTION_FNAME` | ファイル名表示 |
| `F_OPTION_BACKUP` | バックアップ |
| `F_OPTION_FORMAT` | 書式 |
| `F_OPTION_GREP` | 検索 |
| `F_OPTION_KEYBIND` | キー割り当て |
| `F_OPTION_CUSTMENU` | カスタムメニュー |
| `F_OPTION_KEYWORD` | 強調キーワード |
| `F_OPTION_HELPER` | 支援 |
| `F_OPTION_MACRO` | マクロ |
| `F_OPTION_PLUGIN` | プラグイン |
| `F_ONELINER_SETTINGS`（MATSU Editor だけ） | ワンライナー |

「シェル」ページを直接開く機能は、ありません。

## サクラエディタとの違い

- ページの数と並びは、サクラエディタの共通設定に、「シェル」と「ワンライナー」を足したものです
- ダイアログは、MATSU Editor の窓です（OS 標準のプロパティシートではありません）。OK と キャンセル のボタンの組み合わせです
- 設定を OK で書くとき、ファイルのほかの部分と、ほかのウィンドウが書いた変更は消えません。ファイルをロックして書きます
- 使っていない部品を灰色で見せる決まりは、MATSU Editor で入れたものです

## 根拠

- サクラのソース: `sakura_core/prop/CPropCommon.cpp` の `CPropCommon::DoPropertySheet`（前に使ったページ `m_nPageNum` を開く）
- MATSU のコード: `crates/matsu-ui/src/common_settings/mod.rs`（`Page::ALL`、ファイルの書き方、「未対応」の決まり）、`crates/matsu-ui/src/common_settings/dialog.rs`（キー操作）、`crates/matsu-ui/src/ui_commands/settings/option.rs` と `option_*.rs`、`oneliner_settings.rs`、`crates/matsu-commands/src/sakura/macro_functions.rs`（`OptionCommon`）、`key_bindings.rs`（`Ctrl+6`）
