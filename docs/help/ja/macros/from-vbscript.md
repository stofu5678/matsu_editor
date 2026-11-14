# VBScript から書き換える手引き

サクラエディタには、VBScript（`.vbs`）で書いたマクロを持っている人がいます。MATSU Editor は、VBScript を動かしません。マクロを、MATSU Editor が動かす言語に書き換えて使います。

この手引きは、書き換えの手順と、言葉の対応表です。例は、この手引きのために書いたものです。

## 動かない理由

- VBScript は、Microsoft が使えなくしていく予定の言語です。Windows でしか動かず、いつまで使えるかも決まっていません。MATSU Editor は、新しく作るものを、ほかの OS でも動く言語に絞りました（ADR-0025（`docs/adr/0025-script-languages.md`））
- `.vbs` のファイルを選ぶと、「マクロの拡張子は .mac、.mjs、… のどれかにしてください」というエラーが出ます
- 共通設定「プラグイン」の一覧にある、VBScript で書いたプラグインには、「VBScript は使えません」と出ます

使い慣れた `Editor` の関数は、そのまま使えます。変わるのは、言語の部分です。

## 書き換える手順

1. **言語を選ぶ**。入れるものがなく、すぐ動くのは、Lua・JavaScript（`.js`）・MicroPython（`.upy`）・mruby（`.mruby`）です。
   すでに Python や Node.js を入れているなら、`.py` や `.mjs` も選べます。ファイルを読み書きするなら、外部の言語（Python、Node.js、Ruby、PHP など）を選ぶのが確実です。MicroPython はファイルを扱えません
2. **拡張子を決めて、ファイルを作る**。`Editor` は最初からあるので、`CreateObject` や `require` のような準備は要りません
3. **文を 1 つずつ書き換える**。次の「対応表」を使う
4. **「名前を指定してマクロ実行...」で動かす**。エラーが出たら、出力パネルの行番号を見て直す

## 対応表

### `Editor` の呼び方

| VBScript | 書き換えたあと |
|---|---|
| `Editor.InsText "abc"`（括弧なし） | `Editor.InsText("abc")`（括弧を付ける。Ruby は省けます） |
| `x = Editor.GetLineCount(0)` | `x = Editor.GetLineCount(0)`（Lua、Python、Ruby）、`const x = ...;`（JavaScript） |
| `Editor.MoveCursor 3, 1, 0` | `Editor.MoveCursor(3, 1, 0)` |
| 関数の名前と引数 | 同じ。マクロの関数の表（名前・引数・戻り値）のとおり |

関数の名前は、サクラと同じです。引数の数と型も、サクラの表と同じです。数の引数に数、文字列の引数に文字列を渡します。数の引数に、数でない文字列（`"not a number"` など）を渡すと、`EditorError` になります。サクラは、渡された値を引数の型に変えてから使います（`CMacro::HandleFunction` の `VariantChangeType`）。

### 言葉と書き方

| VBScript | Lua | JavaScript | Python |
|---|---|---|---|
| `Dim a` | `local a` | `let a;` | （宣言なし） |
| `s = "あ" & "い"`（文字をつなぐ） | `s = "あ" .. "い"` | `s = "あ" + "い"` | `s = "あ" + "い"` |
| `If a = 1 Then ... End If` | `if a == 1 then ... end` | `if (a === 1) { ... }` | `if a == 1:` |
| `For i = 1 To n ... Next` | `for i = 1, n do ... end` | `for (let i = 1; i <= n; i++) { ... }` | `for i in range(1, n + 1):` |
| `Do While 条件 ... Loop` | `while 条件 do ... end` | `while (条件) { ... }` | `while 条件:` |
| `Chr(10)`、`vbLf` | `"\n"` | `"\n"` | `"\n"` |
| `Len(s)` | `utf8.len(s)` | `s.length` | `len(s)` |
| `Mid(s, 2, 3)`（2 文字目から 3 文字） | 下の関数 `mid(s, 2, 3)` | `s.substr(1, 3)` | `s[1:4]` |
| `InStr(s, "a")`（見つけた位置。1 から数える。なければ 0） | `s:find("a", 1, true)`（見つからないと `nil`。位置はバイト数） | `s.indexOf("a")`（0 から数える。なければ -1） | `s.find("a")`（0 から数える。なければ -1） |
| `UCase(s)` | `s:upper()` | `s.toUpperCase()` | `s.upper()` |
| `Trim(s)` | （`s:match("^%s*(.-)%s*$")`） | `s.trim()` | `s.strip()` |
| `CStr(n)` | `tostring(n)` | `String(n)` | `str(n)` |
| `WScript.Echo x` | `print(x)` | `console.log(x)` | `print(x)` |
| `On Error Resume Next` | `pcall(...)` | `try { ... } catch (e) { ... }` | `try: ... except EditorError as e:` |

- Lua の文字列は UTF-8 のバイト列です。`#s`、`s:sub`、`s:find` は、日本語の文字を数えず、バイトを数えます。文字で数える `utf8.len` と `utf8.offset` を使ってください。バイトの途中で切った文字列は、`Editor.InsText` が受け付けません
- Lua で `Mid` の代わりをするときは、文字列の終わりを越える場合を考えた関数を使います。`utf8.offset` は、範囲の外を指すと `nil` を返すので、そのまま引き算するとエラーになります。

