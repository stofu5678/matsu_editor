# 入力補完

`Complement` と `ComplementGlobal` の 2 つのジャックは、入力補完の候補を足します。

| ジャック | 呼ばれる文書 |
|---|---|
| `ComplementGlobal` | すべての文書のタイプ。この宣言を持つプラグインは、すべて呼ばれます |
| `Complement` | タイプ別設定で、そのプラグインを選んだタイプ |

## 使い方

1. プラグインの `[Plug]` に、`ComplementGlobal=global.js` か `Complement=typed.js` を書きます。`Complement` には、`Complement.Label` で名前を付けます。
2. このジャックは、キー操作で呼ばれるので、プロセスを残すプラグインだけが使えます。`[Wsh]` に `UseCache=1` と書くか、`[Matsu]` に `Resident=1` と書きます（[plugin.def の書き方](plugin-def.md)）。
3. プラグインを有効にします。
4. `Complement` を使うときは、タイプ別設定の「支援」ページの「入力補完機能」で、プラグインの名前を選びます。`ComplementGlobal` は、選ばなくても使われます。
5. 文書で、単語の途中から入力補完のコマンドを動かします（既定のキーは `Ctrl+Space` と `Ctrl+/`）。

ほかの補完の候補元（その文書の単語や、単語ファイルなど）の候補に、プラグインの候補が足されます。

選んだプラグインを使えなくしたとき（無効、削除、変更の検出）は、タイプ別設定の値は消えません。一覧には「（使えないプラグイン N）」と出ます。

## 呼ばれる場面

- 補完のコマンドで、候補の一覧を開くとき、1 回だけ呼ばれます。呼ぶ順番は、先にすべての `ComplementGlobal`（プラグインの一覧の順）、次に、タイプ別設定で選んだ `Complement` です。
- 一覧が開いたあと、文字を打って候補を絞り込むときは、プラグインを呼び直しません。集めてある候補を、新しい単語の先頭で絞ります。
- MATSU Editor には、自動で開く補完がありません。プラグインを呼ぶのは、補完のコマンドだけです。

## ハンドラの書き方

`Complement` のオブジェクトを使います。

| 呼び出し | 内容 |
|---|---|
| `Complement.GetCurrentWord()` | キャレットの前の単語 |
| `Complement.GetOption()` | 補完のオプション。`1` のビットが立っていれば、英字の大文字と小文字を区別しません。`2` のビットが立っていれば、文書の中の単語も候補元です |
| `Complement.AddList(word)` | 候補を 1 つ足します。足したあとの候補の数を返します。その候補が一覧にすでにあるときは `-1` です。空の文字列はエラーです |

```js
// Adds two candidates that begin with the word before the caret.
var word = Complement.GetCurrentWord();
Complement.AddList(word + '_hello');
Complement.AddList(word + '_world');
```

候補は、1 回の一覧につき 10,000 個までです。超えると、`AddList` は `-1` を返します。

## 一覧の出方

- 一覧は、ほかの候補元の候補で開き、プラグインの候補は、届き次第足されます。
- ほかの候補元の候補が 2 つ未満のときは、プラグインが終わるまで何も出しません。候補が 1 つだけなら、すぐに挿入する決まりと、候補がなければ何もしない決まりを、守るためです。
- プラグインが動いている間に、文書かキャレットが動いたときは、プラグインが足した候補は捨てます。
- 同じウィンドウで、ほかのプラグインかマクロが動いているとき、またはそのプラグインのプロセスが別のウィンドウの呼び出しで忙しいときは、そのプラグインは飛ばします。ほかの候補だけで、一覧を作ります。

## 制限

制限の時間は、1 秒です。超えるとプロセスを終わらせます。制限の前に足した候補は残ります。失敗として数えます（3 回続くと止めます。[時間の制限と失敗](plugin-limits.md)）。

## サクラエディタとの違い

- サクラエディタは、補完の一覧が開いている間、文字を打つたびにプラグインを呼び直します。MATSU Editor は、一覧を開くときの 1 回だけです。
- プロセスを残すプラグインだけが使えます。
- 1 秒の制限があります。

## 根拠

- MATSU のコード: `crates/matsu-ui/src/plugin_complement.rs`（冒頭の説明、`Collector`、`MAX_ADDED`、`OPTION_IGNORE_CASE`、`OPTION_BY_FILE`）。`Jack::needs_resident` は `crates/matsu-config/src/plugins/def.rs`
- タイプ別設定: `crates/matsu-ui/src/type_settings/pages/support.rs` の `hokan_types`
- 補完のキー: `crates/matsu-commands/src/sakura/key_bindings.rs`（`F_HOKAN` は `Ctrl+Space` と `Ctrl+/`）
- 試験: `crates/matsu-ui/src/plugin_complement/tests.rs`
- サンプル: `crates/matsu-plugin/samples/plugins/HelloComplement/`
- サクラのソース: `sakura_core/env/CHokanMgr.cpp` の `Search`、`sakura_core/plugin/CComplementIfObj.h`
