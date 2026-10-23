# ADR-0025: マクロの言語を増やす（第 2 弾以降の候補と順番）

- 状態: 決定（2026-10-24 にユーザーが、下の「決めたこと」の 1〜6 を推奨どおりに決めた。7 とプラグインの扱いは残課題）
- 日付: 2026-10-24
- 関連: [ADR-0007](0007-macros-and-plugins.md)（マクロとプラグイン。このADRは、その「第 2 弾」の表を置き換える案）、[docs/poc/json-rpc.md](../poc/json-rpc.md)

## 背景

2026-10-24 にユーザーが「スクリプト言語の追加を検討」と頼んだ。ADR-0007 の第 2 弾は PowerShell・AppleScript・VBScript・Emacs Lisp だが、どれも実装していない。候補を調べ直し、足す言語と順番の案を出す。2026-10-24 にユーザーが推奨どおりに決めた（下の「決めたこと」）。実装は、別の PR で言語ごとに行う。

名前は、ADR-0007 の書かれた当時の `ume-script`・`ume.ini` ではなく、今の `matsu-script`・`matsu.ini` で書く。

## 今の作り（調べたこと）

- `crates/matsu-plugin/src/session.rs` の `Language`: 外部の処理系は Node.js・Python・Ruby・PHP の 4 つ。同梱の処理系は `Engine`（Lua・QuickJS・mruby・MicroPython）で、`crates/matsu-script/` の `matsu-script --engine <名前>` が動かす。拡張子と言語の対応は `Language::of_file` の 1 か所
- 外部の言語を足す作業: `session.rs` に変数と拡張子と既定の処理系名、`boot/` にブート（`boot.*`）、`codegen.rs` に API の生成（`api_gen.*`）、`boot.rs` の `FILES` に埋め込み、テスト、設定画面の処理系の欄。API の一覧（`catalog.rs`）は 1 つなので、言語ごとに生成物とブートだけが増える
- 同梱の言語を足す作業: `matsu-script/src/<言語>.rs` と `engine::by_name` への 1 行。API は `catalog::entries()` から束ねる
- CI（`.github/workflows/ci.yml`）: Linux x64 だけ `require_interpreters: '1'` で、Node.js・Python・Ruby・PHP がないと失敗する。ほかの OS では、処理系がなければテストを飛ばす
- 通信の形（JSON-RPC、ASCII の 1 行 1 メッセージ、行番号つきのエラー、UTF-8）は、すべての言語で共通。スクリプトの渡し方は違う: 同梱の処理系には、エディタが読んだ中身を `initialize` の `params.script` で渡し、外部の処理系（Node.js・Python・Ruby・PHP）には、ファイルのパスを起動の引数で渡す（`session.rs` の `start`）。新しい言語の作業は、その言語でこの形を話す 100〜300 行のブートを書くことが中心になる（Lua のブートは約 200 行。docs/poc/json-rpc.md）

## サクラの利用者が持っているマクロ

- サクラは、WSH（Windows Script Host）に登録されたスクリプトエンジンを、ファイルの拡張子から探して動かす（`sakura_core/macro/CWSHManager.cpp` の `CWSHMacroManager::Creator`、エンジンは `CWSH.cpp` で `CLSIDFromProgID` により生成）。実際に使われているのは JScript（`.js`）と VBScript（`.vbs`）。Python は別の仕組み（`CPythonMacroManager.cpp`、`.py`）
- ADR-0007 のとおり、ソースのままでの互換は求めない。JScript は `.js` を QuickJS で動かすので、`WScript.*` や `ActiveXObject` を使わない部分は動く見込み。VBScript は、別の言語に書き換える必要がある。VBScript のエンジンを足しても、`Editor` の API を VBScript に包むだけで、サクラの `Editor.InsText` と同じ名前を使う部分以外は互換にならない
- そのため、サクラの利用者のための手当ては「言語を足す」より「VBScript から書き換える手引きを作る」の方が効く（本ADRでは決めない。残課題）