```lua
local function mid(s, start, len)
  local n = utf8.len(s)
  if start > n then return "" end
  local last = math.min(start + len - 1, n)
  return s:sub(utf8.offset(s, start), utf8.offset(s, last + 1) - 1)
end
```

- VBScript の文字の位置は 1 から数えます。JavaScript と Python は、0 から数えます。Lua は 1 から数えます。`Editor` の関数の行番号と桁は、どの言語でも 1 から数えます
- コメントの `'` は、`--`（Lua）、`//`（JavaScript）、`#`（Python、Ruby）に変えます
- `WScript.Echo` は、MATSU Editor にはありません。出力パネルに出すには、言語の `print` か `console.log` を使います

### `Editor` 以外の仕組み

| VBScript | MATSU Editor では |
|---|---|
| `MsgBox "..."` | 関数 `MessageBox`（文字と、箱の種類を表す数を渡す。押したボタンを数で返す）。ボタンが OK だけの箱は `InfoMsg`、`WarnMsg`、`ErrorMsg` でも出せます |
| `WScript.*` | 提供しません。上の `print` などで代える |
| `CreateObject("Scripting.FileSystemObject")`、`CreateObject("WScript.Shell")` など、Windows の部品 | 提供しません。ファイルは、外部の言語の標準の機能（Python の `open`、Node.js の `fs` など）で扱う |
| `InputBox(メッセージ, 題, 初期値)` | 関数 `InputBox`。引数は、メッセージ、初期値、最大の長さの順で、題の引数はありません。取り消すと空の文字列を返します |

## 書き換えの例

行に番号を付けるマクロです。文書の各行の先頭に `1: `、`2: ` と付けます。

VBScript の元のマクロです。

```vb
' Number the lines.
Dim count, i, text
count = Editor.GetLineCount(0)
For i = 1 To count
  text = text & i & ": " & Editor.GetLineStr(i)
Next
Editor.SelectAll
Editor.InsText text
```

Lua に書き換えたものです。

```lua
-- Number the lines.
local count = Editor.GetLineCount(0)
local text = ""
for i = 1, count do
  text = text .. i .. ": " .. Editor.GetLineStr(i)
end
Editor.SelectAll()
Editor.InsText(text)
```

JavaScript（`.js`）に書き換えたものです。

```javascript
// Number the lines.
const count = Editor.GetLineCount(0);
let text = '';
for (let i = 1; i <= count; i++) {
  text += i + ': ' + Editor.GetLineStr(i);
}
Editor.SelectAll();
Editor.InsText(text);
```

Python（`.py`）に書き換えたものです。

```python
# Number the lines.
count = Editor.GetLineCount(0)
text = ""
for i in range(1, count + 1):
    text += str(i) + ": " + Editor.GetLineStr(i)
Editor.SelectAll()
Editor.InsText(text)
```

`りんご`、`みかん` の 2 行の文書で動かすと、`1: りんご` と `2: みかん` の 2 行になります。

### 書き換えで気をつけること

- `GetLineStr` は、行の終わりの改行付きで返します。サクラは、その文書の改行（CR LF の文書なら `\r\n`）を付けて返します。MATSU Editor は、改行を `\n` にして返します。改行を取って使うときは、`\n` だけを取ってください
- 上の例は、改行も一緒につなぐので、行の区切りは残ります。ただし、CR LF や CR の文書の改行も `\n` にそろいます。最後の行に改行がない文書は、最後の行にも改行は付きません
- 文書が空のとき、`GetLineCount(0)` は 0 です。ループは 1 回も回りません
- 書き換えたマクロは、必ず、試し用の文書で動かして確かめてください。元のマクロと結果が同じか、`Undo`（`Ctrl+Z`）で戻せるかも確かめられます

## 根拠

- VBScript を動かさない決定と理由: ADR-0025（`docs/adr/0025-script-languages.md`）（「VBScript」の節）、ADR-0007（`docs/adr/0007-macros-and-plugins.md`）（「サクラのマクロとの互換」）、ADR-0026（`docs/adr/0026-resident-plugins.md`）（`WScript.*` と `ActiveXObject` を提供しないこと）
- `.vbs` を選んだときのエラー: `crates/matsu-ui/src/macros/mod.rs`、試験は `crates/matsu-ui/src/macros/tests.rs` の `an_unknown_extension_is_refused`
- 「VBScript は使えません」の表示: `crates/matsu-plugin/samples/plugins/HelloVbs/plugin.def` の注記
- サクラの VBScript の実行: `sakura_core/macro/CWSHManager.cpp`（`Editor` オブジェクトを渡す）、`sakura_core/macro/CWSH.cpp`（スクリプトエンジンを作って動かす。`MsgBox` を使えるようにしたコメント）
- 関数の名前・引数・戻り値: `crates/matsu-commands/src/sakura/macro_functions.rs`（`InsText`、`GetLineStr`、`GetLineCount`、`MoveCursor`、`SelectAll`、`MessageBox` の行）
- `GetLineStr` の改行: `crates/matsu-plugin/src/api.rs`（`GetLineStr`）、サクラは `sakura_core/macro/CMacro.cpp` の `F_GETLINESTR`（`GetDocLineStrWithEOL`）
- 各言語の書き方の例: `crates/matsu-plugin/samples/sample.py`、`sample.mjs`、`sample.js`（`Editor` の使い方）、`crates/matsu-script/src/lua.rs`（Lua の `EditorError`）
