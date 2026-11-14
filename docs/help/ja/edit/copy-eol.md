# 改行コードを決めてコピー

選択範囲を、改行コードをそろえてクリップボードに写します。次の 3 つは、改行コードだけが違います。

| 機能名 | メニュー | キー | マクロ | 写した文字の改行 |
|---|---|---|---|---|
| `F_COPY_CRLF` | 「編集」→「CRLF改行でコピー」 | `Shift+F8` | `CopyCRLF()` | すべて CRLF |
| `F_COPY_LF` | 「編集」→「LF改行でコピー」 | なし | `CopyLF()` | すべて LF |
| `F_COPY_CR` | 「編集」→「CR改行でコピー」 | なし | `CopyCR()` | すべて CR |

## 動き

- [コピー](copy.md)と同じ操作で、写した文字の改行だけを、決めた改行コードに替えます。文書の中は変わりません。
- 矩形選択をコピーしたときの各行の終わりにも、決めた改行が付きます。

## サクラエディタとの違い

サクラエディタにあるのは、`F_COPY_CRLF` だけです。「LF改行でコピー」と「CR改行でコピー」は MATSU Editor が足したもので、OS をまたいで編集するための機能です。

サクラエディタでは、矩形選択のコピーの各行は、いつも CRLF で終わります。MATSU Editor の LF 版と CR 版は、それぞれ LF、CR で終わります。

## アイコン

3 つとも、メニューとツールバーにアイコンが付きます。「CRLF改行でコピー」の絵はサクラエディタのものです。「LF改行でコピー」と「CR改行でコピー」には、サクラエディタに絵がないので、「CRLF改行でコピー」の絵の右下に、白い板に「LF」「CR」と書いた小さな目印を重ねた絵を、MATSU Editor が作って使います。

ツールバーには、共通設定「ツールバー」の「編集」の一覧から、この 2 つも選んで置けます。ボタンの番号は、サクラエディタの表（481 番まで）の次の 482 番（LF）と 483 番（CR）です。`my_icons.bmp` を使っていても、この 2 つの絵は変わりません。

## 根拠

- サクラのソース: `sakura_core/cmd/CViewCommander.cpp` の `HandleCommand`（`Command_COPY` を改行の種類つきで呼ぶ）
- MATSU のコード: `crates/matsu-commands/src/commands/clipboard/copy_crlf.rs`、`copy_lf.rs`、`copy_cr.rs`（先頭のコメント）
- 試験: `crates/matsu-commands/src/commands/clipboard/copy_eol_tests.rs`
- メニューへの追加: `crates/matsu-config/src/settings/main_menu.rs` の `MATSU_ADDITIONS`
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F8` の行）