## 第 2 弾の 4 言語

### PowerShell（`.ps1`）

- 入れるもの: Windows には 5.1 が入っている。7 系は別に入れる（win-arm64・osx-arm64・linux-arm64 の配布物がある。[PowerShell の最新リリース v7.6.8](https://github.com/PowerShell/PowerShell/releases/latest)、2026-10-22）。処理系の名前は `pwsh`（7）と `powershell`（5.1）
- 作業: 標準入出力で JSON-RPC を話すブート。docs/poc/json-rpc.md に Windows ARM64 の PowerShell 5.1 で試した結果がある: `ConvertTo-Json` は `-Depth` と `-Compress` が要り、ASCII へのエスケープは自分で書く。ドットソースはフルパスでないと失敗した。スクリプトの文字コード（5.1 は BOM なしを ANSI として読むおそれがある）は、同梱の処理系と同じく、エディタが中身を `initialize` で渡す形にすれば避けられる見込み（未確認）
- 起動の時間: この環境（Linux ARM64、PowerShell 7、`-NoProfile`）で 690〜980 ms。ほかの処理系（Node.js 36 ms、Python 19 ms、Ruby 97 ms）より 1 桁遅い。Windows ARM64 の 5.1 は約 150 ms と記録がある（docs/poc/json-rpc.md。条件が違うので、上の値とは比べられない）
- CI: GitHub の Ubuntu 24.04・Windows 2025・macOS 15 のイメージに入っている（[Ubuntu](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md): 7.6.6、[Windows](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md)、[macOS](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-arm64-Readme.md): 7.6.5）。Windows ARM64 のイメージは未確認
- 問題: 実行ポリシー（`-ExecutionPolicy Bypass` はグループポリシーを越えられない。docs/poc/json-rpc.md）。ファイルを渡さず `-Command -` や標準入力で渡せば関係しない見込み（未確認）。起動が遅い

### AppleScript（`.applescript`）

- 入れるもの: macOS だけで、OS 標準の `osascript`。ほかの OS では使えない
- 作業: AppleScript は標準入出力を直接読み書きしにくいので、ADR-0007 のとおり中継が要る。docs/poc/json-rpc.md の Codex の指摘に、AppleScriptObjC の `availableData` が分割して返る（ためる必要がある）とある。中継を `matsu-script` の中に作る案（`osascript` を子プロセスにして、`Editor` の呼び出しを Apple Event ではなく中継の標準入出力で受ける）と、AppleScript 自身に JSON を書かせる案がある。どちらも他の言語より作業が大きい
- CI: macOS のイメージの `osascript` は、このADRの調査では確かめていない（OS 標準のはずで、macOS 15 のイメージ一覧には載っていない）
- 問題: macOS 以外で動かせない。マクロが他のアプリを操作するときは、macOS のオートメーションの許可が要る
- JScript for Automation（`osascript -l JavaScript`）も同じ中継で動かせるが、拡張子 `.js` は QuickJS が使っているため、割り当てを決める必要がある。今回は対象に入れない

### VBScript（`.vbs`）

- Microsoft の公式の予定: VBScript は Windows 11 24H2 から「オンデマンドの機能」になり、はじめは既定で入っている。のちに退役する（[Microsoft Learn: Resources for deprecated features](https://learn.microsoft.com/en-us/windows/whats-new/deprecated-features-resources)、更新 2026-09-23）。時期は、2024 年の報道で、2027 年ごろに既定で無効（入れれば使える）になり、その後に削除、削除の日は未定とされている（[The Register, 2024-05-23](https://www.theregister.com/2024/05/23/windows_11_24h2_vbscript/)。公式の文書には年は書かれていない）。現在の予定が変わっていないかは、2026-10-24 時点で公式の文書から確かめられていない
- docs/poc/json-rpc.md の Windows ARM64 での結果: スクリプトのファイルは UTF-16LE（BOM 付き）でないと日本語を読めない。`WScript.Echo` はほかの関数に差し替えられない
- 結論: 退役が決まっている言語に新しい作業をかける理由が弱い。Windows 専用になる。足さない案を勧める（下）

### Emacs Lisp（`.el`）

- 入れるもの: Emacs 30 以降（JSON の関数が本体に入ったのが 30。docs/poc/json-rpc.md）。ユーザーが 2026-10-11 に決めている。Emacs の本体は GPL だが、利用者が入れたものを別プロセスで動かすだけで、このリポジトリの配布物には入らない。Emacs の GPL のソースは読んでいない（マニュアルと試作の結果のみ）
- 作業: ADR-0007 の「第 2 弾の言語で確かめること」のとおり（`emacs -Q --batch`、1 式ずつ読んで行番号を出す、UTF-8 固定、LF だけを送る）。起動は 28 ms（Linux）/ 127 ms（Windows）
- CI: Ubuntu・Windows・macOS のイメージに Emacs は入っていない（上の一覧に載っていない）。CI で入れる手間（`apt`・`brew`・`choco`）が要る。Windows ARM64 の公式の配布は x64 だけ（エミュレーションで動いた）
- 使う人は Emacs の利用者に限られるが、サクラの利用者と編集の好みが近い人たちの要望として決まっている

## そのほかの候補

調べた日は 2026-10-24。起動の時間は、この環境（WSL2 上の Linux ARM64）での 1 回または 2 回目の値で、目安。

### 外部の処理系を使うもの

| 言語 | 動かし方 | 拡張子の案 | 調べたこと |
|---|---|---|---|
| TypeScript | Node.js が型を取り除いて動かす | `.ts` | Node.js は v22.18.0・v23.6.0 から、フラグなしで `.ts` を動かす。v24.12.0・v25.2.0 から「安定」（[Node.js の文書](https://nodejs.org/api/typescript.html)）。型の検査はしない。enum などの、型を消すだけでは動かない構文は、エラーになる。`tsconfig.json` は読まない。この環境の Node.js v25.2.1 で、`.ts` の起動は 75〜80 ms（`.js` は 36 ms）。CI のイメージの Node.js は Ubuntu が 22.23.3 と 24.21.0、Windows と macOS が 22.23.x で、どれも 22.18 以上 |
| Deno | `deno run` | `.ts` を Deno で動かす設定で足りる | 最新 2.9.8（2026-10-21）、MIT。TypeScript をそのまま動かす。CI のイメージには入っていない。許可の仕組み（権限のフラグ）があり、ファイルやネットワークを禁じられる |
| Bun | `bun run` | 同上 | 最新 1.4.4（2026-10-22）。MIT。Windows ARM64 の配布あり。JavaScriptCore（LGPL）を静的にリンクしているため、配布するなら要注意。だが、利用者が入れたものを動かすだけで、このリポジトリには入らない。CI のイメージには入っていない |
| Perl | `perl` | `.pl` | 起動 5 ms。Ubuntu・macOS のイメージにあり、Windows のイメージ一覧は Perl を載せているが版は確かめていない。Windows の利用者は Strawberry Perl などを入れる。JSON は `JSON::PP`（標準に入っている）が使える見込み（未確認） |
| Tcl | `tclsh` | `.tcl` | macOS のイメージにあり（Tcl/Tk 8.6.18）、Ubuntu は確かめていない。Windows のイメージには入っていない。需要は小さい見込み |
| C# | `dotnet run file.cs` | `.cs` | .NET 10 SDK から（[Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/core/sdk/file-based-apps)）。実行環境ではなく SDK が要る。この環境で初回 5.2 秒、2 回目から 0.2 秒（ビルドの結果を残す）。同じファイルを同時に動かすと競合するおそれがあると文書にある。CI のイメージに .NET SDK 8〜10 がある |
| Go | `go run` | `.go` | 初回 0.5 秒、2 回目から 0.18 秒。コンパイル言語で、`Editor` の API を Go のコードに生成する作業がいる。CI のイメージにある（Ubuntu: 1.24〜1.26、Windows・macOS にもある） |
| Rust | `cargo -Zscript` | - | 不安定機能で、nightly と `-Zscript` が要る。安定化の予定は文書に書かれていない（[Cargo の文書](https://doc.rust-lang.org/cargo/reference/unstable.html#script)）。見送る |
| Kotlin | Kotlin のスクリプト | `.kts` | CI のイメージにあるが（Kotlin 2.4）、JVM の起動が要る。今回は起動の時間を測っていない。需要は小さい見込み |
| Scheme・Common Lisp・Clojure | SBCL・Racket・Babashka など | - | docs/poc/json-rpc.md に、6 つの処理系を試した結果がある。「Emacs の利用者には Emacs Lisp、一般の Lisp には SBCL、3 番手は Racket。Babashka・Gauche・Hy は要望が出てから」と調べた側の勧め。今回も同じ |

### 同梱するもの（`matsu-script` に入れる）

| 言語 | ライセンス | 調べたこと |
|---|---|---|
| Fennel（`.fnl`） | MIT（[fennel-lang.org](https://fennel-lang.org/)、最新 1.6.1） | Lua に変換する Lisp 風の言語。Lua の処理系は同梱しているので、Fennel のコンパイラ（Lua で書かれた 1 ファイル）を足し、`.fnl` を Lua に変換して動かす形が考えられる。エラーの行番号を元のソースに合わせる工夫が要る。ファイルの大きさは未測定 |
| Teal（`.tl`） | 確かめていない | 型つきの Lua。リリースの配布物が Linux x64 と Windows x64 の実行ファイルだけで、`tl.lua` 単体の入手は確かめていない。需要も不明 |
| Rhai | MIT OR Apache-2.0（[crates.io](https://crates.io/crates/rhai) 1.27.0、2026-10-21 更新） | 純粋な Rust。C のビルドが要らない。ダウンロードは累計 1300 万件。Rust の API を束ねるので、Lua と同じく `catalog::entries()` から作れる。サクラとの関係はない |
| Rune | MIT OR Apache-2.0（0.14.2、2026-05 更新） | 純粋な Rust。需要は小さい |
| Koto | MIT（0.16.1、2026-01 更新） | 純粋な Rust。需要は小さい |
| Steel | MIT OR Apache-2.0（steel-core 0.8.3、2026-08 更新） | Rust の Scheme。1.0 前。docs/poc/json-rpc.md の「後回しにするもの」にある |
| Starlark | Apache-2.0（starlark 0.14.2） | 設定向けの Python 風の言語。マクロには向かない（再帰やループの制限がある。需要も小さい見込み） |
| Janet | MIT（v1.42.1、2026-09） | C で書かれた小さな Lisp。需要は小さい |
| WebAssembly（wasmtime） | Apache-2.0 WITH LLVM-exception（wasmtime 50.0.0）。`deny.toml` の許可の一覧にある | どの言語でも、WebAssembly に直せれば動かせる（砂場つき）。ただし、マクロの利用者が WebAssembly を作る手間が大きい。Python や Ruby などの「ネイティブ拡張」は使えない（ADR-0007 が見送った理由）。wasmtime は大きい（配布物の大きさは未測定）。今は見送る |

「同梱」の大きさは、どの言語も今回測っていない。`matsu-script` に足す前に、実際にビルドして測る。

## 比べる観点

| 言語 | 使う人 | OS | 利用者が入れるもの | 起動 | 同梱の大きさとライセンス | ブートと API の作業 | CI | 安全（砂場） |
|---|---|---|---|---|---|---|---|---|
| TypeScript | JavaScript/Web 開発者 | 全部 | Node.js 22.18 以上 | 75〜80 ms | 同梱しない | 小（`boot.cjs` を使い回す。`.ts` を Node.js に渡す） | Node.js が全 OS にあり、今のテストが使える | なし（利用者の権限で動く） |
| PowerShell | Windows の管理者 | 全部（5.1 は Windows のみ） | 5.1 は入っている。7 は別 | 7 は約 0.7〜1 秒（Linux）、5.1 は約 150 ms（Windows ARM64）。条件が違う | 同梱しない | 中 | 全 OS にある | なし。実行ポリシーが邪魔をしうる |
| Perl | 古くからの Unix 利用者 | 全部（Windows は別に入れる） | Perl | 5 ms | 同梱しない | 小〜中（JSON は `JSON::PP`） | Ubuntu・macOS にある。Windows は確かめていない | なし |
| Emacs Lisp | Emacs の利用者 | 全部 | Emacs 30 以降 | 28〜127 ms | 同梱しない（GPL なので同梱できない） | 中（試作済み） | 入れる手間が要る | なし |
| AppleScript | macOS の利用者 | macOS のみ | なし（OS 標準） | 未測定 | 同梱しない | 大（中継） | macOS で確かめる | macOS の許可の仕組み |
| VBScript | サクラの古い利用者 | Windows のみ | なし（退役予定） | 未測定 | - | 中 | Windows のみ | なし |
| C# | .NET の開発者 | 全部 | .NET 10 SDK | 初回 5 秒、以後 0.2 秒 | 同梱しない | 中（API を C# に生成） | イメージにある | なし |
| Go | Go の開発者 | 全部 | Go | 0.18〜0.5 秒 | 同梱しない | 中（API を Go に生成） | イメージにある | なし |
| Fennel | Lua の利用者の一部 | 全部 | なし | Lua と同じ+変換 | MIT。大きさ未測定 | 小（変換だけ） | `cargo test` だけで済む | 砂場なし。今の Lua と同じく利用者の権限で動く（`lua.rs` は `io`・`os` を外していない） |
| Rhai | Rust の利用者の一部 | 全部 | なし | 未測定 | MIT OR Apache-2.0。大きさ未測定 | 小〜中 | `cargo test` だけで済む | 作り次第（足すなら、ファイルを開けない作りにできる） |

## 提案（2026-10-24 に推奨どおり決まった）

### 足す言語と順番

1. **TypeScript（`.ts`、外部、Node.js 22.18 以上）**: 作業がもっとも小さい。Web 系の利用者が多く、型つきの補完（`Editor` の型定義の生成）で生成 AI にも書かせやすい。マクロの `.ts` に型の検査はない。Deno や Bun を使いたい人は `matsu.ini` で処理系を変える
2. **PowerShell（`.ps1`、外部）**: Windows でもっとも使われるシェルの言語で、サクラの Windows の利用者に近い。起動が遅いことは、マクロの実行時間の欄に注意書きを出す
3. **Perl（`.pl`、外部）**: 起動が速く、作業が小さい。需要は読みきれないので、2 つ目の PR のあとで、要望があるかを見て決めてもよい
4. **Emacs Lisp（`.el`、外部）**: ユーザーが決めた分。CI に Emacs を入れる手間があるので、CI の準備を同じ PR に含める
5. **AppleScript（`.applescript`、外部、macOS のみ）**: 中継の作りが重いので最後。macOS の利用者の要望が出てから

足さない: **VBScript**（退役予定。サクラの利用者には、AI で JScript か別の言語に書き換える手引きで足りる）、Rust スクリプト（不安定）、WebAssembly、Rhai・Rune・Koto・Steel・Starlark・Janet（需要がなく、同梱の処理系は増えるほど配布物と保守が重くなる）、Kotlin・Tcl（需要が小さい）。C# と Go と Scheme 系は、要望が出てから（コンパイル言語は ADR-0007 の第 3 弾）。Fennel は、Lua の利用者の要望が出たら足す（作業が小さいので後からでも間に合う）。

### 同梱か外部か

TypeScript・PowerShell・Perl・Emacs Lisp・AppleScript は、すべて外部にする。理由: どれも OS または利用者の環境に処理系が既にあり（PowerShell・AppleScript は OS 標準。Node.js は、同梱の QuickJS だけで足りる利用者には新しい要件になる。TypeScript の `.ts` は、型を取り除く作業が要るために同梱の QuickJS では動かせず、Node.js を入れてもらう）、同梱すると配布物が大きくなり、ライセンスの確認（Emacs は GPL で同梱できない）も増える。

### 拡張子と処理系の変更

ADR-0007 の規則（拡張子で処理系が決まる。`matsu.ini` で変えられる）をそのまま使う。

| 拡張子 | 言語 | 動かし方 | 利用者が入れるもの |
|---|---|---|---|
| `.ts` | TypeScript | 外部（Node.js。Deno・Bun に変更できる） | Node.js 22.18 以上 |
| `.ps1` | PowerShell | 外部 | PowerShell 7（Windows は 5.1 でも） |
| `.pl` | Perl | 外部 | Perl |
| `.el` | Emacs Lisp | 外部 | Emacs 30 以降 |
| `.applescript` | AppleScript | 外部（macOS のみ） | なし |

### PR の順番

言語ごとに 1 つの PR にする。順番は上の 1〜5。共通の作業（処理系の欄の設定画面、ブートの生成の仕組み）は、最初の PR で「言語を足す手順」を整え、後ろの PR はその手順に従う。各 PR で、CI に処理系を入れる、または「入っていなければ飛ばす」にする。Linux x64 の `require_interpreters` には、その言語を足す。

### ADR-0007 との関係

- ADR-0007 の「マクロ」の部分は、「決定」にしてよい時期に来ている。理由: 通信の形、同梱の言語（Lua・QuickJS・MicroPython・mruby）、外部の言語（Node.js・Python・Ruby・PHP）は、ユーザーが決めた日付つきで、実装も済んでいる。残っているのは「第 2 弾」と「第 3 弾」の表と、常駐するプラグイン（イベントを受ける形。ADR-0007 の通信の形の 5 に「別の形で後で決める」とある）。プラグインが決まっていないので、ADR 全体を「決定」にするか、マクロの部分だけを決めてプラグインを別のADRに分けるかは、ユーザーが決める
- ADR の規則（docs/adr/README.md）は、古い ADR の決定を書き換えず、新しい ADR を足して置き換えることだ。そこで、このADRが決まったら、ADR-0007 の状態を「決定（第 2 弾の表は ADR-0025 で置き換え）」に変える。ADR-0007 の本文は書き換えない。この PR は ADR-0007 を変えない

## 決めたこと（2026-10-24 にユーザーが推奨どおりに決めた）

1. 足す言語と順番: TypeScript → PowerShell → Perl → Emacs Lisp → AppleScript
2. VBScript は足さない
3. すべて外部の処理系にする。同梱する言語（Fennel・Rhai など）は足さない
4. 拡張子は `.ts`・`.ps1`・`.pl`・`.el`・`.applescript`。`.js` の JScript for Automation は割り当てない
5. Deno・Bun は別の言語にせず、処理系の設定（`matsu.ini`）で変える。`matsu.ini` の形式は、実装の PR で確認する（ADR-0018: 利用者のデータ）
6. ADR-0007 のマクロの部分を「決定」にし、第 2 弾の表をこのADRで置き換える。ADR-0007 の本文は書き換えず、状態の行と README の行だけで示す

## 残っていること

- 同梱の候補の大きさは、実際にビルドして測っていない
- AppleScript の中継、PowerShell 5.1 の文字コードと実行ポリシー、Windows ARM64 の Perl・Emacs の入手は、実装のときに確かめる
- GitHub の Windows ARM64 のイメージにある処理系は確かめていない
- VBScript の最新の予定は、公式の文書では年が示されていない。実装しない案なので影響は小さい
- 決まっていない（残課題）
  - 常駐するプラグインの部分を、ADR-0007 に残すか、別の ADR に分けるか
  - サクラの VBScript マクロを書き換える手引き（生成 AI に渡す対応表）を用意するか
