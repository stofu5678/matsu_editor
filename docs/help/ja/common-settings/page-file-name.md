# 共通設定: ファイル名表示

ウィンドウがファイルのパスを出すときに、パスを、短くしたり、決めた名前に置き換えたりします。キーは `[Nickname]` セクションにあります（`[Common]` ではありません）。

## 長いパスの省略表示

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 長いパスの省略表示 | `bShortPath` | オン | 長いパスを、途中を省いて短く出します |
| 文字数 | `nShortPathMaxWidth` | 100（0〜9999） | 省略して出すときの文字数の上限 |

## ファイル名の簡易表示

パスの一部を、別の名前に置き換える表です。

| 列 | キー | 動き |
|---|---|---|
| 置換前 | `From00`、`From01` … | 置き換える元のパスの部分。259 文字まで |
| 置換後 | `To00`、`To01` … | 表示に使う名前。259 文字まで |

- 行数は `ArrNum` に書かれます。最大 16 行です
- OK を押すと、すべてのウィンドウが、この表でパスを出します

## 根拠

- サクラのソース: `sakura_core/prop/CPropComFileName.cpp`、`CShareData_IO::ShareData_IO_Nickname`
- MATSU のコード: `crates/matsu-ui/src/common_settings/pages/file_name.rs`、`crates/matsu-ui/src/file_names/`
