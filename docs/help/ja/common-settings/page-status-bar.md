# 共通設定: ステータスバー

ステータスバーの文字コードと桁の表示を決めます。キーは `[Statusbar]` セクションにあります（`[Common]` ではありません）。

OK を押すと、すべてのウィンドウのステータスバーが、すぐに描き直されます。

## 表示文字コードの指定

| 項目 | キー | 既定 |
|---|---|---|
| SJIS, Latin1等で文字コード値をUnicodeで表示する | `DispUnicodeInSjis` | オフ |
| JISで文字コード値をUnicodeで表示する | `DispUnicodeInJis` | オフ |
| EUCで文字コード値をUnicodeで表示する | `DispUnicodeInEuc` | オフ |
| UTF-8およびCESU-8をコードポイントで表示する | `DispUtf8Codepoint` | オン |
| サロゲートペアをコードポイントで表示する | `DispSurrogatePairCodepoint` | オン |

キャレットの位置の文字の「文字コード値」を、どの形で出すかを選びます。オンにした項目の文字コードで、Unicode の値（コードポイント）で表示します。

## 表示文字数の指定

| 項目 | キー | 既定 |
|---|---|---|
| 選択の数を、バイトで数えて表示する（高負荷のため非推奨） | `DispSelectCountByByte` | オフ |
| 現在桁をルーラー単位ではなく文字単位で表示する | `DispColByChar` | オフ |

「選択文字数」の項目は、負荷が高いので、おすすめしません。画面の注意書きにも、そう書いてあります。一時的に切り替える機能は [文字カウント方法](menu-count-mode.md) です。

## 根拠

- サクラのソース: `sakura_core/prop/CPropComStatusbar.cpp`、`CShareData_IO::ShareData_IO_Statusbar`
- MATSU のコード: `crates/matsu-ui/src/common_settings/pages/status_bar.rs`、`crates/matsu-ui/src/status_options.rs`
