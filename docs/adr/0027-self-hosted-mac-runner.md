# ADR-0027: macOS の CI を、Mac の上の VM で動く自前の実行機で動かす

- 状態: 決定（2026-11-02 に、ユーザーが #328 をマージし、「判断してほしいこと」の 8 項目を推奨どおりに決めた）
- 日付: 2026-11-02

## 背景

- CI の macOS のジョブは 3 つある。`test (macos-arm64)`（CI）と、`smoke (macos-arm64)`・`screens (macos-arm64)`（E2E）。どれも GitHub の実行機（`macos-15`）で動く
- GitHub の macOS の実行機が足りない日は、ジョブが割り当てられないまま取り消される（"The job was not acquired by Runner of type hosted even after multiple attempts"）。2026-11-01〜02 には何度も起き、調整役が流し直した（`.claude/handoff/tools/rerun-mac.sh` を作った）。アカウントで同時に動く macOS のジョブは 5 つまでで、1 つの PR が 3 つ使う
- ユーザーの Mac（Apple M1、メモリ 16 GB）には、macOS の VM を動かす Tart があり、Mac の Sandbox（`.claude/handoff/mac-sandbox.md`）で使っている
- 2026-11-02 に、ユーザーが次のように決めた: 「Mac の CI は self-hosted runner にする。VM を使い、ローカルの PC の情報がそのまま流出しないことが前提。旅行のときなどに通常の CI に戻したり、帰ってきたら self-hosted にしたり、切り替えやすくする」

## 決定（ユーザーが決めたこと）

1. macOS の CI のジョブを、ユーザーの Mac の上の VM で動く自前の実行機（GitHub Actions の self-hosted runner）で動かす
2. 実行機は VM の中だけで動かし、Mac（ホスト）の情報が外に出ないようにする
3. GitHub の実行機と自前の実行機を、簡単に切り替えられるようにする

## 案（調整役が勧める作り）

下の作りは、Codex（別の会社のモデル）に、GitHub Actions の実行機のソース、Softnet のソース、Tart のソース、Apple の利用許諾と照らして確かめさせ、指摘を入れたもの。

### 1. ジョブごとに使い捨ての VM

- 型の VM（`matsu-ci-golden`）を 1 つ作っておく。ジョブごとに型を複製して動かし（APFS の複製なので、すぐできる）、ジョブが終わったら消す。前のジョブの物は、次のジョブに残らない。使ったあとの複製を、型に戻すことはしない
- 型は、Cirrus Labs が公開している macOS の基本の VM イメージ（`macos-*-base`）に、CI の道具（rustup と固定の版の Rust、Xcode のコマンドラインツール）だけを入れて作る。Apple ID でのサインイン、ホストの鍵やトークン、個人のファイルは入れない
- 実行機の登録は、GitHub の JIT（just-in-time）の設定を使う。ホストが API で設定を作り、VM に渡す。設定は 1 つのジョブで使い切りで、ジョブが終わると登録は消える。ただし、ジョブの間は VM の中に使える資格があるので、ログに出さない
- 実行機は、VM の画面のある利用者のセッション（自動ログイン）で動かす。E2E のジョブが、画面とアクセシビリティの API を使うため
- 1 度に動かす VM は 1 つ（VM に 4 コアと 8 GB）。macOS の利用許諾は、1 台の Mac で同時に 2 つまでの macOS の VM を許す。Mac の Sandbox の VM もこの数に入るので、Sandbox を使っている間は、CI の VM は 1 つまで
- 動かしている間、ホストの Mac はスリープしないようにする（`caffeinate`）。電源につないで使う

### 2. ホストの情報が出ないようにする（VM の閉じ込め）

