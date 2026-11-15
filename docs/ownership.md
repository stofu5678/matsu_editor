# 所有権の決まり

Rust の「だれが値を持つか」「状態をどう持つか」「長く生きるものをだれが終わらせるか」の決まり。決めた理由は [ADR-0033](adr/0033-ownership.md)。Rust のコードを変える PR には、この文書を読んだレビュー（Codex）が 1 回かかる。

- 数（clone の数など）は目標にしない。見るのは、設計の形
- 例は、2026-11-15 の main のコード。コードが変わって例が古くなったら、その PR で直す
- 守れない事情があるときは、コードにコメントで理由を書く。レビューは、理由が書いてあれば、それを読んで判断する

## 1. 状態は enum で持つ。文字列にするのは、保存するときと、見せるときだけ

状態を文字列で持つと、取りうる値がコンパイラに分からず、`match` の取りこぼしも、別の意味の混ざりも見つからない。文字列にしてよいのは、`matsu.ini`・`plugins.ini` に書くときと、画面に出すときだけ。読み込んだらすぐ enum にし、書くときに文字列にする。

- よい例: `crates/matsu-config/src/plugins/consent.rs` の `Consent`（`Off`・`On`・`Changed(Changes)`）。変更の中身が `Changed` の中にあり、`On` と取り違えられない
- 直した例（#449、#460 で直した）: `crates/matsu-ui/src/common_settings/pages/plugin/rows.rs` の行は、同意の状態を `Agreement`（`Off`・`On`・`Changed`）で扱い、「失敗で停止」は持たない。停止は `plugin_call::is_stopped` が持ち、`standing(row)` が読むたびに `Standing::Stopped` を作る。保存前の同意（manifest）は行の列でなく、ページの別の値 `Agreed` が持つ。画面の文言は、描画するとき（`shown_state`）にだけ作る。表を保存する `Vec<String>` の行は、`Agreement` を文字にして置く入れ物にとどまる。表示を直す処理を足すたびに、同意や「失敗で停止」が消える不具合が 3 回続いた、元の形はこの修正で消えた
- 状態は、互いに関係のないものを 1 つにまとめない。「画面に出す状態」「保存前の同意」「実行時の停止」は、持ち主が別

## 2. 1 つの値の持ち主は 1 つ

同じ値を 2 か所で持つと、いつか食い違う。ほかで覚えたくなったら、どちらが正しいかを決め、もう一方はそのつど読む。どうしても覚える（キャッシュする）ときは、いつ捨てるか（何が起きたら古いとみなすか）をコメントに書く。

- 悪い例（#447）: `crates/matsu-ui/src/menu_bar/popup_keys.rs` の `Entry::pos` は、gpui-component の `PopupMenu` が持つ選択の位置を、MATSU の側で追いかけて覚えている（`PopupMenu` が選択を教えないため）。マウスが動くと選択が変わるので、`moves` を数えて「もう分からない」にしている。左右キーで出入りすると、この 2 つが食い違った。相手が教えてくれないときは、覚えずに作り直す（今の `rebuild`）か、相手に選択を教えるよう直す（上流への報告は CLAUDE.md の決まりに従う）ほうがよい
- 設定の値を別の構造体に写すとき（`crates/matsu-ui/src/settings/view_settings.rs` の `input: InputOptions { ... }` は、設定の項目を 1 つずつ写している）は、写しが古くなる時点を決める。設定を持つ型をそのまま借りられないか、まず考える
- 画面の部品が値を持つとき（入力欄の `Entity<InputState>`）は、その値が持ち主。別のフィールドに写して、同期しなくてはならない形にしない

## 3. 部品どうしの持ち合い

- GPUI の部品（`View`・状態）は `Entity<T>` で持つ。互いを持つときや、相手より長く生きたくないときは `WeakEntity<T>` にする
  - 例: `crates/matsu-ui/src/document.rs` の `views: Vec<gpui::WeakEntity<EditorView>>`。文書は、見ているビューが閉じても、ビューを生かさない
  - 例: `crates/matsu-ui/src/file_ops/tag_jump.rs` の `Vec<(WeakEntity<EditorView>, Pos)>`
