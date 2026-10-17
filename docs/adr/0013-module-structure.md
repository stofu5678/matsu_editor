# ADR-0013: モジュールの構造

- 状態: 決定（細部は Phase 1 の始めに確定する）
- 日付: 2026-10-10（2026-10-11 に、構造そのものは問題ないとユーザーが判断した。「残っている確認」のうち 3 つを調べて提案を足した。2026-10-14 に、提案 1〜3 をユーザーが採用した。同じ日に、JSON-RPC の試作の結果で `CommandContext` を詰めた。2026-10-16 に、同梱の C のライブラリを呼ぶクレートでも `unsafe` を使ってよいことにした）

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
    ├ ume-coloring（構文の色分け。ume-core と ume-config に依存する）
    ├ ume-outline（アウトライン解析。ume-core と ume-config に依存する）
    └ ume-platform（OS ごとの処理）
```

- 依存は上から下への一方向にする。クレートに分けると、依存の循環をコンパイラが禁じるので、このルールが自動で守られる
- `ume-core` と `ume-layout` は、GPUI にも OS にも依存しない。テストは GPUI なしで、すべての OS の CI で回す
- `ume-layout` を UI から分けるのは、画面と印刷（[ADR-0011](0011-printing.md)）で同じ割り付けを使うため。試作では、タブを含む行で表示の位置とバッファの位置の変換が UI の中に埋もれていて、カーソルと選択がずれた。変換を UI の外に出してテストする
- `ume-coloring` は、サクラの色分け（コメント・文字列・数値・強調キーワード・URL）を行う。1 行のテキストと行頭の状態から、色の区間と行末の状態を出す。行頭の状態の保持（`ColorStates`）も持つ。GPUI にも OS にも依存しないので、テストは GPUI なしで回り、印刷でも使える。サクラでは `view/colors/` と、状態を持つ `doc/layout/` にあたる。割り付けの `ume-layout` とは別に持つ（色の規則は `ume-config` の型の設定が元で、割り付けは色を知らないため）。2026-10-17 に足した
- `ume-outline` は、サクラのアウトライン解析（C/C++・Java・Python・Perl・Visual Basic・テキスト・HTML/XML の解析器と、ブックマークの一覧）と、ペインに出す木と一覧の組み立てを受け持つ。文書を 1 行ずつ読み（全文を 1 つの文字列にしない）、GPUI にも OS にも依存しないので、テストは GPUI なしで回り、文書の複製でバックグラウンド解析ができる。サクラでは `doc/CDocOutline.cpp`・`types/CType_*.cpp`・`outline/` にあたる。型ごとの規則は ume-config の設定が元なので、`ume-core` には入れず、色分けの `ume-coloring` とも別に持つ。2026-10-17 に足した
- `ume-platform` は、GPUI が持っていない OS の機能（印刷など）だけを受け持つ。OS ごとの処理はトレイトで受け、実装をこのクレートに閉じ込める。OS の API を呼ぶ `unsafe` も、このクレートの中だけにする
- C で書かれたライブラリを同梱して呼ぶクレートは、FFI の `unsafe` をそのクレートの中に閉じ込め、外には安全な関数だけを出す。`ume-regex`（Onigmo、[ADR-0006](0006-regex-engine.md)）がこの形で、同梱の処理系（[ADR-0007](0007-macros-and-plugins.md)）の MicroPython と mruby も、この形のクレート（`ume-micropython-sys`・`ume-mruby-sys`）にする。C のライブラリを足すときは、このようなクレートを 1 つ足して包む。ほかのクレートは `unsafe` を使わない（2026-10-16）
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


## 確認の結果と決定（2026-10-11 に調べ、2026-10-14 に決めた）

「残っている確認」の 4 つのうち 3 つを調べて提案し、2026-10-14 にユーザーが 3 つとも提案どおり採用した。残る 1 つ（`CommandContext`）は、同じ日に JSON-RPC の試作の結果で詰めた（下の 4。Phase 1 の始めに確定する）。

### 1. コマンドを自動で集める方法

決まりは「1 コマンド 1 ファイル。全員が書き換える一覧を作らない」（[CLAUDE.md](../../CLAUDE.md)、[ADR-0008](0008-commands-and-keybindings.md)）。次の 3 つを比べた。

- (a) `inventory`: コマンドのファイルが `register_command!` で登録の 1 行を持つ。登録は `main` の前に、リンカーが集めた情報から行われる
- (b) `linkme`: (a) と同じ形で、1 つの配列（distributed slice）にリンカーが要素を集める。`main` の前の処理はない
- (c) `build.rs`: `src/commands/<グループ>/<名前>.rs` を走査して、`mod` の宣言とコマンドの一覧を生成する。コマンドのファイルには登録の行がない

| 観点 | (a) inventory | (b) linkme | (c) build.rs |
|---|---|---|---|
| 5 つの対象（Windows x64・ARM64、macOS ARM64、Linux x64・ARM64）で動くか | GPUI が使っていて、試作で 3 OS が動いた（下）。Linux ARM64 と Windows ARM64 で試作のテストが通った。x64 と macOS ARM64 の試作は走らせていない | 試作のテストが Linux ARM64 と Windows ARM64 で通った。ほかは走らせていない | リンカーに頼らないので、対象によらない。試作のテストが Linux ARM64 と Windows ARM64 で通った |
| `ume-core` で使えるか（UI に依存しない） | 使える。依存は `inventory` だけ（GPUI は要らない） | 使える。依存は `linkme` だけ | 使える。依存なし |
| 並列開発で競合しないか | コマンドのファイルは独立。ただし、グループのフォルダの `mod.rs` に `pub mod ...;` の 1 行を足す必要がある（Rust の決まり）。同じグループに 2 人が同じ場所へ足すと、git が競合と判断する | (a) と同じ | `mod.rs` が要らないので、ファイルを足すだけで競合しない |
| テストで全コマンドを列挙できるか | できる（`inventory::iter`）。試作で、ID の一覧が期待どおりかを確かめた | できる（配列を回す） | できる（生成した配列） |
| ビルドの時間 | 試作の 3 コマンドでは 0.1 秒ほど（クリーンビルド、Linux） | 同じ条件で 1.6 秒ほど。proc macro（`linkme-impl`）の分で、`syn` を含む | build.rs のコンパイルと実行で 0.4 秒ほど。走査するフォルダが変わるたびに再実行される設定にした（測ってはいない） |
| ライセンス | MIT OR Apache-2.0 | MIT OR Apache-2.0 | 該当なし（依存がない） |
| 成熟度 | 0.3.25。GPUI が使っている | 0.3.37。dtolnay の作 | 標準の仕組み |
| 登録の確かめ | 対応していない対象では、何も登録されず黙って空になる（`inventory` の README）。テストで件数を確かめれば気づける | 同様に、テストで確かめる | ビルドの時点で一覧が決まる |
| 順序 | 不定（テストでは並べ替える） | 不定 | ファイル名順 |

ビルド時間は、3 コマンドの試作を `cargo clean` してから 1 回ずつ測った値で、目安にすぎない。コマンドが数百になったときの値は測っていない。`syn` は GPUI の依存にすでにあるので、本番では (b) の分の差はほぼ出ないと思われる（推測）。

#### GPUI での `inventory` の使い方（gpui-pre 0.3.8）

- GPUI は、アクション（キーやメニューから呼ぶ操作。このエディタのコマンドに近い）の登録に `inventory` 0.3.25 を使っている。`#[action]` や `actions!` が展開するコードに `inventory::submit!` があり（`gpui-pre-macros` の `register_action.rs`）、アクションの型ごとに 1 つの登録が増える
- アプリの起動時に `inventory::iter` で全部を集めて、名前からアクションを引く表（`ActionRegistry`）を作る（`gpui-pre` の `action.rs` の `load_actions`）。同じ名前の登録が 2 つあると、`panic` で止める
- 同じ一覧を、キー割り当ての検査やドキュメントの生成に使う関数も公開している（`generate_list_of_all_registered_actions`）。「全部を列挙してテストや生成に使う」という、このエディタの使い方と同じ
- GPUI は試作（`poc/gpui-textview`）で Windows ARM64・x64、macOS、Linux で動いた。その中で `inventory` も動いたことになる。ただし、試作のアクションが登録の仕組みで正しく見つかった、という確認までは取っていない

