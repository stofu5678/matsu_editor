# コマンド: 変換

メニューの「変換」にある機能の説明です。選んでいる文字列を、その場で別の形に変えます。大文字と小文字、全角と半角、ひらがなとカタカナ、TAB と空白、文字コードの直し方が、ここにあります。

## 共通の動き {#共通の動き}

この章の機能（「変換」の「URLエンコード」などの、MATSU Editor 独自の 6 つも含む）は、次のように動きます。

- **対象**: 選んでいる文字列です。何も選んでいないときは、何もしません（音も鳴りません）。メニューの項目は灰色になります。
- **選択の残り方**: 変えたあと、変えた文字列の全体が選ばれたままになります。キャレットは、その終わりです。選択を逆向きに作っていても、同じです。
- **箱型の選択**: 行ごとに、箱の中の文字列を変えます。変えたあとは、選択が解け、キャレットは箱の左上へ移ります。文字のない行は、そのままです。
- **改行**: 選んだ範囲の改行（CR、LF、CRLF）は、そのまま変換に渡します。
- **元に戻す**: 変換の結果が元と同じときは、文書を変えず、元に戻す操作の 1 手にもなりません（サクラは、同じ文字列で置き換えます）。
- **閲覧モード**: 文書を変える機能は、灰色になります。実行しても、警告音が鳴るだけで、何も変わりません。
- **キーマクロ**: 「Base64デコードして保存」と「uudecodeして保存」を除いて、記録されます。

## 機能の一覧

### 大文字と小文字

| メニュー | 機能名 | マクロ | 既定のキー | ページ |
|---|---|---|---|---|
| 小文字 | `F_TOLOWER` | `ToLower` | `Ctrl+F6`、`Ctrl+Alt+L` | [小文字](to-lower.md) |
| 大文字 | `F_TOUPPER` | `ToUpper` | `Ctrl+F7`、`Ctrl+Alt+Shift+L` | [大文字](to-upper.md) |

### 全角と半角、ひらがなとカタカナ

| メニュー | 機能名 | マクロ | 既定のキー | ページ |
|---|---|---|---|---|
| 全角→半角 | `F_TOHANKAKU` | `ToHankaku` | `Ctrl+F8` | [全角→半角](to-hankaku.md) |
| 半角＋全ひら→全角・カタカナ | `F_TOZENKAKUKATA` | `ToZenKata` | `Ctrl+F9` | [半角＋全ひら→全角・カタカナ](to-zenkaku-kata.md) |
| 半角＋全カタ→全角・ひらがな | `F_TOZENKAKUHIRA` | `ToZenHira` | `Ctrl+F10` | [半角＋全カタ→全角・ひらがな](to-zenkaku-hira.md) |
| 全角英数→半角英数 | `F_TOHANEI` | `ToHanEi` | なし | [全角英数→半角英数](to-hankaku-eisu.md) |
| 半角英数→全角英数 | `F_TOZENEI` | `ToZenEi` | なし | [半角英数→全角英数](to-zenkaku-eisu.md) |
| 全角カタカナ→半角カタカナ | `F_TOHANKATA` | `ToHankata` | なし | [全角カタカナ→半角カタカナ](to-hankaku-kata.md) |
| 半角カタカナ→全角カタカナ | `F_HANKATATOZENKATA` | `HanKataToZenKata` | `Ctrl+F11` | [半角カタカナ→全角カタカナ](hankata-to-zenkata.md) |
| 半角カタカナ→全角ひらがな | `F_HANKATATOZENHIRA` | `HanKataToZenHira` | `Ctrl+F12` | [半角カタカナ→全角ひらがな](hankata-to-zenhira.md) |

### TAB と空白

| メニュー | 機能名 | マクロ | 既定のキー | ページ |
|---|---|---|---|---|
| TAB→空白 | `F_TABTOSPACE` | `TABToSpace` | `Ctrl+Alt+F5` | [TAB→空白](tab-to-space.md) |
| 空白→TAB | `F_SPACETOTAB` | `SPACEToTAB` | `Ctrl+Alt+Shift+F5` | [空白→TAB](space-to-tab.md) |

### URL、HTML、Base64（MATSU Editor だけの機能）

サクラエディタにない 6 つの機能です。説明は、「MATSU Editor だけの機能」の章の「エンコードとデコード」にあります。