| 経路 | 対策 |
|---|---|
| ホストのファイル | 共有フォルダー（`tart run --dir`）、追加のディスク、機器の受け渡しを使わない |
| クリップボード | Tart は、既定でホストとクリップボードを共有する。VM を動かすたびに `--no-clipboard` を付ける |
| 家の LAN（NAS、ほかの PC、ルーター） | Softnet（Tart の通信の道具）で、VM の通信を、インターネットの宛先とホスト側の出入り口だけに絞る。私有のアドレス（192.168.x.x など）へは、既定で送れない。Softnet の「インターネットの宛先」の判定は、マルチキャスト（224.0.0.0/4。Bonjour の mDNS など）を外さないので、明示的に止める |
| ホストの Mac で動いている機能（ssh、ファイル共有、DNS など） | ホスト側の出入り口（ゲートウェイ）を止める。VM の DNS は、型で外の公開の DNS（1.1.1.1 など）にし、ホストの DNS（家の中の名前を知っている）を使わない。ホストの各インターフェースのアドレスも止める |
| 公開のアドレスを通る折り返し | 家の公開の IP アドレス（ルーターが LAN の機器へ転送しているかもしれない）は、VM を動かすたびに調べて止める。ホストが VPN で社内などの網につないでいる間は、実行機を動かさない（または、その網のアドレスを止める） |
| IPv6 | Softnet は IPv4 と ARP のほかの通信を通さない（Codex が Softnet のソースで確かめた）。Softnet を使わない場合は、pf で IPv6 も止める |
| ホストと VM の連絡 | ネットワークではなく、Tart の `tart exec`（VM の中の guest agent）を使う。VM から返る文字は信用せず、ホストのシェルで評価しない |
| GitHub の資格 | ホストが持つトークン（下の 4）は、VM に入れない。VM に渡すのは、1 つのジョブ用の JIT の設定と、ジョブごとの `GITHUB_TOKEN` だけ |
| ジョブのログと成果物 | 公開のリポジトリなので、だれでも読める。VM には個人の情報がないので、ログや画面写真に出るのは VM の中のことだけ。VM で作った成果物は信用しない（リリースの作成などで使わない） |
| キャッシュ | VM のジョブのキャッシュは、GitHub の実行機のリリースのジョブ（`package.yml` など。いま E2E と同じ `shared-key: release` を使う）と名前を分け、VM のジョブが書いたキャッシュをリリースが読まないようにする |
| 前のジョブの残り | VM はジョブごとに消す。ジョブの終わり、取り消し、失敗、下の 3 の拒否、時間切れのどれでも、ホストが必ず止めて消す |

守れないもの:

- インターネットへの通信は、家の回線から出る（相手からは、家の IP アドレスに見える）。だから、信頼できるジョブだけを動かす（次の 3）
- VM の外へ抜け出す macOS の仮想化の不具合は、VM では防げない。ホストの macOS と Tart を新しく保つ

### 3. 動かすジョブを、このリポジトリのブランチの、決めたジョブに限る

公開のリポジトリでは、フォークからの PR が、ワークフローを書き換えて自前の実行機を指定できる。GitHub も、公開のリポジトリで自前の実行機を使う危険として挙げている。次の 4 つで防ぐ。

1. VM の中の「ジョブの前の確かめ」（実行機の `ACTIONS_RUNNER_HOOK_JOB_STARTED`）。実行機の設定で決まるので、ワークフローからは変えられない
   - 確かめること（どれかが欠けたり読めなかったりしたら、拒否する）:
     - リポジトリの ID が、このリポジトリのもの
     - イベントが、main（`refs/heads/main`）への `push`、または、`pull_request` で PR の元のリポジトリの ID（`pull_request.head.repo.id`）がこのリポジトリのもの
     - ワークフローが `ci.yml` か `e2e.yml`（`GITHUB_WORKFLOW_REF`）で、ジョブが `test`、`smoke`、`screens` のどれか（`GITHUB_JOB`）。リリースのワークフロー（書き込みの権限とトークンを持つ）は来られない
   - イベントの JSON は、シェルで評価せずに読む（PR の題やブランチの名前を、コマンドとして扱わない）
   - 拒否するときは、終了コードで失敗にするだけでなく、その場で VM の電源を切る。確かめが失敗してもジョブは「失敗」になるだけで、`if: always()` などを付けた後ろの手順は動くため（Codex が実行機のソースで確かめた）。ホストは VM が止まったのを見て消す
