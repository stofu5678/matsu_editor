# 概要

MATSU Editor（マツエディタ）は、サクラエディタの操作感を macOS・Windows・Linux で使えるようにしたテキストエディタです。Rust で書かれていて、MIT License と Apache License 2.0 のどちらかを選んで使えます。

サクラエディタとは別のプロジェクトです。サクラエディタのソースとヘルプ（zlib License）を、仕様の参考にしています。

## 何ができるか

- サクラエディタと同じメニュー、機能の名前、既定のキー割り当てで編集できます
- 設定は、サクラエディタの `sakura.ini` と同じ書式の `matsu.ini` に保存します（[設定ファイル matsu.ini](settings-file.md)）
- 検索と置換に、サクラエディタの正規表現に合わせた正規表現（Onigmo）を使えます
- マクロとプラグインを書けます。マクロの言語は、Lua・JavaScript・Ruby・Python などです
- 大きさに上限を決めずにファイルを開けます。とても大きなファイルは、範囲ごとに読み込みます
- Windows・macOS・Linux で動きます

MATSU Editor だけの機能もあります。たとえば、シェルのタブ、プレビュー、保存していない文書の自動保存、データベース操作です。サクラエディタにはありません。

## 対応する環境

| OS | CPU | 配布の形 |
|---|---|---|
| Windows | x64、ARM64 | インストーラ、zip |
| macOS | ARM64（Apple シリコン） | `.dmg`、tar.gz |
| Linux | x64、ARM64 | `.deb`、`.rpm`、tar.gz |

入れかたは [インストールと起動](install.md) に書きました。

## 今の状態

サクラエディタの機能を、順に作っています。まだ入っていない機能もあります。入っていないものの一覧は、[サクラエディタとの違い](differences.md)にまとめました。各ページには、その機能が今どう動くかを書いています。

## このヘルプの読みかた

- メニューや機能の名前は、MATSU Editor の画面に出る文字と同じです。その名前は、サクラエディタと同じです
- キーは `Ctrl+S` のように書きます
- 各ページの終わりの「根拠」は、書いてあることを確かめた場所です

## 根拠

- 製品の説明: `README.md`、`docs/roadmap.md`、`docs/adr/0003-language-and-license.md`
- 設定の書式: `docs/adr/0010-sakura-compatible-settings.md`
- 配布の形: `docs/release.md` の「アセットの名前」
- 名前の決まり: `docs/adr/0021-rename-to-matsu.md`