| メニュー | 機能名 | マクロ |
|---|---|---|
| URLエンコード | `F_URL_ENCODE` | `EncodeUrl` |
| URLデコード | `F_URL_DECODE` | `DecodeUrl` |
| HTMLエスケープ | `F_HTML_ESCAPE` | `EscapeHtml` |
| HTMLエスケープを戻す | `F_HTML_UNESCAPE` | `UnescapeHtml` |
| Base64エンコード | `F_BASE64_ENCODE` | `EncodeBase64` |
| Base64デコード | `F_BASE64_DECODE_TEXT` | `DecodeBase64` |

### 文字コード変換

「変換」の「文字コード変換」に入っています。考え方は、[文字コード変換の考え方](code-basics.md)にあります。

| メニュー | 機能名 | マクロ | 既定のキー | ページ |
|---|---|---|---|---|
| 自動判別→SJISコード変換 | `F_CODECNV_AUTO2SJIS` | `AutoToSJIS` | `Alt+F12` | [自動判別→SJISコード変換](code-auto.md) |
| E-Mail(JIS→SJIS)コード変換 | `F_CODECNV_EMAIL` | `JIStoSJIS` | `Alt+F11` | [E-Mail(JIS→SJIS)コード変換](code-email.md) |
| EUC→SJISコード変換 | `F_CODECNV_EUC2SJIS` | `EUCtoSJIS` | `Alt+F10` | [EUC→SJISコード変換](code-euc.md) |
| UTF-16→SJISコード変換 | `F_CODECNV_UNICODE2SJIS` | `CodeCnvUNICODEtoSJIS` | `Alt+F9` | [UTF-16→SJISコード変換](code-utf16.md) |
| UTF-16BE→SJISコード変換 | `F_CODECNV_UNICODEBE2SJIS` | `CodeCnvUNICODEBEtoSJIS` | なし | [UTF-16BE→SJISコード変換](code-utf16be.md) |
| UTF-8→SJISコード変換 | `F_CODECNV_UTF82SJIS` | `UTF8toSJIS` | `Alt+F8` | [UTF-8→SJISコード変換](code-utf8.md) |
| UTF-7→SJISコード変換 | `F_CODECNV_UTF72SJIS` | `UTF7toSJIS` | `Alt+F7` | [UTF-7→SJISコード変換](code-utf7.md) |
| SJIS→JISコード変換 | `F_CODECNV_SJIS2JIS` | `SJIStoJIS` | `Alt+Shift+F11` | [SJIS→JISコード変換](code-sjis-jis.md) |
| SJIS→EUCコード変換 | `F_CODECNV_SJIS2EUC` | `SJIStoEUC` | `Alt+Shift+F10` | [SJIS→EUCコード変換](code-sjis-euc.md) |
| SJIS→UTF-8コード変換 | `F_CODECNV_SJIS2UTF8` | `SJIStoUTF8` | `Alt+Shift+F8` | [SJIS→UTF-8コード変換](code-sjis-utf8.md) |
| SJIS→UTF-7コード変換 | `F_CODECNV_SJIS2UTF7` | `SJIStoUTF7` | `Alt+Shift+F7` | [SJIS→UTF-7コード変換](code-sjis-utf7.md) |
| Base64デコードして保存 | `F_BASE64DECODE` | `Base64Decode` | `Alt+F6` | [Base64デコードして保存](base64-decode-save.md) |
| uudecodeして保存 | `F_UUDECODE` | `Uudecode` | `Alt+F5` | [uudecodeして保存](uudecode-save.md) |

## サクラエディタとの違い

- 結果が元と同じ変換は、文書を変えず、元に戻す操作にもなりません。
- 自動判別と、メールの見出しの読み解きは、MATSU Editor の実装です（[自動判別→SJISコード変換](code-auto.md)）。
- 全角の文字の幅（TAB と空白の変換が使う）は、文字の種類から決めた近似です。
- 「変換」の「URLエンコード」から「Base64デコード」までの 6 つは、サクラエディタにありません。

## 根拠

- 機能と順序: `crates/matsu-commands/src/sakura/main_menu.rs` の「変換」の節、`functions.rs`、`macro_functions.rs`、`key_bindings.rs`
- 共通の動き: `crates/matsu-commands/src/context/convert.rs`、`context/box_edit.rs` の `convert_box`
- 灰色になる条件: `crates/matsu-ui/src/menu_state.rs`（`text_selected`、`MODIFICATION_FORBIDDEN`）
- サクラのソース: `sakura_core/cmd/CViewCommander_Convert.cpp`、`sakura_core/view/CEditView.cpp` の `ConvSelectedArea`
- 試験: `crates/matsu-commands/src/commands/convert/` の各ファイル
