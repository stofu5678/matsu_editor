# アウトライン

`Outline` ジャックは、アウトライン解析をプラグインに任せます。解析の結果は、アウトラインのペインに出ます。

## 使い方

1. プラグインの `[Plug]` に、`Outline=outline.js` のように書きます。名前は `Outline.Label` で付けます。
2. プラグインを有効にします。
3. 文書のタイプのタイプ別設定を開きます。「スクリーン」ページの「アウトライン解析方法」で、「標準ルール」の一覧の下のほうに、プラグインの名前（`Outline.Label`）が並びます。使いたいものを選びます。
4. アウトライン解析を開きます。

選べるのは、動かせるプラグインの `Outline` だけです。プラグインを無効にした、外した、変更を検出した、のどれかのときは、一覧から消えます。そのプラグインを選んでいたタイプの値は、消えません。一覧には「（使えないプラグイン N）」と出ます（`N` は保存している値です）。プラグインを有効に戻すと、また使えます。

プラグインが使えない間は、そのタイプのアウトラインは、テキスト用の標準の解析（`MakeTopicList_txt` に当たるもの）になります。

## 呼ばれる場面

ハンドラは、アウトライン解析のたびに 1 回呼ばれます。ペインを開いたとき、読み直したとき、別の文書やタイプに移ったときです。キーを押すたびには呼ばれません。タイプ別設定で選んだ 1 つだけが呼ばれます。

アウトラインを作っている間に、同じウィンドウで、もう 1 回の解析は始めません。

## ハンドラの書き方

ハンドラは、`Editor` で文書を読み、`Outline` のオブジェクトに結果を渡します。

| 呼び出し | 内容 |
|---|---|
| `Outline.AddFuncInfo(line, column, text, info)` | 項目を 1 つ足します。`line` と `column` は 1 から数えます |
| `Outline.AddFuncInfo2(line, column, text, info)` | 同じです。`info` の下位 16 ビットを、木の深さとして使います |
| `Outline.AddFuncInfo3(line, column, text, info, file)` | 項目に、ファイル名を付けます |
| `Outline.AddFuncInfo4(line, column, text, info, file)` | `AddFuncInfo2` と同じ深さの扱いで、ファイル名も付けます |
| `Outline.SetTitle(title)` | ペインの題 |
| `Outline.SetListType(type)` | 表示の形。`100` は木、`101` はタグジャンプの木、`200` はクラスの木、`300` は一覧。ほかの値は一覧になります |
| `Outline.SetLabel(info, text)` | `info` の種類の項目に付ける名前 |

数を渡す引数は、数字の文字列でも受けます。

サンプル（`crates/matsu-plugin/samples/plugins/HelloOutline/outline.js` の要点）です。`#` で始まる行を、`#` の数を深さにした木にします。

```js
// A line "# a" is at depth 0, "## b" at depth 1.
Outline.SetListType(100); // 100: a tree
Outline.SetTitle('Headings');
const count = Editor.GetLineCount(0);
for (let n = 1; n <= count; n++) {
  const line = Editor.GetLineStr(n).replace(/[\r\n]+$/, '');
  const m = /^(#+)\s*(.*)$/.exec(line);
  if (!m) continue;
  Outline.AddFuncInfo2(n, 1, m[2], m[1].length - 1);
}
```

## 失敗したとき

制限の時間は 10 秒です。ハンドラが例外を投げたり、時間を超えたりすると、標準の解析には切り替えません。ペインには、空の一覧と、理由が出ます。

行を範囲ごとに読む大きなファイルには、アウトラインがありません。標準の解析と同じです。

## サクラエディタとの違い

- 失敗したときに、標準の解析へ切り替えません。空の一覧と理由を出します。
- 呼び出しに 10 秒の制限があります。
- 使えないプラグインを選んでいるタイプの値は、消さずに残します。サクラエディタは、一覧の先頭を見せて、OK でその値を書きます。

## 根拠

- MATSU のコード: `crates/matsu-ui/src/plugin_outline.rs`（冒頭の説明、`Collected`、`plugs_of`）、`crates/matsu-ui/src/outline/pane/plugin.rs`、`crates/matsu-outline/src/kind.rs` の `list_type`、`OutlineKind::from_list_type`
- タイプ別設定の選択肢: `crates/matsu-ui/src/type_settings/pages/screen.rs`（「アウトライン解析方法」）。使えないプラグインの扱い: `docs/adr/0026-resident-plugins.md` の「追記（2026-11-03）」
- 試験: `crates/matsu-ui/src/plugin_outline/tests.rs`、`crates/matsu-ui/src/outline/plugin_tests.rs`
- サンプル: `crates/matsu-plugin/samples/plugins/HelloOutline/`
- サクラのソース: `sakura_core/cmd/CViewCommander_Outline.cpp` の `Command_FUNCLIST`、`sakura_core/plugin/COutlineIfObj.h`
