# タイプ別設定: ウィンドウ

新しい文書の文字コードと改行コード、IME、ウィンドウのアイコン、行番号の見せ方、背景の画像を決めるページです。キーは、`matsu.ini` の `[Types(N)]` にあります。

## デフォルトの文字コード

| 項目 | キー | 動き |
|---|---|---|
| 改行コード | `eDefaultEoltype` | 新しい文書の改行コード。「CR+LF」（1）、「LF (UNIX)」（2）、「CR (Mac)」（3）、「NEL」（4）、「LS」（5）、「PS」（6） |
| BOM | `bDefaultBom` | 新しい文書に BOM を付ける。BOM を持てる文字コードのときだけ使える |
| 文字コード | `eDefaultCodetype` | 新しい文書の文字コード。「SJIS」、「EUC-JP」、「Latin1」、「UTF-16」、「UTF-16BE」、「UTF-8」、「CESU-8」 |
| C&P | | オンにすると、下の「コードページ」の一覧が使えるようになる。オンにしたあと、オフには戻せない |
| コードページ | `eDefaultCodetype` | 「C&P」をオンにしたとき、コードページの番号で文字コードを選ぶ |
| 自動判別時にCESU-8 を優先する | `bPriorCesu8` | 文字コードの自動判別で、UTF-8 と CESU-8 のどちらとも読めるファイルを、CESU-8 として開く |

- 「文字コード」を変えると、「BOM」は、その文字コードの既定（付ける、付けない）に変わります。
- この設定が使われるのは、新しく作る文書と、文字コードや改行コードを自動判別しきれないファイルです。設定ファイルに値がないときは、同じ種類の設定が優先され、それもなければ「基本」の設定を使い、それもなければ MATSU Editor の既定（UTF-8、BOM なし、改行コードは OS の標準）になります。
- OK で保存するとき、値を変えなかったキーは、設定ファイルにないままです（改行コードが OS に従うままにするため）。
- コードページの一覧には、MATSU Editor が変換表を持つものだけが出ます。437・850 などの OEM のコードページや、ISO-2022-KR など変換表のないものは選べません。Windows の変換機能を使わず、どの OS でも同じ結果にするためです。

## 起動時のIME (日本語入力変換)

| 項目 | キー | 動き |
|---|---|---|
| ON/OFF状態 | `nImeState` の下位 2 ビット | 「そのまま」（0）、「常にON」（1）、「常にOFF」（2） |
| 入力モード | `nImeState` の上位のビット | 「標準設定」（0）、「全角」（1）、「全角ひらがな」（2）、「全角カタカナ」（3）、「無変換」（4） |

- 文書を開いたとき、そのウィンドウが使われている状態なら、IME をこの設定にします。1 つの文書につき 1 回です。
- この 2 つは Windows だけで動きます（IMM32 が、アプリから IME を操作できる唯一の OS のため）。macOS と Linux では灰色で、理由が横に出ます。

## ウィンドウ

| 項目 | キー | 動き |
|---|---|---|
| 文書アイコンを使う | `bUseDocumentIcon` | ウィンドウのアイコンを、開いているファイルの種類のアイコンにする |

Windows だけで動きます。macOS と Linux では灰色です。タブのアイコンは、共通設定「タブバー」の「アイコン表示」で決まります。

## 行番号の表示

| 項目 | キー | 動き |
|---|---|---|
| 折り返し単位 | `nInts` の 8 番目 | 折り返した行ごとに、行番号を付ける（`0`） |
| 改行単位 | `nInts` の 8 番目 | 改行で区切られた 1 行ごとに、行番号を付ける（`1`） |
| 桁数 | `nLineNumWidth` | 行番号の欄の、最小の桁数。2〜11 |

## 行番号区切り

行番号と本文の間に入れる印です。

| 項目 | キー | 動き |
|---|---|---|
| なし | `nInts` の 9 番目（0） | 何も入れない |
| 縦線 | `nInts` の 9 番目（1） | 縦線を引く |
| 任意 | `nInts` の 9 番目（2） と `cLineTermChar` | 右の欄に書いた半角の 1 文字を入れる。「任意」のときだけ書ける |

## 背景画像

| 項目 | キー | 動き |
|---|---|---|
| ファイル | `bgImgPath` | 画像のファイル。`*.bmp`、`*.jpg`、`*.jpeg`、`*.png`。相対パスは、設定ファイルのある場所、次にプログラムのある場所から探す |
| 表示位置 | `bgImgPos` | 「左上」（0）、「右上」（1）、「左下」（2）、「右下」（3）、「中央」（4）、「中央上」（5）、「中央下」（6）、「中央左」（7）、「中央右」（8） |
| Offset X、Offset Y | `bgImgPosOffsetX`、`bgImgPosOffsetY` | 画像を右・下へずらす量（ピクセル）。-99999〜99999 |
| Repeat X、Repeat Y | `bgImgRepeartX`、`bgImgRepeartY` | 画像を左右・上下に繰り返す（キーの綴りは、サクラエディタと同じ `Repeart`） |
| Scroll X、Scroll Y | `bgImgScrollX`、`bgImgScrollY` | 本文が横・縦にスクロールするとき、画像も一緒に動かす |
| 透明度 | `bgImgOpacity` | 0〜255。0 が「画像が見える」で、255 が「画像が見えない」。ファイルには、不透明度（255 から引いた値）で保存する |

背景画像は、MATSU Editor の本文の描画が描きます。

## サクラエディタとの違い

- IME の 2 つと文書アイコンは、Windows だけで動きます。ほかの OS では灰色にして、理由を画面に出します（OS がアプリからの操作を許さないため）。
- 「コードページ」の一覧は、Windows にインストールされているものではなく、MATSU Editor が変換表を持つものだけです。
- 「背景画像」を選ぶときの、ファイル選択のダイアログの題名は、MATSU Editor が付けたものです（サクラエディタには、ありません）。

## 根拠

- サクラのソース: `sakura_core/typeprop/CPropTypesWindow.cpp`、`IDD_PROP_WINDOW`（`sakura_core/sakura_rc.rc`）、`sakura_core/charset/charset.cpp`（`CCodeTypeName`）、`sakura_core/charset/CCodePage.cpp`、`sakura_core/env/CShareData_IO.cpp`
- MATSU のコード: `crates/matsu-ui/src/type_settings/pages/window.rs`、`crates/matsu-ui/src/type_settings/values.rs`、`crates/matsu-config/src/settings/types.rs`（`read_over`、`write`）、`crates/matsu-ui/src/settings/new_document.rs`、`crates/matsu-ui/src/open.rs`、`crates/matsu-ui/src/doc_window.rs`（`sync`）、`crates/matsu-platform/src/ime_mode`、`crates/matsu-ui/src/back_image/`、`crates/matsu-ui/src/view/back_paint.rs`
- 試験: `crates/matsu-ui/src/type_settings/tests.rs`
- `docs/sakura-gaps.md` の「タイプ別設定「ウィンドウ」の灰色」の行
