# macOS の CI の自前の実行機（ADR-0027）

CI の `test (macos-arm64)` を、GitHub の macOS の実行機のかわりに、ユーザーの Mac の上の使い捨ての VM で動かす道具。決めたことと理由は [ADR-0027](../../docs/adr/0027-self-hosted-mac-runner.md)。

- ジョブごとに、型の VM（`matsu-ci-golden`）を複製して動かし、ジョブが終わったら消す
- どちらの実行機で動かすかは、リポジトリの変数 `MAC_RUNNER` で選ぶ（`self-hosted` か、それ以外）。`mac-runner up` と `down` が書き換える。Mac を閉じても、15 分たつと GitHub の実行機に戻る（`MAC_RUNNER_SEEN`）
- 第 1 段は `test (macos-arm64)` だけ。E2E（`smoke`・`screens`）は第 2 段

## 置いてある物

| ファイル | 動く所 | 内容 |
|---|---|---|
| `mac-runner` | Mac | `up`・`down`・`status`。VM を 1 つずつ作り、ジョブを 1 つ渡し、消す |
| `build-golden` | Mac | 型の VM を作る（30 分ほど） |
| `lib.sh` | Mac | 上の 2 つが使う設定（リポジトリ、VM の大きさ、時間の上限、Softnet の規則）と関数 |
| `guest/admit.sh` | VM | そのジョブを動かしてよいかを決める |
| `guest/job-started.sh` | VM | 実行機の「ジョブの前の確かめ」。`admit.sh` が断ったら、その場で VM の電源を切る |
| `guest/runner-launch.sh`、`guest/io.github.stofu5678.matsu-runner.plist` | VM | 画面のある利用者のセッションで実行機を動かし、ジョブが終わったら VM の電源を切る |
| `guest/install-guest-agent.sh`、`guest/provision.sh`、`guest/grow-disk.sh` | VM | 型の VM の準備（guest agent、Command Line Tools、Rust、実行機、DNS など） |
| `tests/admit-test.sh` | どこでも | `admit.sh` のテスト（bash と jq が要る） |

## 守っていること

| 経路 | 対策 | 確かめた結果（2026-11-02） |
|---|---|---|
| Mac のファイル | 共有フォルダー・追加のディスクを使わない | － |
| クリップボード | `tart run --no-clipboard` | VM でコピーした文字は Mac に届かない |
| 家の LAN（ルーターなど）、Mac の LAN 側のアドレス | Softnet の既定（インターネットの宛先だけ通す） | 届かない |
| Mac そのもの（ssh、DNS、Bonjour のお知らせ） | Softnet の `out @host,in @host`（両方の向き） | ssh・DNS に届かず、Bonjour で Mac の名前とサービスが見えない |
| マルチキャスト | Softnet の `224.0.0.0/4` | Mac から返事がない |
| 家の公開のアドレス（ルーターが LAN へ転送するかもしれない） | VM を起動するたびに調べて、Softnet で止める | 届かない |
| IPv6 | Softnet は IPv4 と ARP のほかを通さない（0.24.1 のソースで確かめた） | VM に外への IPv6 のアドレスがなく、外へ出ない |
| ホストと VM の連絡 | ネットワークではなく `tart exec`（guest agent） | Mac への通信を止めたまま動く |
| GitHub の資格 | Mac のトークンは VM に入れない。VM には 1 つのジョブ用の設定（JIT）だけを `tart exec` で渡す | － |
| ほかのリポジトリ・フォーク・ほかのワークフローやジョブ | VM の中の「ジョブの前の確かめ」（`admit.sh`）が断り、VM の電源を切る | `tests/admit-test.sh` の 29 件 |
| 前のジョブの残り | ジョブごとに複製を消す | － |

守れないもの: インターネットへの通信は家の回線から出る。VM から抜け出す macOS の仮想化の不具合は防げない（Mac の macOS と Tart を新しく保つ）。

## 用意（1 回だけ）

1. Tart（Mac の Sandbox で入れたもの。`~/bin/tart`）
2. Softnet 0.24.1 を root の setuid で入れる（パスワードを聞かれる）

   ```sh
   mkdir -p ~/matsu-mac-runner/softnet-0.24.1 && cd ~/matsu-mac-runner/softnet-0.24.1
   curl -fsSLO https://github.com/cirruslabs/softnet/releases/download/0.24.1/softnet.tar.gz
   shasum -a 256 softnet.tar.gz   # 8ff8ff099c2b5ac1b1128e7999c4a730ac85f486c8a4cb0239eb98432859a0f7
   tar -xzf softnet.tar.gz
   sudo install -o root -g wheel -m 4755 softnet /usr/local/bin/softnet
   ```

   Softnet のバイナリには Apple の開発者の署名がない（ad-hoc の署名だけ。Homebrew で入れても同じファイル）。ハッシュは GitHub のリリースと同じことを確かめて入れた。

3. トークンを作って、Mac のキーチェーンに入れる
   - GitHub の Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token。名前 `matsu-mac-runner`、期限 1 年、Repository access は `stofu5678/matsu_editor` だけ、Repository permissions は Administration と Variables を Read and write（Metadata の Read-only は自動で付く）
   - Mac の前で（WSL や ssh を通さずに）: `security add-generic-password -a "$USER" -s matsu-mac-runner -w`（2 回聞かれるので、2 回とも貼り付ける）
4. 型の VM を作る: `tools/mac-runner/build-golden`

## 使い方

```sh
tools/mac-runner/mac-runner up         # 使い始める
tools/mac-runner/mac-runner status     # 様子を見る
tools/mac-runner/mac-runner down       # やめる（動いているジョブは最後まで動かす）
tools/mac-runner/mac-runner down --now # すぐやめる（動いているジョブは失敗する）
```

- 旅行などで Mac を使えないとき: `down` しておく。忘れても、Mac が止まって 15 分たつと、CI は GitHub の実行機で動く。ただし、止まる直前に Mac を選んだジョブは待ったままになる（調整役が取り消して流し直す）
- Mac の外から切り替える: GitHub の画面（Settings → Secrets and variables → Actions → Variables の `MAC_RUNNER`）か、`gh variable set MAC_RUNNER --body github`
- 記録: `~/Library/Application Support/matsu-mac-runner/runner.log`
- Mac の Sandbox の VM と合わせて、macOS の VM は同時に 2 つまで（利用許諾）。メモリが 16 GB なので、Sandbox を使う間は `down` しておく

## 作り直すとき

型の VM は `build-golden` で作り直す（古い型は、新しい型ができてから消す）。次のときに要る。

- GitHub が古い実行機を受け付けなくなったとき（新しい版が出てからおよそ 1 か月）: `guest/provision.sh` の `RUNNER_VERSION` と `RUNNER_SHA256` を上げる
- `rust-toolchain.toml` の Rust を上げたとき: `RUST_TOOLCHAIN`（上げなくても、ジョブがその版を入れるので動くが、毎回入れる分だけ遅い）
- Cirrus Labs の macOS のイメージを新しくするとき: `lib.sh` の `VANILLA_IMAGE`（digest で固定）

## 片付け（やめるとき）

```sh
tools/mac-runner/mac-runner down
tart delete matsu-ci-golden
sudo rm /usr/local/bin/softnet
security delete-generic-password -s matsu-mac-runner
rm -rf ~/Library/Application\ Support/matsu-mac-runner
```

GitHub のトークンも、Settings → Developer settings で消す。