2. ワークフローの選び方: フォークからの PR は、いつも GitHub の実行機を選ぶ。フォークの PR のワークフローには、リポジトリの変数が渡らないので、変数がないときは GitHub の実行機にする。これは便利のためで、守りは上の 1
3. リポジトリの設定: Settings → Actions → General で「Require approval for all external contributors」（外部の協力者のすべてに承認を求める）にする
4. 自前の実行機を選べるジョブに、`pull_request_target` を使わない（承認を飛び越えるため）

いまブランチを push できるのは、ユーザーのアカウントだけ（調整役と担当も、このアカウントで push する）。

### 4. ホストが持つ資格

- ホストの Mac は、JIT の設定を作り、切り替えの変数を書くために、GitHub の fine-grained personal access token を 1 つ持つ。対象はこのリポジトリだけ。権限は Administration（読み書き。実行機の登録に要る）と Variables（読み書き）だけ。期限は 1 年
- トークンは、ユーザーが作って Mac のキーチェーンに入れる。スクリプトはキーチェーンから読み、ファイルにもログにも書かない。VM には入れない

### 5. 切り替え

- リポジトリの変数（Actions variables）`MAC_RUNNER` で選ぶ
  - `self-hosted`: macOS のジョブを自前の実行機で動かす
  - それ以外（`github`、または無し）: GitHub の実行機（`macos-15`）で動かす
- Mac のスクリプト `tools/mac-runner/mac-runner`
  - `mac-runner up`: 実行機を動かし、`MAC_RUNNER` を `self-hosted` にする
  - `mac-runner down`: 先に `MAC_RUNNER` を `github` にし、新しい VM を作るのをやめ、動いているジョブが終わったら止める
  - `mac-runner status`: 変数、VM、登録されている実行機を見せる
- GitHub の画面（Settings → Secrets and variables → Actions → Variables）や、どの PC の `gh variable set MAC_RUNNER --body github` でも切り替えられる。調整役に頼んでもよい
- 切り替えを忘れたときの備え: 実行機が動いている間、ホストは 5 分ごとに変数 `MAC_RUNNER_SEEN`（時刻）を書く。ワークフローの最初の小さなジョブ（Linux）は、`MAC_RUNNER` が `self-hosted` でも、`MAC_RUNNER_SEEN` が 15 分より古い、無い、読めない、未来の時刻のときは、GitHub の実行機を選ぶ。Mac を閉じて出かけても、15 分後からの CI は GitHub の実行機で動く
  - 順番を待った run（`release.yml`）は、push したコミットの時刻から見て 15 分以内に書かれていれば、自前の実行機を選ぶ（GitHub が作ったコミット（マージ）で、作られてから 2 時間以内の run で、心拍がコミットより 1 分以上新しくないときに限る）。run が読む変数は、run が作られたときの値のままのため（2026-11-08 の追記）
  - 残る穴: Mac が止まる直前に自前の実行機を選んだジョブと、順番を待っている間に Mac が止まった run のジョブは、待ち続ける。調整役が見つけて、取り消して流し直す（Mac を開けば、そのまま続く）

### 6. 移すジョブ

| ジョブ | 自前の実行機 |
|---|---|
| `test (macos-arm64)`（CI） | 第 1 段で移す（画面収録などの許可が要らない） |
| `smoke (macos-arm64)`、`screens (macos-arm64)`（E2E） | 第 2 段で移す。VM の型で、実行機に画面収録とアクセシビリティの許可を与えてから |
| `package.yml`（リリースの成果物）、`release.yml` | 移さない（2026-11-06 の追記で移すことにした。下を見る） |
| `mac-check.yml`（手で動かすビルド） | 移さない |

### 7. ホストが守る上限

- 1 つのジョブの時間の上限（例: 3 時間）を、ホストが VM ごとに数える。過ぎたら VM を止めて消す
- 複製の前に、ホストのディスクの空きを確かめる。VM のディスクの大きさは型で決まっているので、ジョブがあふれさせても VM の中だけで済む

### 8. 置く物と道具

- `tools/mac-runner/`: ホストのスクリプト（`mac-runner`、型の VM を作る手順）、VM の中の「ジョブの前の確かめ」、README
- `.github/workflows/`: 実行機を選ぶ小さなジョブと、macOS のジョブの `runs-on`、キャッシュの名前の分け方
- 道具のライセンス: Tart と Softnet は FSL-1.1-ALv2（競合する商用の使い方のほかは自由に使える。それぞれの版が出てから 2 年後に Apache-2.0 になる）。MATSU には入らず、ユーザーの Mac で動かす道具として使うだけ（Tart は Mac の Sandbox で使っている）

