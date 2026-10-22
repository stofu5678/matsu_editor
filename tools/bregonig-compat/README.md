# bregonig-compat

サクラエディタが使う正規表現ライブラリ bregonig.dll の実物を Windows で動かして、挙動を測るための道具。測った結果は `bregonig-4.20-x64.json` などに保存してあり、`matsu-regex` のテスト（`crates/matsu-regex/tests/bregonig_compat.rs`）が毎回これと見比べる。テストの実行に DLL は要らない。

bregonig の説明書（`bregonig.html`）が決めていない振る舞い（壊れた `$` の形、同名グループ、`\Q`、`\G` と空一致、サロゲートなど）の期待値を、ソースを見ずに実物から取るために作った（ADR-0006、2026-10-16 のユーザー判断）。

## ライセンスと DLL の扱い

- bregonig のソースコードは「GPL または Artistic License」で、このリポジトリ（MIT OR Apache-2.0）には取り込めない。この道具は bregonig のソースを読まない。DLL を動かして、返ってきた値を記録するだけ
- DLL はリポジトリに入れない。サクラエディタの配布物に同梱されているものを使う（`installer/externals/bregonig/bron420.zip` の `bregonig.dll`（32 ビット）と `x64/bregonig.dll`（64 ビット））。`run.sh` は zip の中からこの 2 つの DLL だけを取り出す。同じ zip にあるソースのアーカイブ（`src.7z`）は開かない
- `Harness.cs` は、サクラエディタの `sakura_core/extmodule/CBregexpDll2.h`・`CBregexpDll2.cpp`・`CBregexp.cpp`（zlib License）の呼び出し手順と構造体を C# に移したもの。ファイルの先頭に、元の著作権表示と zlib License の全文と、改変したことを書いてある

## 測ったもの

| | |
|---|---|
| DLL | bregonig.dll Ver.4.20 with Onigmo 6.2.0（サクラエディタの配布物 `bron420.zip` が同梱するもの） |
| 日付 | 2026-10-16 |
| 環境 | Windows 11 ARM64（x86 は WOW64、x64 はエミュレーションで動かした） |
| 結果 | `bregonig-4.20-x64.json`（基準）、`bregonig-4.20-x86.json` |

x86 と x64 の結果は、`$-{name}[n]` の `n` が範囲外のとき（8 件）だけ違う。x64 は空になり、x86 は別のグループの文字列を返す。範囲外を読んでいるように見える。基準は x64 にしてある。

## 何をしているか

`Harness.cs` は、サクラエディタと同じ順番で Unicode 版の API を呼ぶ。

1. `CBregexp::MakePatternAlternate` と同じ変換（`.` を `[^\r\n]` に、`$` を `(?<![\r\n])(?=\r|$)` に）をかける
2. `CBregexp::MakePatternSub` と同じに、`m<D>パターン<D>km` または `s<D>パターン<D>置換<D>km` を作る。`D` は U+FFFF。大文字小文字を区別しないときは `i`、すべて置換は `g` を付ける
3. ダミーの文字列に `BMatchW`（置換は `BSubstW`）をかけてコンパイルする
4. 本物の行に、パターンなしの `BMatchExW`（置換は `BSubstExW`）をかける

結果は、ケースごとに戻り値、エラーメッセージ、グループの位置（UTF-16 単位）、置換後の文字列を JSON に書く。ケースはそれぞれ別のプロセスで動かすので、DLL が落ちても残りは測れる。

## 動かし方

WSL2 から Windows の C# コンパイラ（`csc.exe`）と DLL を使う。

```sh
python3 tools/bregonig-compat/make_cases.py     # cases.json を作り直す（ケースを足したとき）
tools/bregonig-compat/run.sh                    # 測って bregonig-4.20-*.json を書く（5 分ほど）
cargo test -p matsu-regex --test bregonig_compat  # 結果と matsu-regex を見比べる
```

- `run.sh` は、`%LOCALAPPDATA%\Temp\matsu-bregonig-compat` に作業用のフォルダーを作り、終わったら消す。zip の場所は 1 つ目の引数か環境変数 `BRON_ZIP` で変える
- `cmd.exe` は UNC のカレントディレクトリを嫌うので、スクリプトは Windows 側のフォルダーに移ってから動かす
- ケースは `make_cases.py` に書く。`cases.json` はそこから作る。ケースを足したら `run.sh` をやり直して、結果も一緒にコミットする（テストは、ケースの数と結果の数が合わないと落ちる）

## ケースの書き方

`make_cases.py` の先頭に、項目の説明がある。`kind` は `match`（`BMatchEx`）、`replace`（`g` なしの `BSubstEx`。`start` 以降の最初の一致）、`replace_all`（`g` あり）の 3 つ。

MATSU Editor がDLLと違うことが分かっていて、直さないものには `known_difference` に理由を書く。テストは、その場合に結果がまだ違うことを確かめる（違わなくなったら、記述が古いので落とす）。理由は `make_cases.py` の `KNOWN` にまとめてある。

## 今のところの違い

- DLL は「テキストの終わりから探す」呼び出しを `invalid target parameter` で断る。サクラの行は最終行以外は行末の改行を含むので、行の終わりから探せる場面が多い。MATSU Editor は改行を除いた行を照合するので、行の終わりに空一致が見つかる
- 複数行にまたがる一致（`\R`、`(?s:.)`、改行を含む部分への `\s`・`[^x]`）は、MATSU Editor は照合しない（PR #38 の 1）
- 孤立サロゲート（`\x{D800}`、`\x{110000}` 以上）は Rust の文字列に入らない
- 検索パターンの `\xHH`・`\NNN`（バイト指定）は、DLL（UTF-16 の Onigmo）は断る。MATSU Editor（UTF-8 の Onigmo）は ASCII 文字として通す
- パターンに U+FFFF を書くと、DLL では区切りと読まれる
- 範囲外の `$-{name}[n]`、20 桁のグループ番号などの未定義の動作