#### 試作（`poc/command-registry`）

3 つの方法で、別のファイルに 3 つのコマンド（`delete_line`、`duplicate_line`、`open_file`）を置いた小さなクレートを作った。実行ファイルとテストは、どのコマンドも名前で参照せず、クレートに依存するだけにした（本物の実行ファイルが `ume-commands` を使う形）。

- 3 つの方法とも、全コマンドが列挙され、登録されたコマンドを呼べた。Linux ではデバッグとリリースの両方で、Windows ではデバッグのテストとリリースの実行で確かめた
- Linux（WSL2、aarch64）と Windows 11 ARM64 で、`cargo test` が通った。Linux では `cargo clippy -- -D warnings` もきれい
- (c) のコマンドのファイルには、`mod` の宣言も登録の行もない。(a) と (b) では、`mod.rs` に宣言の 1 行が要る
- 確かめていないこと: クレートに何も参照がない場合にリンカーが登録ごと落とすか、LTO の細かい設定、macOS と x64、コマンドの数が多いとき、rust-analyzer での見え方

#### 決定: (a) `inventory`

- 理由
  - GPUI がすでに同じ用途で使っていて、依存が増えない。GPUI が動く対象では動くと期待できる
  - 登録の 1 行がコマンドのファイルの中にあり、「ファイルを足せばコマンドが増える」という Rails のような形に近い。サクラの ID と、コードの `ID` が同じファイルで見える
