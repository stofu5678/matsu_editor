# vendor/mruby

mruby を、Ruby なしでビルドできる形にして置いたもの。`build.rs` が `cc` でコンパイルする。

- 上流: https://github.com/mruby/mruby 、タグ `4.0.0`（コミット `831da26b9021de0369d17b71b5667e2941a1a32d`）
- 取得日: 2026-10-16
- ライセンス: MIT（`LICENSE`）。例外は `LEGAL` に書いてある（下の「ライセンス」）

## 中身

| 場所 | 中身 |
|------|------|
| `include/` | 上流の `include/`（そのまま） |
| `src/` | 上流の `src/`（そのまま） |
| `gems/<gem>/{src,core,include}` | 使う mrbgem の C のソース（`test/`・`mrblib/`・`mrbgem.rake` は入れない） |
| `gems/mruby-compiler/core/` | `codegen.c`・`y.tab.c`（上流が生成済みで持っているパーサー）・`node.h`・`lex.def` |
| `generated/include/mruby/presym/{id,table}.h` | rake が生成した、シンボルの表 |
| `generated/mrblib.c` | コアライブラリの Ruby で書いた部分を、バイトコードの C の配列にしたもの |
| `generated/gem_init.c`、`generated/mrbgems/<gem>.c` | rake が生成した、gem の初期化コードと、gem の Ruby 部分の C の配列 |
| `tools/build_config.rb` | 生成のときに使うビルド設定（gem の一覧もここ） |
| `tools/regen.sh` | 生成のやり直し |

`generated/` は、`MRB_UTF8_STRING`・`MRB_USE_BIGINT`・`MRB_USE_SET` を定義して、`tools/build_config.rb` の gem を入れたときの結果。定義や gem を変えたら、`../../build.rs` の定義も合わせて、必ず生成し直す（シンボルの表が合わないと動かない）。

## 作り直す手順

Ruby 3 と rake と git が要る（この手順だけ。通常のビルドや CI には要らない）。

```sh
crates/matsu-mruby-sys/vendor/mruby/tools/regen.sh <空の作業ディレクトリ> [タグ]
```

やることは次のとおり。

1. 作業ディレクトリに上流を `git clone --depth 1 --branch <タグ>` する（タグの既定値は `VERSION`）。
2. `MATSU_MRUBY_BUILD_DIR=<作業>/build MRUBY_CONFIG=tools/build_config.rb rake` で、mruby 自身のビルドを 1 回回して、生成ファイルを作る。
3. 必要な C のソースと生成ファイルを、このディレクトリに写す（`README.md`・`VERSION`・`tools/` は残す）。

版を上げるときは、`VERSION` と、このファイルの上流のタグ・コミットも直す。

## 入れた gem と理由

マクロが使う分だけにした。入出力・ファイル・ソケット・時刻の待ち・外部コマンドは入れない（マクロはエディタの API でしか文書に触れないため）。

| gem | 理由 |
|-----|------|
| `mruby-compiler` | ソースの文字列を実行するのに要る（`mruby-eval` も使う） |
| `mruby-error` | `Exception` まわりの拡張 |
| `mruby-bigint` | 大きな整数（`2**70` など） |
| `mruby-sprintf` | `format`・`sprintf`・`String#%` |
| `mruby-string-ext` `mruby-array-ext` `mruby-hash-ext` `mruby-numeric-ext` `mruby-range-ext` `mruby-symbol-ext` `mruby-object-ext` `mruby-kernel-ext` `mruby-class-ext` `mruby-proc-ext` `mruby-compar-ext` `mruby-enum-ext` `mruby-toplevel-ext` | 標準の Ruby で普通に使うメソッド（`start_with?`・`each_slice`・`sum`・`tap`...）。コアだけだと足りない |
| `mruby-enumerator` `mruby-enum-lazy` `mruby-enum-chain` `mruby-fiber` | `each` をブロックなしで呼んだときの `Enumerator`、`loop` |
| `mruby-set` `mruby-struct` `mruby-data` | `Set`・`Struct`・`Data` |
| `mruby-time` `mruby-strftime` | `Time` と `strftime` |
| `mruby-random` `mruby-math` | `rand`・`Random`・`Math` |
| `mruby-pack` | `Array#pack`・`String#unpack`（バイト列の処理） |
| `mruby-catch` | `catch`・`throw` |
| `mruby-objectspace` | `ObjectSpace`（ほかの gem が頼る） |
| `mruby-metaprog` `mruby-method` `mruby-eval` `mruby-binding` | `define_method`・`instance_variable_get`・`Method`・`eval`・`binding`（マクロを書く人が使いうる。`mruby-eval` が `mruby-binding` を引く） |

入れなかったもの: `mruby-io`・`mruby-dir`・`mruby-socket`・`mruby-sleep`・`mruby-task`・`mruby-exit`・`mruby-os-memsize`・`mruby-bin-*`・`mruby-complex`・`mruby-rational`・`mruby-cmath`・`mruby-benchmark`・`mruby-test`。

`puts`・`printf`・`pp`・`warn`・`$stdout`・`$stderr` は `mruby-io` にあるので、`crates/matsu-script/src/mruby_boot.rb` で定義し直している。

## ライセンス

全体は MIT。次のファイルは別の条件で、どれも MIT / Apache-2.0 の配布と両立する。GPL 系のものは入れていない。

| ファイル | 条件 |
|----------|------|
| `src/string.c`（`memsearch_swar`） | 2 条項 BSD（Wojciech Muła）。全文は `LEGAL` |
| `src/fmt_fp.c` | パブリックドメイン（Dave Hylands） |
| `gems/mruby-bigint/core/bigint.h` ほか | パブリックドメイン（FGMP。`gems/mruby-bigint/README-fgmp.md`） |
| `gems/mruby-set/LICENSE` | 同じ MIT（gem の作者の表記） |
| `gems/mruby-compiler/core/y.tab.c` | Lrama 0.7.0（bison 互換）が生成したパーサー。ヘッダーは GPL-3.0-or-later だが、Bison の特別例外（"you may create a larger work that contains part or all of the Bison parser skeleton and distribute that work under terms of your choice"）がついている。パーサーの生成器そのものではないので、MIT で配布してよい |

`mruby-dir` の `dirent.c`（`LEGAL` が挙げる MIT 風のファイル）は入れていない。
