# コマンド: 設定（タイプ別設定）とウィンドウ

この章は、2 つのことを説明します。

- 「設定」メニューの「タイプ別設定一覧...」と「タイプ別設定...」、そして、タイプ別設定のダイアログの 6 つのページ
- 「ウィンドウ」メニューの機能

「設定」メニューのほかの機能（共通設定、フォント、バーの表示など）は、「コマンド: 設定（共通設定）」の章にあります。

## タイプ別設定

タイプは、ファイルの種類ごとの設定のまとまりです。ファイル名から自動で決まり、色、折り返し、タブ幅、新しい文書の文字コードなどを、タイプごとに変えられます。

| ページ | 内容 |
|---|---|
| [タイプ別設定一覧...](type-list.md) | タイプの一覧。一時適用、追加、複製、削除、並べ替え、初期化、インポート、エクスポート |
| [タイプ別設定...](type-settings.md) | いまの文書のタイプの設定を開く。ダイアログの作りと、タイプの決まり方 |
| [スクリーン](type-page-screen.md) | 名前と拡張子、折り返し、タブ幅、インデント、アウトライン解析、フォント、禁則処理 |
| [カラー](type-page-color.md) | 色、強調キーワードの選択、コメントと文字列の書き方、縦線 |
| [ウィンドウ](type-page-window.md) | 新しい文書の文字コードと改行コード、IME、行番号、背景画像 |
| [支援](type-page-support.md) | 入力補完、外部ヘルプ、保存時の警告、インデントの検出 |
| [正規表現キーワード](type-page-regex.md) | 正規表現で色を付ける規則 |
| [キーワードヘルプ](type-page-keyhelp.md) | 言葉の意味を引く辞書 |

## ウィンドウメニュー

「ウィンドウ」メニューの機能です。メニューに出る順に並べています。

### 分割

| メニュー項目 | 機能名 | 既定のキー | ページ |
|---|---|---|---|
| 上下に分割／分割解除 | `F_SPLIT_V` | `F4` | [上下に分割／分割解除](split-v.md) |
| 左右に分割／分割解除 | `F_SPLIT_H` | `Shift+F4` | [左右に分割／分割解除](split-h.md) |
| 縦横に分割／分割解除 | `F_SPLIT_VH` | `Ctrl+Shift+;` | [縦横に分割／分割解除](split-vh.md) |

### 閉じる

| メニュー項目 | 機能名 | 既定のキー | ページ |
|---|---|---|---|
| 閉じる | `F_WINCLOSE` | `Alt+F4` | 「コマンド: ファイル」の章の「閉じる」 |
| すべて閉じる | `F_WIN_CLOSEALL` | なし | [すべて閉じる](win-closeall.md) |
| このタブ／ウィンドウ以外を閉じる | `F_TAB_CLOSEOTHER` | なし | [このタブ／ウィンドウ以外を閉じる](tab-closeother.md) |

### 移る

| メニュー項目 | 機能名 | 既定のキー | ページ |
|---|---|---|---|
| 次のウィンドウ | `F_NEXTWINDOW` | `Ctrl+Tab` | [次のウィンドウ](nextwindow.md) |
| 前のウィンドウ | `F_PREVWINDOW` | `Ctrl+Shift+Tab` | [前のウィンドウ](prevwindow.md) |
| ウィンドウ一覧... | `F_WINLIST` | なし | [ウィンドウ一覧...](winlist.md) |
| ウィンドウ一覧表示... | `F_DLGWINLIST` | なし | [ウィンドウ一覧表示...](dlgwinlist.md) |

### 並べる

| メニュー項目 | 機能名 | 既定のキー | ページ |
|---|---|---|---|
| 重ねて表示 | `F_CASCADE` | `Ctrl+Alt+E` | [重ねて表示](cascade.md) |
| 上下に並べて表示 | `F_TILE_V` | `Ctrl+Alt+H` | [上下に並べて表示](tile-v.md) |
| 左右に並べて表示 | `F_TILE_H` | `Ctrl+Alt+T` | [左右に並べて表示](tile-h.md) |
| 常に手前に表示 | `F_TOPMOST` | なし | [常に手前に表示](topmost.md) |
| 縦方向に最大化 | `F_MAXIMIZE_V` | `Ctrl+Alt+Shift+↑` | [縦方向に最大化](maximize-v.md) |
| 横方向に最大化 | `F_MAXIMIZE_H` | `Ctrl+Alt+Shift+→` | [横方向に最大化](maximize-h.md) |
| すべて最小化 | `F_MINIMIZE_ALL` | `Ctrl+Alt+Shift+↓` | [すべて最小化](minimize-all.md) |