- 守ること
  - 実行ファイルから `use ume_commands as _;` のように必ず参照し、テストで「コマンド数が 0 でない」「ID が重複しない」を確かめる（登録が黙って空になる事故を防ぐ）
  - 順序に頼らない。表示順が必要なものは、ID やメニューの定義で並べる
- 弱い点
  - グループの `mod.rs` の 1 行で競合が起きうる。`cargo xtask new-command` が行を足す（アルファベット順）形にしても、同じ位置に足せば競合する。競合しても解消は 1 行で済むが、まったく起きないわけではない
- 代わりの案
  - (c) は競合が起きない点で優れる。ファイルの名前と構造体の名前の対応（`delete_line.rs` なら `DeleteLine`）が暗黙の決まりになり、プラグインや別クレートのコマンドは集められない点が弱い。(a) が対象のどれかで使えないとわかったときの代わりにする
  - (b) は (a) と大差がない。GPUI が使っていない分、(a) を選ぶ理由のほうが強い
  - `automod` のようなクレートで `mod` の宣言も自動にする案は、試していない

競合の少なさ（(c)）と、登録の 1 行がファイルの中にある分かりやすさ（(a)）のどちらを重くみるかをユーザーに聞き、2026-10-14 に (a) に決まった。

### 2. `ume-layout` を画面と印刷で共有できるか

印刷の試作（[印刷の試作の結果](../poc/printing.md)、[ADR-0011](0011-printing.md)）で示せたことと、示せなかったことを分けて書く。

示せたこと

- 割り付け（`poc/gpui-textview/src/print/layout.rs`）は、GPUI にも OS にも依存せず（依存は文字の幅を求める `unicode-width` だけ）、単体テストで確かめられる。文書の行を受け取って、ページごとの「文字列とその位置」を返す
- その結果から、GPUI のプレビューと、Mac・Windows・Linux の印刷の 4 つを作れ、文字の位置が揃った。つまり「割り付けは UI の外に出せる」「UI 側は結果を描くだけにできる」は示せた
- 折り返し、タブ、英文ワードラップは、サクラの画面の割り付け（`sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp`）に合わせて書いた。サクラが画面と印刷で同じ考え方を使うことの裏づけにはなるが、サクラのコードが共有されているかは調べていない

示せなかったこと

- 画面と印刷が同じ折り返しのコードを使うこと。試作の画面は折り返さない。1 行を GPUI で整形して描き、タブは表示のために 4 つの空白に置き換えている（`main.rs`）。画面と印刷は、コードを共有していない
- 単位の違い。印刷は文字を「半角 1 マス・全角 2 マス」の格子に置く（0.1 mm 単位）。画面は GPUI のフォントの幅（ピクセル）で整形する。画面でも格子に置くのか、フォントの幅で折り返すのかは、決めていない
- 画面の折り返しに必要な、バッファの位置と表示の位置の変換（カーソル、選択、クリックの位置）。印刷にはこの変換が要らないので、印刷の試作では試していない

