# 共通設定: 支援

入力補完の決定キー、外部ヘルプ、キーワードヘルプのフォントを決めます。キーは `[Common]` セクションです。

## 入力補完機能

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 候補決定キー: Enter | `bHokanKey_RETURN` | オン | 補完の候補を、`Enter` で決めます |
| 候補決定キー: Tab | `bHokanKey_TAB` | オフ | `Tab` で決めます |
| 候補決定キー: → | `bHokanKey_RIGHT` | オン | `→` で決めます |

## 外部ヘルプ

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 外部ヘルプの設定 | `szExtHelp` | 空 | 外部ヘルプのファイル。`F_EXTHELP1` で開きます |
| 外部HTMLヘルプの設定 | `szExtHtmlHelp` | 空 | 外部の HTML ヘルプのファイル。`F_EXTHTMLHELP` で開きます |
| ビューアを複数起動しない | `bHtmlHelpIsSingle` | オン | HTML ヘルプのビューアを、1 つだけ起動します |

それぞれの欄の横の `...` のボタンで、ファイルを選べます。タイプ別設定の「支援」ページで、タイプごとに別のヘルプのファイルを決めることもできます。

## キーワードヘルプ

| 項目 | キー | 動き |
|---|---|---|
| フォント... | `khlf`、`khps` | キーワードヘルプのチップのフォントを、フォントのダイアログで選びます。選んだフォントと大きさが、ボタンの横に出ます。OK から使われます |

キーワードヘルプの入り切りは、「設定」メニューの [キーワードヘルプ自動表示](menu-modes.md) です。

## migemo 設定

| 項目 | キー | 状態 |
|---|---|---|
| 項目 | キー | 動き |
|---|---|---|
| DLL | `szMigemoDll` | Migemo のライブラリ（C/Migemo）の場所。「...」で選びます。空のときは、設定ファイルのフォルダーと、プログラムのフォルダーの `migemo.dll`（macOS は `libmigemo.dylib`、Linux は `libmigemo.so`）を探します |
| 辞書 | `szMigemoDict` | 辞書のフォルダー。「...」で選びます。空のときは、設定ファイルのフォルダーの `dict` です。その下の `utf-8/migemo-dict` を使います |

ライブラリと辞書は、MATSU Editor には付いていません。利用者が置きます。相対パスは、設定ファイルのフォルダーを基準にします（DLL は、そこになければプログラムのフォルダー）。

- ライブラリは、フルパスにして読み込みます。カレントフォルダーと `PATH` は探しません。
- Windows では、ライブラリの種類が MATSU Editor と同じでなければ読めません（ARM64 版には ARM64 の DLL、x64 版には x64 の DLL）。
- Windows では、ライブラリが辞書の場所を、システムのコードページ（日本語の Windows なら CP932）で受け取ります。MATSU Editor は、場所をそのコードページに直して渡します。場所に、そのコードページで書けない文字（絵文字など）があると、辞書を読まず、ステータスバーに「MIGEMOの辞書の場所に、システムの文字コードで書けない文字があります。」と出ます。
- 使い方は [インクリメンタルサーチ](../search/incremental-search.md) にあります。

## 根拠

- サクラのソース: `sakura_core/prop/CPropComHelper.cpp`、`IDD_PROP_HELPER`
- MATSU のコード: `crates/matsu-ui/src/common_settings/pages/support.rs`、`crates/matsu-ui/src/support/completion`、`crates/matsu-ui/src/support/help`、`crates/matsu-ui/src/key_help/`
- `docs/sakura-gaps.md`（migemo の行）
- Migemo: `crates/matsu-platform/src/migemo/`、`crates/matsu-ui/src/search/migemo.rs`
