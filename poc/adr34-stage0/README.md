# ADR-0034 段階 0 の試作

[ADR-0034](../../docs/adr/0034-proportional-fonts.md) の段階 0 で、GPUI が `Window` なしで別のスレッドから 1 文字の送り幅を測れるかと、比例フォントを 1 文字ずつ shape する費用を調べた、捨てる前提の試作。結果は ADR の「段階 0 の結果」。ワークスペースにも CI にも入れていない。

```sh
cd poc/adr34-stage0
cargo run --release                      # 比例のフォントと等幅のフォントを探して測る
cargo run --release -- "Noto Sans JP"    # フォント名を指定する
```

ウィンドウも画面も要らない（`gpui_platform::bench_text_system()` の、プラットフォームのテキストシステムだけを使う）。
