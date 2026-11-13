# 共通設定: 書式

日付・時刻を入れる機能の書式、アウトラインの見出し記号、引用符を決めます。どれも MATSU Editor で使われます。キーは `[Common]` セクションです。

## 日付書式

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 標準、カスタム | `nDateFormatType` | 標準（0）。カスタムは 1 | 「カスタム」のときだけ、下の書式の欄が使えます |
| 書式 | `szDateFormat` | `yyyy'年'M'月'd'日('dddd')'` | 日付の書式。`F_INS_DATE`（日付挿入）で使います |

## 時刻書式

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 標準、カスタム | `nTimeFormatType` | 標準（0）。カスタムは 1 | 「カスタム」のときだけ、下の書式の欄が使えます |
| 書式 | `szTimeFormat` | `tthh'時'mm'分'ss'秒'` | 時刻の書式。`F_INS_TIME`（時刻挿入）で使います |

「表示例」の欄（いまの日付・時刻を、書式で出した例）は、MATSU Editor のダイアログにはありません。

## 見出し記号

| キー | 既定 | 動き |
|---|---|---|
| `szMidashiKigou` | 全角の数字、括弧、「第」「一」などの並び | テキストのアウトラインで、見出しの行の頭に使う文字の一覧 |

既定の一覧は、`matsu-outline` の `DEFAULT_HEADING_MARKS` にあります。

## 引用符

| キー | 既定 | 動き |
|---|---|---|
| `szInyouKigou` | `> `（大なりと空白） | 「引用符付きコピー」（`F_COPYLINESASPASSAGE`）で、各行の頭に付ける文字 |

`F_SET_QUOTESTRING` で、この実行の間だけ引用符を変えられます。その値は、ダイアログに出て、OK のときに戻されることはありません。

## 根拠

- サクラのソース: `sakura_core/prop/CPropComFormat.cpp`、`IDD_PROP_FORMAT`
- MATSU のコード: `crates/matsu-ui/src/common_settings/pages/format.rs`、`crates/matsu-commands/src/convert/datetime.rs`（`DEFAULT_DATE_PICTURE`、`DEFAULT_TIME_PICTURE`）、`crates/matsu-outline/src/text.rs`（`DEFAULT_HEADING_MARKS`）、`crates/matsu-ui/src/clipboard/`（`QuoteString`）、`crates/matsu-ui/src/common_settings/mod.rs`