## 検討した案

- 調整役が Mac に ssh でジョブを流し、GitHub の macOS のジョブは main への push だけにする: ユーザーが self-hosted runner を選んだ
- 既製の管理ソフト（Tartelet、Cilicon など）: GitHub の資格を持つ別のソフトを、ホストで信頼することになる。小さなスクリプトなら、何をするかを読める
- VM を使わず、ホストで実行機を動かす: ホストの情報が出ないという前提に合わない
- ホストの資格を、personal access token ではなく GitHub App にする（1 時間で切れるトークン）: App の秘密鍵をホストに置くことになり、守る物は同じ。始めは token にし、必要なら後で替える

## 判断してほしいこと

1. 上の 4 のトークン（このリポジトリだけ、Administration と Variables の読み書き、期限 1 年）を作って、Mac のキーチェーンに入れること
2. 上の 3 の 3 のリポジトリの設定を、「Require approval for all external contributors」にすること
3. Mac の設定を変えること: Softnet を root の権限で起動できるようにする（Softnet だけの SUID か、パスワードなしの sudo）。Softnet で足りない所は、ホストの防火壁（pf）に規則を足す
4. Softnet は macOS 26 以降が要る。Mac が古いときは、macOS を更新するか、Softnet を使わず pf だけで止めるか（pf だけのときは IPv6 も止める）
5. VM の DNS を、外の公開の DNS（例: 1.1.1.1）にすること。VM が引く名前（github.com、crates.io など）は、その DNS の運営者に見える
6. 実行機を動かしている間は、Mac で、社内などの網につなぐ VPN を使わないこと
7. 第 1 段（`test (macos-arm64)` だけ）から始め、E2E は第 2 段にすること
8. 1 度に動かす VM は 1 つから始めること

## 決めたこと（2026-11-02）

上の「判断してほしいこと」の 8 項目は、すべて推奨どおり。決めたあとに分かったことと、済んだこと。

- 2: リポジトリの設定を「Require approval for all external contributors」にした（調整役が API で設定。`GET /repos/stofu5678/matsu_editor/actions/permissions/fork-pr-contributor-approval` が `all_external_contributors` を返す）
- 4: ユーザーの Mac は macOS 27.0.1（`.claude/handoff/mac-sandbox.md` の記録）なので、Softnet の条件（macOS 26 以降）を満たす。macOS の更新は要らない
- 1・3: ユーザーの作業（トークンを作ってキーチェーンに入れる、Softnet を root で動かす設定）は、Mac で確かめてから行う

## 調整役が確かめること（Mac がつながってから）

- VM から、家の LAN、ホスト（出入り口とインターフェースのアドレス、ssh）、家の公開の IP アドレス、マルチキャスト（Bonjour で LAN の機器が見えないこと）に届かず、GitHub と crates.io には届くこと
- クリップボードがホストと共有されていないこと
- `tart exec` で JIT の設定を渡せること（ネットワークを使わない）
- 型の VM が自動ログインし、画面のある利用者のセッションで実行機が動くこと
- 「ジョブの前の確かめ」が、フォークからの PR、ほかのワークフロー、ほかのジョブを拒否し、`if: always()` の手順も動かないうちに VM が止まること
- 時間切れと取り消しで、ホストが VM を消すこと
- 1 つのジョブにかかる時間（GitHub の実行機と比べる）、Mac のメモリとディスクの余り

## 追記（2026-11-06）: パッケージ作りも自前の実行機で動かす

「移すジョブ」の表（上の 6）は、`package.yml` を移さないとしていた。次の理由で、これを変える。ユーザーが 2026-11-06 に「パッケージづくりのジョブもMacの実行機に移しましょう」と決めた。上の E2E の「第 2 段」とは別の話で、ここではパッケージ作りの追加と呼ぶ。

### 理由

