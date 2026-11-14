# 共通設定: 編集

コピーと貼り付け、上書きモード、ドラッグ＆ドロップ、ファイルのダイアログの初めの場所などを決めます。キーは `[Common]` セクションです。

## コピー

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 折り返し行に改行を付けてコピー | `bAddCRLFWhenCopy` | オフ | コピーと切り取りで、折り返した行の終わりにも改行を付けます |
| コピーしたら選択解除 | `bCopyAndDisablSelection` | オフ | コピーのあと、選択をやめます |
| 選択なしでコピーを可能にする | `bEnableNoSelectCopy` | オン | 何も選んでいないときのコピーと切り取りが、カーソルのある行を対象にします |
| ラインモード貼り付けを可能にする | `bEnableLineModePaste` | オン | 行をコピーしたものは、行頭に貼り付けます |
| 改行コードを変換して貼り付ける | `bConvertEOLPaste` | オフ | 貼り付ける文字の改行を、文書の改行に変えます |

## ドラッグ & ドロップ

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| ドラッグ & ドロップ編集する | `bUseOLE_DragDrop` | オン | 選んだ文字をマウスで動かして、移したり複写したりできます。ほかから文字をビューに落とすこともできます |
| ドロップ元にする | `bUseOLE_DropSource` | オン | 選択の上でマウスを押して動かすと、ドラッグが始まります。上の項目がオンのときだけ使えます |

## 上書きモード

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 改行コードは上書きしない | `bNotOverWriteCRLF` | オン | 上書きモードで行の終わりに入力すると、改行を消さずに挿入します |
| 文字幅に合わせてスペースを詰める | `bOverWriteFixMode` | オフ | 幅が違う文字で上書きしたとき、幅の差をスペースで埋めます |
| 矩形入力で選択範囲を削除する | `bOverWriteBoxDelete` | オフ | 上書きモードで矩形選択の上に入力すると、選択を削除します |

## クリッカブルURL

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| クリックでURLを選択する | `bSelectClickedURL` | オン | URL の上をクリックすると、その URL を選びます |

## ファイルダイアログの初期位置

| 項目 | キー | 既定 |
|---|---|---|
| カレントフォルダー | `eOpenDialogDir`（0） | 選択 |
| 最近使ったフォルダー | `eOpenDialogDir`（1） | |
| 指定フォルダー | `eOpenDialogDir`（2） | |
| 指定フォルダーの名前 | `szOpenDialogSelDir` | `%Personal%\` |
| Vistaスタイルのファイルダイアログ | `bVistaStyleFileDialog` | 未対応 |

指定フォルダーの欄と「...」のボタンは、「指定フォルダー」を選んだときだけ使えます。

MATSU Editor のファイルダイアログは GPUI が出し、Windows では、いつも Vista スタイルです。切り替える先がないので、この項目は未対応です。

## その他

| 項目 | キー | 既定 | 動き |
|---|---|---|---|
| 改行コードNEL, PS, LSを有効にする | `bEnableExtEol` | オフ | NEL・PS・LS を改行として扱います |
| 矩形選択移動で選択をロックする | `bBoxSelectLock` | オン | 未対応 |

## 根拠

- サクラのソース: `sakura_core/prop/CPropComEdit.cpp`、`IDD_PROP_EDIT`（`sakura_core/sakura_rc.rc`）
- MATSU のコード: `crates/matsu-ui/src/common_settings/pages/edit.rs`、`crates/matsu-config/src/dialog_dir.rs`（`DEFAULT_SELECTED`）
- `docs/sakura-gaps.md` の「共通設定「編集」の灰色 2 個」の行
