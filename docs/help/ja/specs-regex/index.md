# 仕様等と正規表現

MATSU Editor の決まりごとを、まとめて説明する章です。

| ページ | 内容 |
|---|---|
| [正規表現の構文](regex-syntax.md) | 検索パターンに書ける記法 |
| [置換文字列の書き方](regex-replace.md) | `$1`、`\U` などの記法 |
| [検索の仕組みと上限](search-rules.md) | 行ごとの照合、改行の扱い、置換の進め方 |
| [コマンドラインオプション](command-line.md) | 起動のときに付けるオプション |
| [特殊文字（ExpandParameter）](expand-parameter.md) | `$F`、`${U?...$}` など |
| [文字コード](encodings.md) | 読み書きできる文字コード、自動判定、BOM |
| [改行コード](line-endings.md) | 読む改行、書く改行 |
| [上限と制限](limits.md) | 数や大きさの上限 |

## 根拠

- `docs/adr/0006-regex-engine.md`、`docs/adr/0010-sakura-compatible-settings.md`、`docs/adr/0012-file-size-limit.md`
- 各ページの「根拠」を見てください。