決定（2026-10-14 にユーザーが提案どおり採用した）

- 共有する対象は、ページへの割り付けではなく、その下の「1 行を桁数で折り返し、タブを展開し、桁と文字の位置を対応づける」部分にする。ページへの割り付け（ヘッダー、段組、行番号）は、これまでどおり印刷の側（`ume-print`）に置く
- 共有できるかの最終的な確かめは、Phase 1 の始めに、画面の折り返しを `ume-layout` で作るときに行う。その時点で、共有が難しければ、画面用と印刷用に分けて、共通の部品（文字の幅、タブの展開）だけを `ume-layout` に置く

### 3. GPUI の `Entity` と ViewModel の関係

試作の作り

- `Entity<T>` は、`App` が持つ状態へのハンドル。読むのは `entity.read(cx)`、書き換えるのは `entity.update(cx, ...)`。`T` が `Render` を実装すれば、それが画面の部品（ビュー）になる。状態を変えたら `cx.notify()` で再描画を頼む（GPUI の `docs/contexts.md`）。ほかのエンティティが `observe` や `subscribe`（`EventEmitter` で送るイベント）で変更を受け取れる
- 試作の `Editor`（`main.rs`）は、状態（`lines`、カーソル、選択、スクロール、フォントの大きさ、IME の範囲）と操作と描画を 1 つの構造体に持つ。それを `Entity<Editor>` にしている
- `Workspace` が `Entity<Editor>`、`Entity<MenuBar>`、`Option<Entity<PrintPreview>>` を持つ。印刷プレビューは `Entity<Editor>` のハンドルを持ち、`editor.read(cx).lines` で文書を読んで割り付ける。つまり、エンティティどうしで文書を共有する形は、すでに使っている
- 試作の `Editor` は文書とビューの状態を分けていないので、ADR-0013 の Model・ViewModel の分け方はまだ試していない

決定（2026-10-14 にユーザーが提案どおり採用した）

- `ume-core` と `ume-layout` は GPUI に依存しないので、`Entity` を使えない。普通の構造体にして、`&mut` で渡して操作する
- `ume-ui` が、それらを `Entity` に包む。包みかたは次の 2 層にする
  - `Entity<DocumentModel>`: `ume_core` の文書（バッファ、Undo）を持つ。1 つの文書を複数のビューで共有できるようにするため、ビューの外に置く。サクラの画面分割（1 つの文書を複数の分割画面で見る。サクラのソースでは確かめていない）を作るときに効く。変更したら `cx.emit(...)` でイベントを送る
  - `Entity<EditorView>`: `ume_layout` の状態（折り返しの結果など）、カーソルと選択、スクロール、フォーカスを持ち、`Render` を実装する。`DocumentModel` を `cx.subscribe` して、変更が来たら `ume_layout` をやり直し、`cx.notify()` する
- コマンド（`ume-commands`）は、`Entity` を知らない。`ume-ui` が `entity.update(cx, |model, cx| ...)` の中で、コマンドに `&mut` の文書と状態を渡して呼ぶ。コマンドのテストは、GPUI なしで普通の構造体に対して書ける
- この提案で期待できること: コマンドと割り付けのテストが GPUI なしで回る。弱い点: `ume-ui` に、イベントを受けて `ume-layout` を更新する薄い層が要る。`Entity` の更新中に別の `Entity` を更新すると GPUI が止める（二重の借用）ので、`update` の入れ子の書き方に気をつける必要がある（GPUI の規則であって、試作では起きていない。確認はしていない）
- 確かめていないこと: 上の 2 層で入力（IME、マウス）、スクロール、100 万行の文書が現在の試作と同じ性能になるか。Phase 1 の始めに、小さな縦切りで確かめる

### 4. `CommandContext` に何を渡すか（2026-10-14、JSON-RPC の試作の結果）

ここは、JSON-RPC の試作の結果で決めることにしていた（[試作の結果](../poc/json-rpc.md)）。

試作の作り

