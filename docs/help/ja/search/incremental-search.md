# インクリメンタルサーチ

パネルを開かずに、文字を打つたびにその場で探し直す検索です。

- メニュー: 検索 → インクリメンタルサーチ
- 既定のキー: なし（メニューか、キー割り当てで呼びます）
- マクロ: なし

## 機能

| 機能 | 機能名 | 検索 |
|---|---|---|
| 前方インクリメンタルサーチ | `F_ISEARCH_NEXT` | 文字列、後ろへ |
| 後方インクリメンタルサーチ | `F_ISEARCH_PREV` | 文字列、前へ |
| 正規表現前方インクリメンタルサーチ | `F_ISEARCH_REGEXP_NEXT` | 正規表現、後ろへ |
| 正規表現後方インクリメンタルサーチ | `F_ISEARCH_REGEXP_PREV` | 正規表現、前へ |
| MIGEMO前方インクリメンタルサーチ | `F_ISEARCH_MIGEMO_NEXT` | まだない |
| MIGEMO後方インクリメンタルサーチ | `F_ISEARCH_MIGEMO_PREV` | まだない |

MIGEMO の 2 つは、メニューに名前がありますが、動きません。Migemo の辞書と DLL の扱いを、ライセンスの面で決めてからになります（`docs/sakura-gaps.md`）。

## 使い方

1. メニューで開始する。ステータスバーに `I-Search:`（正規表現は `[RegExp] I-Search:`、後方は `I-Search Backward:`）と出る。
2. 文字を打つ。打った文字が探す文字列に足され、そのたびに、いまの一致から探し直す。
3. 一致が見つからないと、ステータスバーの文字列の後ろに「 (見つかりません)」が付く。
4. 同じ機能をもう一度呼ぶと、次の一致へ進む（前方 / 後方の機能を替えれば向きが変わる）。見つからないまま、もう一度呼ぶと、文書の反対の端から探し直す。
5. `Backspace` で 1 つ前の状態（文字列と選択）に戻る。
6. 文字入力と `Backspace` 以外の操作（カーソルの移動、編集、別のコマンドなど）をすると、モードが終わる。

## 細かい決まり

- モードの間は、英大文字と小文字を区別しません。文字列の検索では、単語単位も切ります。
- モードを始めるとき、選択は外れます。
- `Tab` と `Space` は、文書に入れる文字ではなく、探す文字列に足す文字です。IME で確定した文字も足されます。
- モードが終わったあとも、探していた文字列と選択肢は残るので、`F3` で続けて探せます。1 文字も探さなかった文字列は残りません。
- 検索開始位置は、最初にモードを始めた位置です（[検索開始位置へ戻る](search-start-pos.md)）。
- 正規表現が正しくないときは、箱を出さず、見つからない扱いにします。
- `Backspace` で戻れる手順は、最大でおよそ 256 段です。いっぱいになると、古い 100 段が捨てられます。

## サクラエディタとの違い

- IME の入力途中の文字は、モードの間は画面に出ません。確定した文字だけが探す文字列に足されます。
- ポインタをインクリメンタルサーチの形に変えません。

## 根拠

- サクラのソース: `sakura_core/view/CEditView_Cmdisrch.cpp`（`ISearchEnter`、`ISearchExit`、`ISearchExec`、`ISearchBack`、`ISearchSetStatusMsg`、`TranslateCommand_isearch`）
- MATSU のコード: `crates/matsu-ui/src/search/isearch.rs`（ファイル先頭の説明、`translate`、`isearch_enter`、`isearch_exec`、`isearch_back`）、`crates/matsu-ui/src/ui_commands/search/isearch_next.rs` ほか
- 試験: `crates/matsu-ui/src/search/isearch_tests.rs`
- 既定のキー: `crates/matsu-commands/src/sakura/key_bindings.rs` に、これらの機能の割り当てはない
- 差: `docs/sakura-gaps.md`（Migemo）
