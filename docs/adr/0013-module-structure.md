# ADR-0013: モジュールの構造

- 状態: 決定（細部は印刷と JSON-RPC の試作で詰め、Phase 1 の始めに確定する）
- 日付: 2026-10-10

## 背景

試作（`poc/gpui-textview`）は、ほとんどの処理が `main.rs` 1 つに入っている。本番のコードは Phase 1 で作り直すので、その前に、サクラエディタの機能から逆算した構造を決めておく。

決めるときに重視したことは 2 つ。

- Rust の作法に沿うこと
- MVC や MVVM のような、よく知られた形に寄せること。Ruby on Rails のように「置き場所が決まっていて、置けば使われる」形にして、機能を足しやすくすること

## 提案

### 役割の分け方

MVVM を基本にし、コマンドを MVC の Controller として扱う。

| 役割 | Rails でいうと | クレート | 中身 | 元にするサクラのソース |
|---|---|---|---|---|
| Model | app/models | `ume-core` | バッファ、カーソル・選択、Undo、検索、文字コード | `doc/` `mem/` `cmd/COpe*` `agent/` `charset/` `convert/` |
| Controller | app/controllers | `ume-commands` | 名前付きのコマンド（1 コマンド 1 ファイル） | `cmd/CViewCommander_*.cpp`、`Funccode_x.hsrc` |
| routes | config/routes.rb | `ume-commands` | キー割り当てとメニュー（キーとコマンド名の対応表） | `func/CKeyBind.cpp`、`MainMenu.ini` |
| ViewModel | — | `ume-layout` | 折り返し、表示の桁、タブの展開、行の割り付け、バッファの位置と表示の位置の変換 | `view/CLayout*` |
| View | app/views | `ume-ui` | GPUI での描画、ダイアログ、ウィンドウ | `view/` `dlg/` `window/` `uiparts/` |
| config | config/ | `ume-config` | 設定ファイル（サクラ互換、[ADR-0010](0010-sakura-compatible-settings.md)） | `env/` `prop/` `typeprop/` |
| generator | rails generate | `xtask` | コマンドのひな形とテストを作る（`cargo xtask new-command`） | — |

### クレートと依存の向き

```
ume（実行ファイル。組み立てるだけ）
 └ ume-ui（GPUI）
    ├ ume-commands
    │  ├ ume-layout
    │  │  └ ume-core
    │  └ ume-config
    └ ume-platform（OS ごとの処理）
```

- 依存は上から下への一方向にする。クレートに分けると、依存の循環をコンパイラが禁じるので、このルールが自動で守られる
- `ume-core` と `ume-layout` は、GPUI にも OS にも依存しない。テストは GPUI なしで、すべての OS の CI で回す
- `ume-layout` を UI から分けるのは、画面と印刷（[ADR-0011](0011-printing.md)）で同じ割り付けを使うため。試作では、タブを含む行で表示の位置とバッファの位置の変換が UI の中に埋もれていて、カーソルと選択がずれた。変換を UI の外に出してテストする
- `ume-platform` は、GPUI が持っていない OS の機能（印刷など）だけを受け持つ。OS ごとの処理はトレイトで受け、実装をこのクレートに閉じ込める。`unsafe` が要るのもここだけにする
- 後のフェーズで、次のクレートを足す: `ume-regex`（Onigmo、[ADR-0006](0006-regex-engine.md)）、`ume-macro`（キーマクロ）、`ume-plugin`（JSON-RPC、[ADR-0007](0007-macros-and-plugins.md)）、`ume-print`（ページの割り付け）
- 最初から細かく分けすぎない。クレートは、それを使うフェーズになってから足す。`ume-core` が大きくなったら、文字コードの処理（サクラでは約 80 ファイル）を `ume-encoding` に分ける

### クレートの中

- 機能ごとにフォルダを分ける（`edit/`、`search/`、`clipboard/` など）。種類ごと（`structs/`、`traits/`）には分けない
- コマンドは次の形にする。キー・メニュー・キーマクロ・JSON-RPC（JavaScript、Python など）は、すべて `ID` でコマンドを呼ぶ

```rust
// crates/ume-commands/src/edit/delete_line.rs
// Sakura: F_DELETE_LINE (Funccode_x.hsrc), CViewCommander_Edit.cpp
pub struct DeleteLine;

impl Command for DeleteLine {
    const ID: &'static str = "delete_line";
    fn run(&self, cx: &mut CommandContext) -> Result<()> { /* ... */ }
}
register_command!(DeleteLine); // collected automatically, no central list
```

- イベント（ファイルを開いた、保存の前など。サクラのジャックに相当）も、名前で登録する形にする。プラグインで足すコマンドとイベントも同じ仕組みに乗せる
- コマンドのテストは、カーソル位置付きのテキストで期待値を書く（[roadmap](../roadmap.md) の Phase 5 の準備）

## 残っている確認

- コマンドを自動で集める方法（`inventory` クレートのようにリンク時に集めるか、`build.rs` でファイルを走査して一覧を生成するか）
- `CommandContext` に何を渡すか（文書、ビュー、設定、クリップボード）。JSON-RPC の試作で確かめる
- `ume-layout` を画面と印刷で共有できるか。印刷の試作で確かめる
- GPUI の `Entity` と ViewModel の関係（`ume-ui` で `ume-layout` の状態を `Entity` に包むか）