- マクロから呼ぶ 14 個の関数（サクラのマクロ API）は、`MacroDocument` というトレイト（`poc/gpui-textview/src/macro_host/api.rs`）に対して 1 回だけ書いた。トレイトの実装は 2 つ。テストとウィンドウなしの実行に使うテキスト（`TextModel`）と、ウィンドウのエディタ（`EditorDoc`。`&mut Editor` と GPUI の `Context` を包む）
- トレイトが持つのは、行の数、行の中身と長さ、カーソル、選択（あるか、選択した文字列）、カーソルの移動（選択を伸ばすかどうか）、上下の移動、すべて選択、挿入（選択があれば置き換える）、文書のパス
- マクロの要求は、UI のスレッドで、エディタのエンティティの `update` の中で 1 つずつ実行した。3. の決定の形と同じ

分かったこと

- 14 個の関数に要ったのは、文書の行、カーソルと選択、移動、挿入、文書のパスだった。クリップボードと設定は要らなかった
- 編集と移動は、エディタ自身のメソッドを通した。そうすると、キーで操作したときと同じく、カーソルが見えるようにスクロールし、再描画される
- 移動と編集では、IME の変換中の範囲を外すようにした。外さないと、変わった後の文字列を指してしまう。読むだけの呼び出しでは残す
- 上下の移動: 試作のエディタは、最後に描画したときの割り付けを使って上下に動く。マクロは描画の間に何度も呼び出すので、マクロ自身が編集した後では、その割り付けが古い。試作のマクロでは、割り付けを使わずに文字単位で動くようにした。本番では、コマンドが使う割り付け（`ume-layout`）が、その時点の文書と合っている必要がある
- GPUI で `cx.emit` したイベントは、いちばん外側の `update` が終わってから届く（gpui-pre 0.3.8 の `app.rs` の `finish_update`）。3. の決定のように、割り付けを `DocumentModel` のイベントでやり直す形だと、1 つのコマンドの中で編集してから上下に動くとき、割り付けがまだ古い

詰めた形（案。Phase 1 の始めに確定する）

- `CommandContext` は、コマンドが 1 回走る間だけ使う借用の束にする。GPUI の型（`Entity`、`Context`）は入れない（3. のとおり）
- 中身は次の 4 つ
  - 文書（`ume_core`）: 行を範囲で読む。挿入と削除（Undo に記録する）
  - カーソルと選択（ビューごと）。上下の移動の目標の桁も含む
  - 割り付け（`ume_layout`）: 上下の移動や表示の桁に使う
  - 文書のパス（ファイルでなければなし）
- 編集と移動は、`CommandContext` のメソッドを通す。メソッドは、編集のたびにその場で割り付けを直し、何が変わったか（文字列、カーソル）を覚えておく
- コマンドが終わったら、`ume-ui` が、変わったものに応じて、IME の変換中の範囲を外し、カーソルが見えるようにスクロールし、`cx.notify()` で再描画を頼む
- 3. の 2 層のうち、`DocumentModel` のイベントで割り付けをやり直す部分は、同じ文書を見るほかのビュー（分割画面）のために使う。コマンドを走らせたビューの割り付けは、`CommandContext` がその場で直す
- クリップボードと設定は、要るコマンド（貼り付け、設定に従う動きなど）を作るときに足す。OS や GPUI に頼るものはトレイトで受け、`ume-commands` が GPUI に依存しないようにする

```rust
// A sketch, to be fixed at the start of Phase 1.
pub struct CommandContext<'a> {
    doc: &'a mut ume_core::Document,    // text and undo
    cursor: &'a mut ume_core::Cursor,   // cursor, selection anchor, goal column (per view)
    layout: &'a mut ume_layout::Layout, // re-laid out on every edit, not on the next paint
    path: Option<&'a Path>,
    changed: Changes,                   // what the command changed, for ume-ui
}
```

- 確かめていないこと: 編集のたびに割り付けを直す方式の速さ（大きい文書、長い行）。マクロの多くの呼び出しを、まとめて 1 回の再描画にするか（試作は呼び出しごとに再描画を頼む）。マクロを 1 回の Undo にまとめるか

## 残っている確認

- 4（`CommandContext`）の形を、Phase 1 の始めに最初のコマンドを作るときに確かめる。編集のたびに割り付けを直す方式の速さも、そのときに確かめる
- 2（`ume-layout` の共有）が決定のとおりにできるかを、Phase 1 の始めに画面の折り返しを作るときに確かめる。3 の 2 層で、入力・スクロール・100 万行の性能が試作と同じになるかも、同じ時期に小さな縦切りで確かめる
