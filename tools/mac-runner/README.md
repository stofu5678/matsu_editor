# macOS の CI の自前の実行機（ADR-0027）

CI の `test (macos-arm64)` と、パッケージ作りの `package (macos-arm64)` を、GitHub の macOS の実行機のかわりに、ユーザーの Mac の上の使い捨ての VM で動かす道具。決めたことと理由は [ADR-0027](../../docs/adr/0027-self-hosted-mac-runner.md)。

- ジョブごとに、型の VM（`matsu-ci-golden`）を複製して動かし、ジョブが終わったら消す
- どちらの実行機で動かすかは、リポジトリの変数 `MAC_RUNNER` で選ぶ（`self-hosted` か、それ以外）。`mac-runner up` と `down` が書き換える。Mac を閉じても、15 分たつと GitHub の実行機に戻る（`MAC_RUNNER_SEEN`）
- 動かすのは `test (macos-arm64)`（`ci.yml`）と、パッケージ作りの `package (macos-arm64)`（`release.yml` の nightly とバージョン、`package-check.yml`。ADR-0027 の 2026-11-06 の追記）。E2E（`smoke`・`screens`）は第 2 段。`mac-check.yml` は移していない
- 実行機を選ぶ決まりは `.github/actions/mac-runner/action.yml` に 1 つだけある（`ci.yml` と `package.yml` が使う）
- 1 度に動かす VM は 1 つなので、`test` とパッケージ作りは順番を待つ

## 置いてある物

| ファイル | 動く所 | 内容 |
|---|---|---|
| `mac-runner` | Mac | `up`・`down`・`status`。VM を 1 つずつ作り、ジョブを 1 つ渡し、消す。ssh から呼ぶと、ログイン中のセッションで動かし直す |
| `build-golden` | Mac | 型の VM を作る（30 分ほど） |
| `relocate-recovery.py` | Mac | 型の VM のディスクを広げるため、止めた VM のディスクのイメージの中で、macOS の復旧用の区画を末尾へ移す（diskutil は SIP のため動かせない） |
| `lib.sh` | Mac | 上の 2 つが使う設定（リポジトリ、VM の大きさ、時間の上限、Softnet の規則）と関数 |
| `guest/admit.sh` | VM | そのジョブを動かしてよいかを決める。通す組は設定の `ALLOW=<ワークフローのファイル>:<ジョブ>[:<イベント>]`（イベントを付けると、そのイベントだけ）。`lib.sh` の `ALLOWED_JOBS` が元 |
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
| ほかのリポジトリ・フォーク・ほかのワークフローやジョブ | VM の中の「ジョブの前の確かめ」（`admit.sh`）が断り、VM の電源を切る | `tests/admit-test.sh` の 47 件 |
| 前のジョブの残り | ジョブごとに複製を消す | － |

守れないもの: インターネットへの通信は家の回線から出る。VM から抜け出す macOS の仮想化の不具合は防げない（Mac の macOS と Tart を新しく保つ）。

- **VPN をつないでいる間は動かさない**（ADR-0027 の決めたことの 6）。VPN の先の網には、インターネットの宛先に見えるアドレスがあり、Softnet は通してしまう。全部の通信を VPN に流す形では、家の公開のアドレスも分からなくなる。スクリプトは、VPN がつながっている（`scutil --nc list` が Connected、または既定の経路がトンネル）と、VM を起動しない
- VM の中のジョブは、その VM の実行機の資格（1 つのジョブ用）を読める。資格はそのジョブが終わると使えなくなり、Mac のトークンは VM に入らない
- 再利用のワークフロー（`package.yml`）のジョブは、`GITHUB_WORKFLOW_REF` が呼んだ側（`release.yml`・`package-check.yml`）になる、と見て通している。断られたときの理由（ワークフローのファイル、ジョブ、イベント）は、そのジョブのログに出る（VM が止まるので、GitHub に届かないこともある）。見え方が違えば `lib.sh` の `ALLOWED_JOBS` を直す
- 断ったジョブは、VM の電源が切れるので、GitHub の上では 10 分ほどたってから「実行機との通信が切れた」として失敗になる

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
   - GitHub の Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token。名前 `matsu-mac-runner01`、期限 1 年、Repository access は `stofu5678/matsu_editor` だけ、Repository permissions は Administration と Variables を Read and write（Metadata の Read-only は自動で付く）
   - Mac の前で（WSL や ssh を通さずに）: `security add-generic-password -a "$USER" -s matsu-mac-runner01 -w`（2 回聞かれるので、2 回とも貼り付ける）。項目の名前は `lib.sh` の `KEYCHAIN_SERVICE`
   - 期限が切れる前に作り直したら、同じ項目を入れ替える: `security add-generic-password -U -a "$USER" -s matsu-mac-runner01 -w`
