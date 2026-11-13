# マクロのサンプル

動くマクロのサンプルを、言語ごとに載せます。どれも同じ仕事をします。

1. 文書の先頭に題の行を足す
2. 行数と、1 行ずつの内容を出力する
3. 3 行目を選んで、内容を出力し、別の文字に置き換える
4. 間違った引数で関数を呼んで、エラーを受ける
5. 文書の終わりに 1 行足し、先頭に戻る

ここにあるコードは、リポジトリの `crates/matsu-plugin/samples/` にあるファイルそのものです。このページは、それを読み込んで載せているので、写しではありません。コードの中のコメントは英語です。

## 動かし方

1. サンプルを、拡張子を変えずに、自分のフォルダーに保存する
2. 試す文書を開く。次の 3 行の文書が、出力を比べやすいです

```
りんご
みかん😀
ぶどう
```

3. メニューの「ツール」の「名前を指定してマクロ実行...」で、保存したファイルを選ぶ

出力パネルに、スクリプトの出力が出ます。

## 結果

上の 3 行の文書で動かすと、文書は次のようになります（コードを読んで書いた結果です）。

```
# サンプルマクロ 😀
りんご
みかん😀 (置き換えました)
ぶどう
-- マクロの終わり --
```

出力パネルには、次の内容が出ます。タイトルのない文書なので、ファイル名は `(無題)` です。

```
行数: 4 ファイル: (無題)
1: # サンプルマクロ 😀
2: りんご
3: みかん😀
4: ぶどう
選択: みかん😀 / IsTextSelected = 1
エラー: -32602 ...
```

- 出力の細かい書式（区切りの空白など）は、言語の `print` の違いで少し変わります
- 最後の行の `...` は、エラーの理由です。言語によって文面が違います。`-32602` は「引数が不正」を表す番号です
- 試験は、文書の先頭が `# サンプルマクロ` であること、`みかん😀` が残っていること、何かが出力されることを確かめています

## 読むときの要点

- 行番号と桁は 1 から数えます。`MoveCursor(3, 1, 0)` は 3 行目の先頭に動き、`MoveCursor(3, 1000, 1)` は、選択しながら 3 行目の行末（桁が大きすぎるので、行の終わり）まで動きます
- `GetLineStr(行)` は、行の終わりの改行（`\n`）付きで返します。表示するときは、改行を取ります
- `GetLineCount(0)` の引数 `0` は、使われない値です。必ず 0 を渡します
- `InsText` は、選択があれば、選択を置き換えます

## Lua

Lua のサンプルファイルは、まだ `samples/` にありません。同じ仕事を、Lua で書くと次のようになります。

```lua
Editor.GoFileTop()
Editor.InsText("# sample macro\n")

local count = Editor.GetLineCount(0)
print("lines:", count)
for i = 1, count do
  print(i .. ": " .. Editor.GetLineStr(i):gsub("\n$", ""))
end

-- Select line 3 and replace it.
Editor.MoveCursor(3, 1, 0)
Editor.MoveCursor(3, 1000, 1)
Editor.InsText("replaced")

-- An error response is an error that pcall can catch.
local ok, err = pcall(Editor.GetLineStr, "not a number")
if not ok then
  print("error:", err.code)
end

Editor.AddTail("-- end --\n")
Editor.Jump(1)
```

このコードは、ここに書いただけで、試験にかけていません。

## Python

- 拡張子: `.py`
- 利用者が入れるもの: Python
- `python3`（Windows は `python`）で動きます。

```python
{{#include ../../../../crates/matsu-plugin/samples/sample.py}}
```

## JavaScript（Node.js）

- 拡張子: `.mjs`
- 利用者が入れるもの: Node.js
- 拡張子は `.mjs` です。ES モジュールとして動きます。

```javascript
{{#include ../../../../crates/matsu-plugin/samples/sample.mjs}}
```

## JavaScript（QuickJS、同梱）

- 拡張子: `.js`
- 利用者が入れるもの: なし
- 何も入れずに動きます。`require` はありません。

