# 次のウィンドウ

次のウィンドウを前面に出します。最後のウィンドウから使うと、最初のウィンドウに戻ります。

| 項目 | 内容 |
|---|---|
| メニュー | ウィンドウ > 次のウィンドウ |
| 機能名 | `F_NEXTWINDOW` |
| 既定のキー | `Ctrl+Tab` |
| マクロ | `NextWindow` |
| キーマクロに記録 | しない |

## 動き

- 順番は、ウィンドウを開いた順です。
- 最小化したウィンドウも、順番に入ります。
- タブモード（タブバーを表示していて、ウィンドウをまとめる設定のとき。共通設定の「タブバー」ページで決めます）では、ウィンドウの中のタブを右へ 1 つ進めます（最後のタブから、最初のタブへ）。
- 文書を持たないウィンドウ（シェルだけのウィンドウ）も、順番に入ります。
- ウィンドウが 1 つだけのときは、そのウィンドウを前面に出し直します。

## サクラエディタとの違い

- サクラは、分割した区画があると、まず次の区画へ移り、区画が尽きてからウィンドウを移ります。MATSU Editor は区画を移らず、すぐ次のウィンドウ（またはタブ）へ移ります。

## 根拠

- サクラのソース: `sakura_core/doc/CEditDoc.cpp` の `HandleCommand`、`sakura_core/_main/CControlTray.cpp` の `ActivePrevWindow`（名前が逆に付いている）
- MATSU のコード: `crates/matsu-ui/src/ui_commands/window/nextwindow.rs`、`crates/matsu-ui/src/window_ops/mod.rs`（`window_after`、`all_handles`）、`crates/matsu-ui/src/tabs/order.rs`（`next_tab`）
- キー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`Tab` の Ctrl）