- `Rc<RefCell<T>>` は、GPUI のコールバック（`'static` なクロージャ）が、`Entity` を通さずに同じ値を触る必要があるときだけ使う。例: `crates/matsu-ui/src/macros/plugin_call.rs` の `outline: Option<Rc<RefCell<Collected>>>`。まず `Entity` と `update` で足りないか考える
- `Cell`・`RefCell` を、描画の最中に値を書き換えるために使うときは、なぜ `&mut self` ではだめかをコメントに書く。描画中に書く値でも、描画する側（`render`）が `&mut` を持つなら、`Entity::update` で書けば `Cell` は要らない（例: `crates/matsu-ui/src/outline/pane.rs` の `shown_place`・`float_shown`・`drawn_side` は、`Workspace::render_text_area` が `pane.update` で書く）
- `Arc` は、スレッドをまたぐ共有と、変わらない大きなデータの共有にだけ使う。例: `crates/matsu-core/src/buffer/storage.rs` の `MemorySource(Arc<[u8]>)`（写すと中身が共有される）、`crates/matsu-core/src/tags_make.rs` の出力を集める `Arc<Mutex<Vec<u8>>>`（読むスレッドと共有）。UI のスレッドだけで使う値に `Arc` を使わない（`Rc` で足りる）
- グローバル（`Global`・`thread_local!`）は、アプリに 1 つのものだけにする。値の持ち主を隠すために使わない

## 4. 関数の境界: 借りるか、受け取るか

呼ぶ側が何を持っているかと、呼ばれる側が値をとっておくかで決める。

| 呼ばれる側 | 引数 |
|---|---|
| 読むだけ | `&str`・`&[T]`・`&T`（`&String`・`&Vec<T>`・`&Option<T>`・`&Box<T>` にしない） |
| とっておく（構造体に入れる、スレッドへ渡す） | `String`・`Vec<T>`・`T`。呼ぶ側が持っているなら、渡す（`clone` を呼ぶ側でなく、持っていない側に任せない） |
| 値によって、読むだけのときと、とっておくときがある | `Cow<'_, str>`・`impl Into<String>`・`impl Into<Arc<[u8]>>` など |
| 小さく `Copy` できる値（数・小さな enum） | 値で受け取る（`&u32` にしない。`trivially_copy_pass_by_ref`） |

- 例: `crates/matsu-core/src/file/source.rs` の `Source::read` は `io::Result<Cow<'_, [u8]>>` を返す。メモリにあるものは借りて返し、ファイルは読んで持って返す
- 例: `MemorySource::new(bytes: impl Into<Arc<[u8]>>)`。`Vec<u8>` も `Arc<[u8]>` も受けられる
- 戻り値も同じ。持ち物の一部を見せるだけなら借りて返す。1 行ごとに呼ばれる関数が、呼ぶたびに新しい `Vec` を作って返さない。例: `crates/matsu-print/src/source.rs` の `LineSource::read_line(&self, index, buf: &mut String)` は、呼ぶ側の `String` を使い回す。`crates/matsu-outline/src/source.rs` の `LineSource::line(&self, index) -> Option<Vec<char>>` は 1 行ごとに `Vec<char>` を作る（「直す候補」）
- Clippy の `needless_pass_by_value`・`ref_option`・`trivially_copy_pass_by_ref` は、ワークスペースで警告にしてある（`Cargo.toml`）

## 5. clone

よい clone（理由を書かなくてよい）:

- 安い持ち手: `Entity`・`WeakEntity`・`Rc`・`Arc`・`SharedString`・`FocusHandle`
- 小さな `Copy` に近いデータ（数、短い enum、`Pos` など）
- 値を作り直して別の持ち主に渡す、本当の複製（保存用の写し、やり直しの記録）

よくない clone:

- 借用チェッカーを通すために、短くできる借用の周りで clone する。借用を先に終わらせる（値を取り出してから使う、`let` で区切る、`take`・`mem::replace` を使う）ほうがよい
- ループの中で、毎回同じ `String`・`Vec` を clone する
- 持ち主が 1 つでよいものを、複製して 2 つにする（2 を見る）
- `.to_owned()`・`.to_string()` で、借りたままで足りる場所に `String` を作る

clone を見つけたときは、「なぜ clone が要るのか」を 1 つ言えるかを見る。言えなければ、借用か持ち主の見直しを探す。

## 6. 長く生きるもの: 持ち主と終わらせ方

外部のプロセス、スレッド、タイマー、購読（`Subscription`）、タスク（`Task`）、一時ファイルは、作った所で「だれが持ち、どう終わるか」を決める。