4. 型の VM を作る: `tools/mac-runner/build-golden`

## 使い方

```sh
tools/mac-runner/mac-runner up         # 使い始める
tools/mac-runner/mac-runner status     # 様子を見る
tools/mac-runner/mac-runner down       # やめる（動いているジョブは最後まで動かす）
tools/mac-runner/mac-runner down --now # すぐやめる（動いているジョブは失敗する）
```

- `up` は、最初の VM の実行機が GitHub に「online」と出てから、`MAC_RUNNER` を `self-hosted` にする。それまでの CI は GitHub の実行機で動く
- Mac の側の上限: ジョブは 3 時間、ジョブの来ない VM は 6 時間で作り直す。VM が何も答えなくなっても、9 時間で止めて消す。ディスクの空きが 120 GB（VM のディスクの 100 GB と、Mac に残す 20 GB）より少ないときは VM を作らない（複製は型と場所を分け合うが、VM の中で書いた分だけ Mac の場所を使い、最大で VM のディスクの大きさになる）。VM が動いている間に Mac の空きが 20 GB を切ったら、VM を止めて消す
- 旅行などで Mac を使えないとき: `down` しておく。忘れても、Mac が止まって 15 分たつと、CI は GitHub の実行機で動く。ただし、止まる直前に Mac を選んだジョブは待ったままになる（調整役が取り消して流し直す）
- Mac の外から切り替える: GitHub の画面（Settings → Secrets and variables → Actions → Variables の `MAC_RUNNER`）か、`gh variable set MAC_RUNNER --body github`
- ssh から使うとき: ssh のセッションでは、ログインのキーチェーンが開かない（Mac の画面にログインしていても、`security` が「User interaction is not allowed」で断る）。そのときの `up`・`down`・`status` は、自分を launchd のジョブとして、Mac の画面にログインしているセッションで動かし直し、その出力と終了コードを ssh の側に写す（トークンはそのセッションの外に出ない）。だれも画面にログインしていないと動かない。ssh の側で Ctrl+C を押しても、止まるのは写すことだけで、コマンド（`down` の止める頼みなど）はそのまま続く
- 記録: `~/Library/Application Support/matsu-mac-runner/runner.log`
- Mac の Sandbox の VM と合わせて、macOS の VM は同時に 2 つまで（利用許諾）。メモリが 16 GB なので、Sandbox を使う間は `down` しておく

## 設定を変えたとき（`ALLOWED_JOBS`、`admit.sh`）

`guest/` のファイルと設定は型の VM の中にある。変えたら型を作り直す（`build-golden`。30 分ほど）。新しい型ができて、`mac-runner` が次のジョブから使い始める。動いているジョブは古い型のまま終わる。

## パッケージ作りの道具

パッケージ作り（`scripts/package-mac-app.sh`）が使う `iconutil`・`plutil`・`codesign`・`file` は macOS に入っている（`/usr/bin`）。Rust は型の VM に入れてある。型に足す物はない。

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
security delete-generic-password -s matsu-mac-runner01
rm -rf ~/Library/Application\ Support/matsu-mac-runner
```

GitHub のトークンも、Settings → Developer settings で消す。