### タブの操作

「タブの操作」のサブメニューです。

| メニュー項目 | 機能名 | ページ |
|---|---|---|
| グループ化 | `F_BIND_WINDOW` | [グループ化](bind-window.md) |
| グループを閉じる | `F_GROUPCLOSE` | 「コマンド: ファイル」の章の「グループを閉じる」 |
| 左をすべて閉じる | `F_TAB_CLOSELEFT` | [左をすべて閉じる](tab-closeleft.md) |
| 右をすべて閉じる | `F_TAB_CLOSERIGHT` | [右をすべて閉じる](tab-closeright.md) |
| 次のグループ | `F_NEXTGROUP` | [次のグループ](nextgroup.md) |
| 前のグループ | `F_PREVGROUP` | [前のグループ](prevgroup.md) |
| タブを右に移動 | `F_TAB_MOVERIGHT` | [タブを右に移動](tab-moveright.md) |
| タブを左に移動 | `F_TAB_MOVELEFT` | [タブを左に移動](tab-moveleft.md) |
| 新規グループ | `F_TAB_SEPARATE` | [新規グループ](tab-separate.md) |
| 次のグループに移動 | `F_TAB_JOINTNEXT` | [次のグループに移動](tab-jointnext.md) |
| 前のグループに移動 | `F_TAB_JOINTPREV` | [前のグループに移動](tab-jointprev.md) |

これらの機能に、既定のキーはありません。

### 表示

| メニュー項目 | 機能名 | 既定のキー | ページ |
|---|---|---|---|
| 再描画 | `F_REDRAW` | `F5` | [再描画](redraw.md) |
| アウトプット | `F_WIN_OUTPUT` | なし | [アウトプット](win-output.md) |
| プレビューを別のタブで開く | `F_PREVIEW_TAB` | なし | 「MATSU Editor だけの機能」の章の「プレビュー」（MATSU Editor の独自機能） |
| シェルのタブを開く | `F_SHELL_TAB` | なし | 「MATSU Editor だけの機能」の章の「シェル」（MATSU Editor の独自機能） |

### ウィンドウのリスト

メニューの最後の「ウィンドウ一覧」のサブメニューには、サクラエディタでは、開いているウィンドウの名前が並びます（`F_WINDOW_LIST`）。MATSU Editor のメニューでは、いまのところ、この一覧を入れる処理が見つからず、「(なし)」と灰色で出ます。ウィンドウの一覧を見るには、「ウィンドウ一覧...」か「ウィンドウ一覧表示...」を使ってください。

## タブモードとグループ

ウィンドウのメニューの機能には、タブを使っているかどうかで動きが変わるものがあります。

- 共通設定の「タブバー」ページで、タブバーを表示していて、「タブをまとめない」がオフのとき、これを**タブモード**と呼びます。タブモードでは、1 つのウィンドウが複数の文書をタブで持ちます。
- サクラエディタの「グループ」は、1 つの枠にまとまったウィンドウの集まりです。MATSU Editor では、1 つのウィンドウが 1 つのグループに当たります（ADR-0015）。
- タブモードでないとき、文書は 1 つずつ別のウィンドウで開き、すべてのウィンドウが、1 つのグループです。
- タブモードにすると（タブバーを出す、または「グループ化」でまとめる）、すべてのウィンドウの文書が、いまのウィンドウのタブに集まります。タブモードをやめると、どのタブも自分のウィンドウを持ちます。

タブバーとタブの使い方そのものは、「基本的な使い方」の章にあります。

## キーの割り当てについて

この章の「既定のキー」は、サクラエディタ互換の既定の割り当てです。macOS では、MATSU Editor の macOS 用の割り当てが重なります。「共通設定」の「キー割り当て」ページで変えられます。

## 根拠

- サクラのソース: `sakura_core/func/CKeyBind.cpp`（既定のキー）、`sakura_core/Funccode_x.hsrc`（機能の一覧）、`sakura_core/cmd/CViewCommander_Window.cpp`
- MATSU のコード: `crates/matsu-commands/src/sakura/main_menu.rs`（「ウィンドウ」のノード）、`crates/matsu-commands/src/sakura/functions.rs`、`crates/matsu-commands/src/sakura/key_bindings.rs`、`crates/matsu-commands/src/keymap/map.rs`（`default_function`）、`crates/matsu-ui/src/menu.rs`（`special`）、`crates/matsu-ui/src/tabs/settings.rs`（`bound`）
- `docs/adr/0015-tabs-and-groups.md`
