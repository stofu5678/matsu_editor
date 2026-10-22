# matsu-commands

MATSU Editor の名前付きコマンド（[ADR-0008](../../docs/adr/0008-commands-and-keybindings.md)、[ADR-0013](../../docs/adr/0013-module-structure.md)）。サクラエディタの関数コードとキー割り当ての表（`src/sakura/`、生成物）も持つ。

## 作り

| ファイル | 中身 |
|---|---|
| `src/command.rs` | `Command` トレイトと `register_command!`。`Arg`、`CommandError` |
| `src/registry.rs` | 全コマンドの登録簿。ID と、サクラの関数コードで引く。登録の誤りの検査 |
| `src/context/` | `CommandContext`。コマンドが走る間だけ使う借用の束（文書、カーソル、割り付け、パス）と、編集・移動のメソッド |
| `src/layout.rs` | 上下の移動や表示の桁に使う `Layout` トレイトと、折り返しなしの `PlainLayout`（テスト用）。`matsu-layout` ができたら、そちらが実装する。編集のたびに `edited`、Undo / Redo のたびに `reset` で更新される |
| `src/commands/<グループ>/<コマンド>.rs` | コマンド。1 コマンド 1 ファイル |
| `src/testing.rs` | テストの書式（下）と、コマンドを走らせるテスト用の `Editor` |
| `src/sakura/` | サクラの関数コード・キー割り当て・メニュー（`cargo xtask sakura-tables` で生成） |

カーソルと選択（`Cursor`）、Undo（`History`）、文書と Undo をまとめた `DocumentModel` は `matsu-core` にある。

## コマンドの足し方

1. `src/commands/<グループ>/<id>.rs` を作る（下のひな形）。グループはサクラの分類に合わせる（`edit`、`cursor`、`select` など）。
2. グループの `mod.rs` に `pub mod <id>;` を 1 行足す（アルファベット順）。
3. 同じファイルの `tests` にテストを書く。
4. `cargo test -p matsu-commands` を通す。登録簿の検査が、ID の重複、サクラの関数名の誤り、同じ関数を 2 つのコマンドが実装していること、を見つける。

ほかに一覧はない。`register_command!` がファイルの中で登録する（`inventory`）。

ひな形:

```rust
//! `F_DELETE_BACK`: deletes the selection, or the char before the caret.
//!
//! Sakura: `CViewCommander::Command_DELETE_BACK` in `sakura_core/cmd/CViewCommander_Edit.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct DeleteBack;

impl Command for DeleteBack {
    const ID: &'static str = "delete_back";      // 読みやすい snake_case。サクラの名前そのままではない
    const SAKURA: &'static str = "F_DELETE_BACK"; // sakura::FUNCTIONS にある名前

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        // 編集と移動は cx のメソッドを通す。cx.doc() は読むだけ
        ...
        Ok(())
    }
}

register_command!(DeleteBack);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn deletes_the_char_before_the_caret() {
        check("delete_back", "ab|cd", "a|cd");
    }
}
```

決まり:

- ID は小文字の `snake_case`。サクラの名前に近い読みやすい形にする（`F_FILESAVE` は `file_save`）。生成した表の `id` は機械的な名前なので、使わない。
- ファイルの先頭のコメントに、根拠にしたサクラのソース（`sakura_core/cmd/CViewCommander_*.cpp` の関数）を書く。サクラと違う動きにしたときは、違いとその理由を書く。
- 共有する処理は `CommandContext` のメソッドにする（`move_left`、`delete_range` など）。`+_sel` のような、選択を伸ばす版は、同じメソッドを `extend = true` で呼ぶ。
- 文書は `cx.doc()` で範囲を指定して読む。全文を 1 つの文字列にしない。
- できないこと（サクラの `ErrorBeep()`）は、エラーにしないで `cx.beep()` を呼んで `Ok(())` を返す。`Err` は、引数の誤りや読み込みの失敗に使う。
- 1 回の実行（`Registration::run`、`Registry::run`）が 1 つの Undo の単位になる。コマンドの中で Undo の登録を考える必要はない。マクロのように、複数のコマンドを 1 つの Undo にしたいときは、`cx.begin_block()` と `cx.end_block()` で囲む。

## テストの書式

期待する結果を、カーソルと選択の印を入れたテキストで書く。

| 印 | 意味 |
|---|---|
| `\|` | キャレット。必ず 1 つ |
| `[` | 選択の起点（動かない側）。なくてもよい。あれば 1 つ |

```rust
check("delete_back", "ab|cd", "a|cd");          // キャレットだけ
check("cursor_left_sel", "ab|cd", "a|b[cd");    // 左へ選択を伸ばす。起点は `[`、キャレットは `|`
check("cursor_right", "a|bc[d", "abc|d");       // 後ろ向きの選択（キャレットが起点より前）
check("delete", "ab|\r\ncd", "ab|cd");          // CRLF
check("delete", "ab\n|", "ab\n|");              // 最後の改行の後の空行
check("delete", "|", "|");                      // 空の文書
```

- `a[bc|d` は `bc` を選択していて、キャレットが後ろにある（右へ伸ばした）。`a|bc[d` は同じ範囲を、キャレットが前にある形（左へ伸ばした）。向きも比べる。
- 改行は本物の文字で書く（`"\r\n"`、`"\n"`、`"\r"`）。印は CRLF の途中には置けない。
- 文書の終わりは、文字列の終わり。`"ab|"` は最後の行の末尾、`"ab\n|"` は最後の改行の後の空行（文書の終わり）。
- 文書の中に `|` と `[` は書けない。必要なテストは `Editor::from_parts(text, cursor)` で作る。
- 複数の手順や、`Changes`（何が変わったか）の確認は `Editor` を使う。

```rust
let mut e = Editor::new("ab|cd");
e.run("delete").run("undo").assert_marked("ab|cd");
assert!(e.changes().text_changed());
```

- コマンドは登録簿を通して ID で走らせるので、テストは登録も確かめる。
- Enter の改行コードは、文書の最初の改行（なければ CRLF）。`Editor::with_new_line(Eol::Lf)` で決める。
- 失敗すると、期待と実際の両方を、改行を `\r\n` と見える形にして表示する。

## 割り付けの更新

`CommandContext` は `&mut dyn Layout` を持つ。バッファを編集した直後（`replace` の中）に `Layout::edited(doc, change)` を呼び、Undo / Redo のあとに `Layout::reset(doc)` を呼ぶ。コマンドの途中（編集してから上下に動く、マクロが続けて何度も呼ぶ）でも、割り付けは今の文書と合っている。`PlainLayout` のように状態を持たない実装は、両方とも何もしない。

## Undo の方式

サクラの `COpeBuf`・`COpeBlk` と同じく、1 コマンド（マクロなら 1 回の実行）が 1 つの Undo の単位。続けて打った文字もまとめない（サクラは 1 文字ごと）。

内部は、操作の記録ではなく、バッファの版（Undo の前と後の `Document` とカーソル）を持つ方式にした。バッファが永続構造（複製は 1 つのポインタのコピー）なので、Undo と Redo は版の差し替えで済み、操作を逆に当てる処理が要らない。理由と比較は `matsu-core` の `undo.rs` の先頭に書いた。

## まだないもの

キーの処理とキー割り当て、メニュー、クリップボード、検索、ファイルのコマンド、マクロ、IME、画面全体の割り付け（折り返し）、上書きモード。実行ファイルは `use matsu_commands as _;` で必ずこのクレートを参照する（参照がないと `inventory` の登録が黙って空になりうる）。
