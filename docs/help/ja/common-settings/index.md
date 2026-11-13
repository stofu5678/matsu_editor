# コマンド: 設定（共通設定）

「設定」メニューの機能と、「共通設定」ダイアログの各ページを説明する章です。

「設定」メニューにある「タイプ別設定一覧」と「タイプ別設定」は、「コマンド: 設定（タイプ別設定）とウィンドウ」の章にあります。

## 「設定」メニューの機能

| メニュー項目 | 機能名 | 既定のキー | ページ |
|---|---|---|---|
| ツールバーの表示／非表示 | `F_SHOWTOOLBAR` | `Ctrl+1` | [バーの表示と非表示](menu-bars.md) |
| ファンクションキーの表示／非表示 | `F_SHOWFUNCKEY` | `Ctrl+2` | [バーの表示と非表示](menu-bars.md) |
| タブバーの表示／非表示 | `F_SHOWTAB` | なし | [バーの表示と非表示](menu-bars.md) |
| ステータスバーの表示／非表示 | `F_SHOWSTATUSBAR` | `Ctrl+3` | [バーの表示と非表示](menu-bars.md) |
| ミニマップの表示／非表示 | `F_SHOWMINIMAP` | なし | [バーの表示と非表示](menu-bars.md) |
| 共通設定... | `F_OPTION` | `Ctrl+6` | [共通設定...](menu-common-settings.md) |
| フォント設定... | `F_FONT` | `Ctrl+7` | [フォント設定...](menu-font.md) |
| 履歴の管理... | `F_FAVORITE` | なし | [履歴の管理...](menu-history.md) |
| 折り返し方法（折り返さない・指定桁で折り返す・右端で折り返す） | `F_TMPWRAPNOWRAP` `F_TMPWRAPSETTING` `F_TMPWRAPWINDOW` | `Ctrl+Alt+X` `Ctrl+Alt+S` `Ctrl+Alt+W` | [折り返しの切り替え](menu-wrap.md) |
| 折り返し桁数 | `F_WRAPWINDOWWIDTH` | `Ctrl+Alt+U` | [折り返しの切り替え](menu-wrap.md) |
| 文字カウント方法 | `F_SELECT_COUNT_MODE` | なし | [文字カウント方法](menu-count-mode.md) |
| 挿入／上書きモード切り替え | `F_CHGMOD_INS` | `Insert` | [モードと文字コード・改行コード](menu-modes.md) |
| ビューモード | `F_VIEWMODE` | なし | [モードと文字コード・改行コード](menu-modes.md) |
| キーワードヘルプ自動表示 | `F_TOGGLE_KEY_SEARCH` | なし | [モードと文字コード・改行コード](menu-modes.md) |
| 文字コードセット指定... | `F_CHG_CHARSET` | なし | [モードと文字コード・改行コード](menu-modes.md) |
| 入力改行コード指定（CRLF・LF・CR） | `F_CHGMOD_EOL_CRLF` `F_CHGMOD_EOL_LF` `F_CHGMOD_EOL_CR` | なし | [モードと文字コード・改行コード](menu-modes.md) |
| タイプ別設定一覧...、タイプ別設定... | `F_TYPE_LIST` `F_OPTION_TYPE` | `Ctrl+4` `Ctrl+5` | 「コマンド: 設定（タイプ別設定）とウィンドウ」の章 |

「既定のキー」は、サクラエディタ互換の既定の割り当てです（`key_bindings.rs`）。macOS では、MATSU Editor の macOS 用の割り当てが重なります。「キー割り当て」ページで変えられます。

## 「共通設定」ダイアログのページ

ダイアログのページは、次の順に並びます。

| ページ | 何の設定か |
|---|---|
| [全般](page-general.md) | カーソル、スクロール、履歴の数、閉じるときの確認 |
| [ウィンドウ](page-window.md) | バーの表示、ルーラー、タイトルバーの書式、分割ウィンドウ |
| [メインメニュー](page-main-menu.md) | メニューバーの組み立て |
| [ツールバー](page-toolbar.md) | ツールバーのボタン |
| [タブバー](page-tab.md) | タブバーの表示と動き |
| [ステータスバー](page-status-bar.md) | 文字コードと桁の表示 |
| [編集](page-edit.md) | コピー・貼り付け、上書きモード、ドラッグ＆ドロップ |
| [ファイル](page-file.md) | 排他制御、保存、ファイルを開くとき |
| [ファイル名表示](page-file-name.md) | パスの省略と置き換え |
| [バックアップ](page-backup.md) | 保存時のバックアップ |
| [書式](page-format.md) | 日付・時刻の書式、見出し記号、引用符 |
| [検索](page-search.md) | 検索、Grep、タグジャンプ |
| [キー割り当て](page-keys.md) | キーと機能の対応 |
| [カスタムメニュー](page-custom-menu.md) | 右クリックメニューなど 25 個のメニュー |
| [強調キーワード](page-keyword.md) | 強調キーワードのセット |
| [支援](page-support.md) | 入力補完の決定キー、外部ヘルプ、キーワードヘルプ |
| [マクロ](page-macro.md) | 登録マクロと自動実行 |
| [プラグイン](page-plugin.md) | プラグインの有効化と一覧 |
| [シェル](page-shell.md) | シェルのタブの設定（MATSU Editor だけ） |
| [ワンライナー](page-oneliner.md) | 登録ワンライナー（MATSU Editor だけ） |

「シェル」と「ワンライナー」は、サクラエディタにはないページです。

## ページの読み方

どのページも、設定の項目を表にしています。

- 「キー」は、`matsu.ini` の `[Common]` セクション（そうでないときは、表の上に節の名前を書いています）にある名前です。名前はサクラエディタの `sakura.ini` と同じです。`matsu.ini` については「はじめに」の章の「設定ファイル matsu.ini」を見てください
- 「既定」は、設定ファイルにキーがないときの値です
- 「未対応」と書いた項目は、画面には出ますが、MATSU Editor が使っていません。灰色で表示され、値は書き換えません。ファイルの値はそのまま残ります

## 根拠

- MATSU のコード: `crates/matsu-commands/src/sakura/main_menu.rs`（「設定」メニュー）、`crates/matsu-commands/src/sakura/key_bindings.rs`（既定のキー）、`crates/matsu-ui/src/common_settings/mod.rs`（ページの並び `Page::ALL`）
