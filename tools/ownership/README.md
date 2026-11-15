# 所有権の測定の道具

[ADR-0033](../../docs/adr/0033-ownership.md) の、月に 1 回の測定に使う。標準ライブラリだけの Python 3 で動く。

## ownership-metrics.py

Rust のコードにある、所有権まわりの書き方（`.clone()`、`Rc`、`Arc`、`RefCell`、`.to_owned()`、`unsafe` など）の数を、本番のコードとテストに分けて数え、1,000 行あたりの表を出す。

```sh
python3 -I tools/ownership/ownership-metrics.py .
```

## clippy-summary.py

Clippy の JSON を、lint ごと、本番のコードとテストに分けて数える。テストの分け方は `ownership-metrics.py` と同じ（`ownership-metrics.py` を同じフォルダーから読むので、2 つのファイル名は変えない）。

```sh
cargo clippy --workspace --all-targets --locked --message-format=json -- \
  -W clippy::redundant_clone -W clippy::needless_pass_by_value \
  -W clippy::trivially_copy_pass_by_ref -W clippy::ref_option \
  -W clippy::needless_pass_by_ref_mut -W clippy::clone_on_ref_ptr \
  -W clippy::assigning_clones -W clippy::rc_buffer \
  -W clippy::needless_collect -W clippy::implicit_clone > clippy.json
python3 -I tools/ownership/clippy-summary.py clippy.json .
```

前の月の結果と比べて、増えていないかを見る。2026-11-11 と 2026-11-15 の数は、ADR-0033 の背景にある。