- 持ち主の `Drop` で終わる形にする。例: `crates/matsu-core/src/buffer/block/scan.rs` の `impl Drop for ScanJob`（走査を止める）、`crates/matsu-core/src/file/save.rs` の `impl Drop for TempFile`（一時ファイルを消す）
- 購読とタスクは、持ち主の構造体のフィールドに入れる。捨てると止まる（GPUI）。例: `PrintSettingDialog` の `_subscriptions: Vec<Subscription>`。`detach()` する場合は、なぜ持ち主なしで生きてよいかを書く
- 外部のプロセスは、キャンセルで `kill` してから `wait` する。例: `crates/matsu-core/src/tags_make.rs`（`child.kill()`、`child.wait()`。読むスレッドは待たない理由をコメントに書いてある）
- 呼び出しの範囲（ハンドラ）は、呼んだ側（ホスト）の持ち物に触れさせない。ハンドラの変数、`return`、`exit` が、ホストの待ち受けのループに届かないようにする。例（#444・#448）: `crates/matsu-plugin/src/boot/boot.rb` の `evaluate` は、ローカル変数を持たない専用のメソッドで実行し、ホストの `message`・`plugin` を書き換えさせず、`return` もハンドラだけで終わらせる。`boot.ps1` は、ハンドラを別のスコープで動かし、`exit` もハンドラだけで終わる
- 外から来たコード（プラグイン、シェルのタブのプログラム）が、エディタの状態を直接触れる形にしない。触らせるのは、コマンドと API を通したものだけ

## レビューの観点

ownership のレビューは、変更された部分だけを、次の順で見る。指摘は、場所と、何が食い違うかと、直し方の案を書く。バグ探しや、整形・名前の指摘は、この観点に含めない。

1. 状態を表す文字列・数・`bool` の組はないか。enum にできないか（1）。文字列は、保存と表示の境にだけあるか
2. 同じ値を 2 か所で持っていないか。持つなら、どちらが正しいか、いつ古くなるかが分かるか（2）。外のライブラリが持つ値を、MATSU が写して追いかけていないか
3. 1 つの構造体に、持ち主の違う値が混ざっていないか（画面に出す状態、保存前の値、実行時の状態）
4. `Entity`・`WeakEntity`・`Rc`・`RefCell`・`Arc` は、3 の決まりの場所で使っているか。互いを強く持ち合って、循環していないか。`Cell`・`RefCell` を `&mut` で足りる所に使っていないか
5. 関数の引数と戻り値は、4 の表に合うか。`&String`・`&Vec<T>`・`&Option<T>` はないか。1 行・1 項目ごとに呼ばれる関数が、毎回割り当てていないか
6. clone に、言える理由があるか（5）。ループの中、借用を避けるためだけの clone はないか
7. 外部のプロセス、スレッド、タイマー、購読、タスク、一時ファイルを作っていないか。作るなら、持ち主と終わらせ方（Drop、cancel、kill）が書いてあるか。エラーの道や途中の `return` でも終わるか（6）
8. 呼び出しの範囲（ハンドラ、コールバック）が、呼んだ側の持ち物（変数、ループ、状態）に触れていないか
9. 仕様が分からないと判断できない指摘は、「人の判断が要る」と分けて書く（直さずに、PR で聞く）

## 直す候補

2026-11-15 の時点で、まだ直していないもの。1 PR 1 テーマで、新しい機能の PR と並べて直す。直したら、この一覧から消す。

| もの | 場所 | 直し方 |
|---|---|---|
| アウトラインの `LineSource::line` が `Vec<char>` を返す | `crates/matsu-outline/src/source.rs`、`crates/matsu-ui/src/outline/pane.rs`（実装） | 印刷の `LineSource::read_line`（呼ぶ側のバッファを使い回す）に合わせる。解析が文字の位置で見る部分は、バッファの `Vec<char>` を呼ぶ側が持つ |
| 印刷の設定ダイアログの下書きの二重の状態 | `crates/matsu-ui/src/print_setting/dialog.rs`（`settings`・`original`・`draft`・入力欄の `Entity<InputState>`） | 入力欄が持つ値と `draft` の値のどちらが正しいかを決める。今は `keep_current` で集めて `draft` と `settings` の両方に書く。正しいほうを 1 つにして、もう一方はそのつど読む |
| `InputOptions` を、設定から 1 項目ずつ写している | `crates/matsu-ui/src/settings/view_settings.rs`（`input: InputOptions { ... }`）、`crates/matsu-commands/src/input_options.rs` | `InputOptions` を設定の側の型にして、写しをなくす。または、写しを作る時点（設定が変わったとき）を 1 か所にして、コメントで書く |

#444・#448（プラグインの起動のしくみ）は直してある。#447（サブメニューのキー）は、食い違いを直したが、gpui-component が選択を教えない限り、追いかける形が残る（確かめていない推測: 上流に頼めば、なくせる）。上流に頼むかは、CLAUDE.md の「ほかの OSS への報告」に従って、ユーザーが決める。