- GitHub の macOS の実行機が足りない日が続き、パッケージ作りの macOS のジョブが 30 分待って取り消された（手順は 1 つも動かない）。v0.7.0 の Release では `package-release / package (macos-arm64)` が取り消され、`release` のジョブが飛んで、v0.7.0 にパッケージが 1 つも載らなかった（11/5 に流し直して直した）。nightly、Package check、mac-check も何度も取り消された

### 決めること

| ジョブ | 自前の実行機 | イベント |
|---|---|---|
| `release.yml` の `package`（nightly）と `package-release`（バージョン）の `package (macos-arm64)` | 移す | main への `push` だけ |
| `package-check.yml` の `package (macos-arm64)` | 移す | 同じリポジトリからの `pull_request` だけ |
| `mac-check.yml` | 移さない | － |

- 実行機の選び方は、CI の `test (macos-arm64)` と同じ決まり（`MAC_RUNNER`、`MAC_RUNNER_SEEN`、イベント、リポジトリの ID。欠けたら GitHub の `macos-15`）。同じシェルを 2 か所に写さないよう、`.github/actions/mac-runner/action.yml`（composite action）に 1 つだけ置き、`ci.yml` の `changes` の手順と `package.yml` の `mac-runner` のジョブが使う
- `package.yml` は、`runs-on` が同じジョブの手順の出力を読めないので、選ぶだけの小さなジョブ（Linux、数秒）を先に動かす。macOS の行だけがその出力を使い、ほかの 4 つの行は今のまま
- `package.yml` に渡す secrets はない（`release.yml` は `with:` だけを渡す）。VM のジョブの `GITHUB_TOKEN` は読み取りだけ（`permissions: contents: read`）。リリースの公開（書き込みの権限を持つ `nightly`・`release` のジョブ）は、今までどおり GitHub の実行機（Linux）で動き、VM には来ない
- VM の中の「ジョブの前の確かめ」は、上の 3 の 1 の決まりのまま、通すジョブの表を広げる。`ALLOW` の行に、イベントを付けられるようにした（`ALLOW=release.yml:package:push` は push だけ）。再利用のワークフローの中のジョブは、`GITHUB_WORKFLOW_REF` が呼んだ側のワークフロー（`release.yml`、`package-check.yml`）、`GITHUB_JOB` が再利用のワークフローの中の id（`package`）になる、と見て、`release.yml:package:push`、`release.yml:package-release:push`（`GITHUB_JOB` が呼んだ側の id になる場合の備え。どちらになるかは、実際に動かして確かめる。断られたときのログに、ファイル・ジョブ・イベントが出る）、`package-check.yml:package:pull_request` を通す。ほかの `release.yml` のジョブ（`release-please`・`nightly`・`release`）は通さない
- VM のジョブのキャッシュは、GitHub の実行機の `release` と分けて `release-vm` にする（上の 2 の「キャッシュ」）

### 変わる前提

- 上の 2 の「ジョブのログと成果物」は、「VM で作った成果物は信用しない（リリースの作成などで使わない）」としていた。この追記で、リリースに載るパッケージ（macOS の `.tar.gz`）を VM で作る。守りは、VM が使い捨てで、secrets がなく、トークンが読み取りだけで、動かすのが main への push と同じリポジトリの PR だけ（上の 3）、という点にある。VM の中のジョブが書き換えた成果物は、公開のリポジトリの Release に載る
- 1 度に動かす VM は 1 つなので、`test` とパッケージ作りは順番を待つ

### 移さないもの

- `mac-check.yml`: `workflow_dispatch` で、任意のスクリプトの道を受け取る。動かせるのは書き込みの権限のある人だけだが、「ジョブの前の確かめ」の決まり（main への push か、同じリポジトリの PR）に合わない `workflow_dispatch` を通す必要があり、ブランチの任意のコードを VM で動かす口になる。移す価値（手で動かすので待ちが苦にならない）より危うさが大きいので、移さない

## 追記（2026-11-08）: 順番を待った run は、作られたときの心拍で決める

### 背景