```javascript
{{#include ../../../../crates/matsu-plugin/samples/sample.js}}
```

## TypeScript

- 拡張子: `.ts`
- 利用者が入れるもの: Node.js 22.18 以降
- Node.js が型を取り除いて動かします。型の検査はしません。`enum` のように、型を消すだけでは動かない書き方はエラーです。

```typescript
{{#include ../../../../crates/matsu-plugin/samples/sample.ts}}
```

## Ruby

- 拡張子: `.rb`
- 利用者が入れるもの: Ruby

```ruby
{{#include ../../../../crates/matsu-plugin/samples/sample.rb}}
```

## mruby（同梱）

- 拡張子: `.mruby`
- 利用者が入れるもの: なし
- 同梱の小さな Ruby です。

```ruby
{{#include ../../../../crates/matsu-plugin/samples/sample.mruby}}
```

## PHP

- 拡張子: `.php`
- 利用者が入れるもの: PHP
- クラス `Editor` の静的メソッドとして呼びます。

```php
{{#include ../../../../crates/matsu-plugin/samples/sample.php}}
```

## PowerShell

- 拡張子: `.ps1`
- 利用者が入れるもの: PowerShell 7 か Windows PowerShell 5.1
- 変数 `$Editor` を使います。実行ポリシーの対象になりません（マクロはファイルではなく、文字として実行されるため）。

```powershell
{{#include ../../../../crates/matsu-plugin/samples/sample.ps1}}
```

## Perl

- 拡張子: `.pl`
- 利用者が入れるもの: Perl 5.14 以降
- `Editor->名前(...)` の形で呼びます。スクリプトは文字（`use utf8` と同じ）として読み、出力は UTF-8 です。

```perl
{{#include ../../../../crates/matsu-plugin/samples/sample.pl}}
```

## Emacs Lisp

- 拡張子: `.el`
- 利用者が入れるもの: Emacs 30 以降
- `emacs -Q --batch` で動かします。Emacs 自身の `insert` などは文書に作用しません。`editor-` で始まる関数を使います。最上位の式を 1 つずつ読むので、エラーには、その式が始まった行が出ます。

```elisp
{{#include ../../../../crates/matsu-plugin/samples/sample.el}}
```

## AppleScript

- 拡張子: `.applescript`
- 利用者が入れるもの: macOS だけ
- `editorCall("名前", {引数})` で呼びます。`tell` ブロックの中では `my editorCall(...)` と書きます。

```applescript
{{#include ../../../../crates/matsu-plugin/samples/sample.applescript}}
```

## MicroPython（同梱）

- 拡張子: `.upy`
- 利用者が入れるもの: なし
- 同梱の小さな Python です。`json` `math` `struct` `collections` は使えますが、ファイルは扱えません。

```python
{{#include ../../../../crates/matsu-plugin/samples/sample.upy}}
```

## ほかのサンプル

- SQL を動かすサンプル（`sample_db.py`、`sample_db.mjs`、`sample_db.php`）が、同じフォルダーにあります
- プラグインのサンプル（`HelloCommands` ほか）が、`crates/matsu-plugin/samples/plugins/` にあります

## 根拠

- サンプルのファイル: `crates/matsu-plugin/samples/sample.*`
- 動かして確かめる試験: `crates/matsu-plugin/src/tests.rs` の `the_sample_macros_run`（`.mjs` `.ts` `.ps1` `.pl` `.el` `.applescript` `.py` `.rb` `.php`。処理系がない環境では飛ばします）。`.js` は `crates/matsu-script/tests/quickjs.rs`、`.mruby` は `crates/matsu-script/tests/mruby.rs`、`.upy` は `crates/matsu-script/tests/micropython.rs` が確かめます
- 関数の働き（行番号、桁、`GetLineStr` の改行、`InsText` の選択の置き換え）: `crates/matsu-plugin/src/api.rs` の `FUNCS`
- エラーの番号 `-32602`: `crates/matsu-plugin/src/protocol.rs` の `INVALID_PARAMS`
- Lua の `Editor` の作り: `crates/matsu-script/src/lua.rs`
