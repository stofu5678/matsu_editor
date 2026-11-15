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
| DLL | `szMigemoDll` | 未対応 |
| 辞書 | `szMigemoDict` | 未対応 |

MATSU Editor には、migemo がありません。ライセンスの事情で、まだ組み込んでいません（`docs/sakura-gaps.md`）。

## 根拠

- サクラのソース: `sakura_core/prop/CPropComHelper.cpp`、`IDD_PROP_HELPER`
- MATSU のコード: `crates/matsu-ui/src/common_settings/pages/support.rs`、`crates/matsu-ui/src/support/completion`、`crates/matsu-ui/src/support/help`、`crates/matsu-ui/src/key_help/`
- `docs/sakura-gaps.md`（migemo の行）