- 2026-11-06 に、`release.yml` の run（#364 のマージ。20:48 に作られた）が、前の run（apt が 6 時間止まった）の後ろで、`release` のグループで 30 分ほど待った。21:19 に動いた選ぶジョブは、`MAC_RUNNER_SEEN` を 2030 秒前（20:45）の値として読み、GitHub の `macos-15` を選んだ。空きがなく 15 分で取り消され、nightly が出なかった
- 待っている間も、ホストは 5 分ごとに `MAC_RUNNER_SEEN` を書いていた。run が読む変数（`vars`）は、run が作られたときの値のまま変わらない（この run で見た。待つ間に書かれた値は読まれなかった）
- `release.yml` は、前の run が終わるまで次の run を待たせる（`cancel-in-progress: false`）。main へのマージ 1 回で、VM には CI の `test`（15 分ほど）と Release の `package`（7 分ほど）が並ぶ。続けてマージすると、次の run が 15 分を超えて待つことがある。版のリリースで起きると、その版にパッケージが付かない（v0.7.0 と同じ）

### 決めたこと

ユーザーが 2026-11-08 に、案 A を選んだ（「案Aで進めてください」）。

- 選ぶジョブは、`MAC_RUNNER_SEEN` が今から見て 15 分より古くても、push したコミットの時刻（`github.event.head_commit.timestamp`）から見て 15 分以内なら、自前の実行機を選ぶ。squash マージのコミットの時刻は、run が作られる 3〜5 秒前（11/6 の 3 つの run で見た）なので、「run が作られたときに Mac が動いていた」という意味になる
- この決まりは、GitHub が作ったコミット（committer が `noreply@github.com`。GitHub の上のマージ）にだけ使う。ほかのコミットは、作った人が日付を決められ、push よりずっと古いことがある。main には、もともと GitHub の squash マージしか入らない（リポジトリの設定で、管理者にも保護をかけている）が、その前提をスクリプトでも確かめる
- ただし、コミットの時刻が今から 2 時間より前なら、この決まりは使わない。どのジョブにも 1 時間までの上限があるので、それより長い待ちはふつうでなく、Mac が閉じられているかもしれない
- 心拍がコミットの時刻より 1 分以上新しいときも、この決まりは使わない。run が作られたときに読んだ値なら、心拍がコミットより新しいことはない（多くて数秒）。新しいのは、後から読んだ値（再実行など）か、コミットが run より古いときで、どちらも今から見た 15 分の決まりで決める
- 再実行（Re-run）で GitHub が変数を読み直すかどうかは、確かめていない。読み直すなら、マージのあとに Mac が止まった再実行は、上の 1 分の決まりで GitHub の実行機を選ぶ。読み直さない（作られたときの値のまま）なら、マージから 2 時間以内の再実行は、待った run と同じに扱われる（下の残る穴と同じ）
- GitHub の実行機に回したときは、ログに、この決まりが使えなかった理由（コミットを作ったのが GitHub でない、時刻の形、2 時間、心拍が古すぎる・新しすぎる）を書く
- コミットの時刻は、ISO 8601 の形（時刻帯つき）だけを受け取る。それ以外は無いものとし、今までどおり GitHub の実行機を選ぶ
- pull request の run（コミットの時刻が無い）と、待たなかった run は、今までと同じ。CI と Package check は、新しい run が古い run を取り消すので、順番を待たない
- 決まりは `.github/actions/mac-runner/pick.sh` に移し、`pick-test.sh` で確かめる（CI の lint）。新しい権限は要らない

### 残る穴

- 順番を待っている間に Mac が止まった（閉じた、眠った）ときは、その run の macOS のジョブが Mac を待ち続ける。GitHub は、自前の実行機を最長 24 時間待つ。Mac を開けば、そのまま続く。待つ間は、次の Release の run も待つ。再実行で変数が読み直されない場合の、マージから 2 時間以内の再実行も同じ

### 検討した案

- 案 B: 待ちそのものを減らす。main への push の CI の `test (macos-arm64)` を GitHub の実行機に回し、VM を Release と PR に空ける。main での macOS のテストが GitHub の空きしだいになり、古い値で決めること自体は残るので、採らなかった
- 案 C: 今のままにする。起きたら流し直す。ただし「失敗したジョブだけ再実行」では、選ぶジョブの前の答え（GitHub）がそのまま使われる。版のリリースで起きたときの戻し方が別に要るので、採らなかった
