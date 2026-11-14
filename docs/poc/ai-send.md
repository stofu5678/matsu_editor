# 下調べ: 右クリックで生成 AI に送る（ADR-0030 用）

調べた日: 2026-11-13。読んだのは公式文書とヘルプだけ。ログインも、プロンプトの送信もしていない。GPL 系のソースは読んでいない。
印の意味: 【確認】は文書かローカルの `--help` で確かめた。【未確認】は読んだ文書に書かれていなかった。【推測】は私の推測。

## 1. コマンドライン AI ツール

ローカルで `--help` を見たのは claude 2.1.323 と codex-cli 0.160.0 だけ。gemini、copilot、llm、ollama は WSL に入っていない。

### 1.1 一覧表

| | Claude Code | Codex CLI | Gemini CLI | Copilot CLI | `llm` | Ollama |
|---|---|---|---|---|---|---|
| 非対話 | `claude -p` | `codex exec` | `gemini -p` | `copilot -p` | `llm 'prompt'` | `ollama run MODEL "prompt"` |
| 指示 | 引数 | 引数、または `-` で stdin | 引数 (`-p`) | 引数 (`-p`) | 引数、`-s` で system | 引数 |
| 本文 (stdin) | 可。10 MB まで | 可。指示の引数があると、stdin は `<stdin>` ブロックとして足される【ローカル help】 | 可。`-p` の文字は stdin の後ろに足される | `-p` があると stdin は無視される | 可。stdin のあとに引数が続く | 可（文書の例は埋め込みモデルだけ）【生成モデルは未確認】 |
| 出力 | `text` / `json` / `stream-json` | 最終メッセージだけ stdout。`--json` で JSONL。`-o FILE` | `text` / `json` / `stream-json` | `-s` で付随情報を消す。`--output-format` は別のサブコマンドの文書だけ【未確認】 | 既定は流し出力。`--no-stream`、`--json`、`-x`（コードブロックだけ） | 文字。`--format json` は【未確認】 |
| 終了コード | 0 と非 0。SIGTERM は 143 | 文書になし【未確認】 | 0 / 1 / 42（入力）/ 53（ターン上限） | 文書になし【未確認】 | 文書になし【未確認】 | 文書になし【未確認】 |
| 続き | `--continue`、`--resume <id>`（id は json の `session_id`） | `codex exec resume --last` / `<SESSION_ID>` | `--resume`（最新か番号） | `--continue` / `--resume`【選択画面の説明だけ】 | `-c` / `--cid ID` | 【未確認】 |
| 認証 | ログイン (OAuth)、`ANTHROPIC_API_KEY`、Bedrock など | ChatGPT ログイン、`CODEX_API_KEY` | Google ログイン、`GEMINI_API_KEY`、Vertex | `/login`、`COPILOT_GITHUB_TOKEN` / `GH_TOKEN` / `GITHUB_TOKEN` | `llm keys set`、プラグインごと | なし（ローカル）。クラウドのモデルはアカウント |
| OS | Win10 1809+、macOS 13+、Ubuntu 20.04+ ほか。Windows は素のままで可（Git for Windows は任意） | Windows、macOS、Linux (arm64/x86_64) | macOS と Linux の導入手順は確認。Windows は【未確認】 | Linux、macOS、Windows（PowerShell 6 以上） | Python 製。【推測: 3 つとも動く】 | macOS、Windows、Linux |
| ライセンス | 独自。Anthropic の利用規約が適用される（OSS ではない） | Apache-2.0 | Apache-2.0 | LICENSE.md としか分からない【未確認】 | Apache-2.0【今回は文書で見ていない。未確認】 | MIT |

出典:
- Claude: https://code.claude.com/docs/en/headless 、https://code.claude.com/docs/en/setup 、https://code.claude.com/docs/en/legal-and-compliance 、`claude --help`
- Codex: https://learn.chatgpt.com/docs/non-interactive-mode （developers.openai.com/codex/noninteractive から転送）、https://github.com/openai/codex 、`codex exec --help`。ローカルの npm パッケージの `package.json` に `"license": "Apache-2.0"`
- Gemini: https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/headless.md 、同 `docs/cli/cli-reference.md`、https://github.com/google-gemini/gemini-cli
- Copilot: https://docs.github.com/en/copilot/how-tos/copilot-cli/automate-copilot-cli/run-cli-programmatically 、https://github.com/github/copilot-cli
- llm: https://llm.datasette.io/en/stable/usage.html
- Ollama: https://docs.ollama.com/cli 、https://docs.ollama.com/faq 、https://github.com/ollama/ollama

気づいたこと:
- Gemini CLI の README と tos-privacy ページに、「個人の無料利用と Google AI Pro/Ultra は 2026-06-18 に Antigravity CLI に移った」とある。個人が Google ログインで Gemini CLI を使う道は、今はないかもしれない【文書の記述。詳細は未確認】。
- Claude Code の文書は、失敗（認証がないなど）を「結果として stdout に出す」と書く。終了コードを見ずに stdout を挿すと、エラー文を答えとして挿してしまう。
- Copilot CLI は `-p` と stdin を同時に使えない。本文を引数に入れるしかなく、引数は他のプロセスから見え（`ps` など）、長さにも上限がある（Windows の `CreateProcess` は 32,767 文字【推測: 一般知識】）。
- Codex CLI は Git リポジトリの外では `--skip-git-repo-check` が要る。

### 1.2 ツールと権限を切る（プロンプト・インジェクション対策）

文書の本文が「ファイルを消せ」「このコマンドを実行せよ」と書いていても、ツールが動かないようにしたい。

| ツール | 方法 | 文書が約束していること |
|---|---|---|
| Claude Code | `--tools ""`（組み込みのツールを全部無効にする。`claude --help` の文: `Use "" to disable all tools`）。重ねて `--permission-mode dontAsk --permission-prompts none`（聞かれる操作は全部拒否）。`--disallowedTools`、`--allowedTools` もある。`--restricted` は、コマンドを実行するツールと WebFetch を外し、ユーザー・プロジェクトの設定ファイルを無視する（help の文）。MCP は `--strict-mcp-config` で `--mcp-config` のものだけにできる | 権限モードの説明はある。「`--tools ""` で攻撃が防げる」とまでは書いていない。`--bare` を付けないと、`-p` でもプロジェクトの hooks と `.mcp.json` が、信頼の確認なしで動く（headless 文書）。`--bare` は OAuth を読まず `ANTHROPIC_API_KEY` だけになる（ログインで使う人には使えない） |
| Codex CLI | `--sandbox read-only`（`codex exec` の既定）。`--ignore-user-config`、`--ignore-rules`、`--ephemeral`。ツールを全部切るフラグは【未確認】 | 既定のネットワークは「切」（agent-approvals-security の文）。モデルのシェル実行は read-only でも読み取りはできる【推測】。文書は `Prompt injection can cause the agent to fetch and follow untrusted instructions` と書く |
| Gemini CLI | `--approval-mode`（`default` / `auto_edit` / `yolo` / `plan`）、`--sandbox`、`--allowed-tools`（廃止予定。Policy Engine へ） | ツールを全部切る方法は、読んだページにない【未確認】。`plan` が読み取り専用かも【未確認】 |
| Copilot CLI | `--available-tools`（許可リスト。それ以外をモデルが使えない）、`--excluded-tools`、`--deny-tool`、`--allow-tool`、`--no-ask-user`、`--assisted-approval`。許可リストと除外リストを両方渡すと、許可リストが勝つ。deny は `--allow-all` にも勝つ（検索結果の要約。公式ページの本文は一部しか読めていない） | 空の一覧で全部切れるか【未確認】。`--allow-all` はサンドボックスの外で避けるよう警告がある |
| `llm` | 既定ではツールを使わない【推測。`-T` を足したときだけ】 | 【未確認】 |
| Ollama | `ollama run` はモデルと会話するだけで、ツールを実行しない【推測】 | 【未確認】 |

どのツールの文書も「完全に防げる」とは約束していない。Claude の security ページは `no system is completely immune` と書く。

### 1.3 提供元のデータの扱い（この経路の分）

| 提供元 | 要点 | リンク |
|---|---|---|
| Anthropic (Claude Code) | 個人プラン（Free/Pro/Max/Ultra）: 学習への利用を設定で選ぶ。許可すると保持 5 年、許可しないと 30 日。法人（Team/Enterprise/API）: 学習に使わない。保持は標準 30 日。ZDR は契約者だけ。ローカルに会話の記録が平文で `~/.claude/projects/` に 30 日残る（`--no-session-persistence` で残さない）。テレメトリは `DISABLE_TELEMETRY=1` などで切れ、メトリクスに本文は含まない | https://code.claude.com/docs/en/data-usage 、https://www.anthropic.com/legal/commercial-terms 、https://www.anthropic.com/legal/consumer-terms 、https://www.anthropic.com/legal/privacy |
| OpenAI (Codex) | API の既定: 不正利用の監視ログを最大 30 日。学習には使わない（オプトインのときだけ）。ZDR は承認制。ChatGPT ログイン経由の学習・保持は読めていない【未確認】。Codex の OpenTelemetry は既定で切。`log_user_prompt = false` を勧めている | https://developers.openai.com/api/docs/guides/your-data 、https://learn.chatgpt.com/docs/agent-approvals-security 、https://openai.com/policies |
| Google (Gemini CLI) | 認証ごとに規約が違う。無料の API キー: 入力と出力を製品の改善に使い、人が読むことがある。「機密や個人の情報を送るな」と書いてある。有料の API キー: プロンプトを改善に使わない。ログは「限られた期間」。Google ログインの個人向けと法人向けは別の通知 | https://www.geminicli.com/docs/resources/tos-privacy 、https://aistudio.google.com/docs/terms （ai.google.dev/gemini-api/terms から転送）、https://developers.google.com/gemini-code-assist/resources/privacy-notices |
| GitHub (Copilot CLI) | CLI 専用の保持期間は、公式では見つからなかった【未確認】。Trust Center は、Copilot for Business はプロンプトを学習に使わないと述べる。CLI の保持を 28 日とするのはコミュニティの書き込みで、公式ではない | https://resources.github.com/copilot-trust-center 、https://docs.github.com/site-policy/privacy-policies/github-privacy-statement |
| `llm` | 送り先はプラグインのモデル次第。既定で、プロンプトと答えを手元の SQLite に平文で保存する（`llm logs`）。`-n/--no-log` で保存しない | https://llm.datasette.io/en/stable/usage.html |
| Ollama | ローカル実行: 「Ollama はプロンプトを見ない」。127.0.0.1:11434 だけで待つ（`OLLAMA_HOST` で変わる）。クラウドのモデルは、プロンプトを Ollama に送る（保存・ログ・学習はしないと文書にある）。`OLLAMA_NO_CLOUD=1` でクラウドを全部切れる | https://docs.ollama.com/faq |

## 2. Smalltalk のワークスペース

【確認】Pharo のワイキ https://github.com/pharo-open-documentation/pharo-wiki/blob/master/General/Playground.md と、検索で出た Pharo by Example の Playground の説明。

- Do it (Ctrl/Cmd+D): 選んだ式を実行する。結果は見せない。見た目の変化もない。
- Print it (Ctrl/Cmd+P): 実行して、結果の `printString` を出す。Pharo の Playground は、結果を横に表示する（文書の「on the side」）。Pharo の開発者の書き込みでは、文書には入れず、別に出す小さな枠で、別の場所をクリックすると消える。
- Inspect it (Ctrl/Cmd+I): 実行して、結果のオブジェクトのインスペクタを Playground の外に開く。
- Do it and go (Ctrl/Cmd+G): インスペクタを横に開く。
- Squeak: Print it は、選択のすぐ後ろに結果をそのまま入れる（Squeak-dev の議論: `6 * 7` が `6 * 7 42` になる。https://lists.squeakfoundation.org/pipermail/squeak-dev/2020-January/206415.html ）。入れた結果が選択されるかは、読んだ範囲に書かれていない【未確認】。

「smalltalk の inspect のように使える」の読み方:
1. 選んだ文字を、その場で評価して結果を見る。文書は選択の直後以外に変わらず、結果は別の場所（インスペクタ）で見る。
2. 気軽に試せて、結果が気に入らなければ捨てて、もう一度やれる。Print it は文書に入れるが、取り消せる。
3. 結果をまた選んで、次の入力にできる（連鎖）。結果が文書に入っていると、これがそのまま効く。
4. 【推測】ユーザーの文は「次の行に送り返す」なので、Print it に近い。Inspect it は「別タブで見る」側に当たる。

エディタにするなら、出力先 3 つが Smalltalk の 3 つに対応する。結果を捨てる（Do it）、次の行に入れる（Print it）、ログのタブで見る（Inspect it）。

## 3. ほかのエディタ

文書だけを読んだ。GPL のコードは読んでいない。

| | 答えの行き先 | 別の画面で、質問と答えが見えるか | 送る前に、送る文字が見えるか |
|---|---|---|---|
| VS Code + Copilot インライン・チャット | 差分としてエディタに出る。Keep か Undo で決める | ⌘I で「チャットに聞く」に切り替わる設定がある。Quick Chat と Chat ビューもある | 打ったプロンプトは見える。選択範囲の文字をそのまま見せる説明は文書にない |
| VS Code の Claude Code 拡張 | 編集は横並びの差分で出て、許可を聞く（手動のモード）。差分の中で直してから受け入れられる | 専用のパネル（会話ごとに履歴） | 選択は自動で共有される。行数とファイル名がフッターに出る。X で外せる。`claudeCode.attachSelection` で切れる。選択の全文のプレビューは文書にない。`files.exclude` や gitignore のファイルは、パスだけ送る |
| JetBrains AI Assistant | チャットでは提案を見て、自分で適用する。エージェントは複数ファイルを直し、受け入れか破棄を選ぶ | AI Chat のツール窓。エディタのタブにもできる | 添付ボタンで文脈を足す。送る前の表示は、読んだページにない【未確認】 |
| Emacs gptel | 選択に対して「rewrite」「refactor」「fill」ができる | 専用のチャットのバッファ【検索結果の要約から。ページ自体は未読】 | `gptel-send` の dry-run で、送る内容を全部見られる（NonGNU ELPA の記述と変更履歴）。rewrite の dry-run の挙動は【未確認】 |
| Neovim（CodeCompanion など） | 【未確認】文書のページが 404 だった | 【未確認】 | 【未確認】 |
| Zed のインライン・アシスタント | 選択（なければ現在の行）を、答えで置き換える（ctrl-enter） | 別のスレッドは Agent パネルで、インラインとは別。`@thread` で参照できる | 文書にない。キーバインドでプロンプトを先に入れておくことはできる |

出典: https://code.visualstudio.com/docs/copilot/chat/inline-chat 、https://code.claude.com/docs/en/vs-code 、https://www.jetbrains.com/help/ai-assistant/ai-chat.html 、https://elpa.nongnu.org/nongnu/gptel.html 、https://zed.dev/docs/ai/inline-assistant 。

読み取れる傾向:
- 差分で見せてから受け入れる（VS Code、JetBrains）が主流。Zed は置き換えて、取り消しに任せる。
- 送る内容を事前に見せるのは、gptel の dry-run だけが確かめられた。
- 答えを入れる先は「置き換え」か「差分」が多く、「次の行」は少ない。

## 4. MATSU のコードで分かったこと

- 外部コマンド実行（`crates/matsu-ui/src/exec/`、`crates/matsu-platform/src/exec/mod.rs`）: Windows は `cmd.exe /C`、macOS と Linux は `/bin/sh -c`。標準出力と標準エラーは同じ 1 本の流れに混ざる（Sakura に合わせた作り）。stdin はバイト列のチャンネルで、閉じると終端になる。中断は子の全プロセスを終わらせる。出力は 1 回の取り消しで入り、選択があったときは入れた文字が選択される。
  - 影響: AI ツールが標準エラーに警告や進捗を出すと、答えに混ざる。AI 用には、標準エラーを分ける作りが要る。Claude は `--output-format json` の `result` で取れるが、JSON を読む部品が要る。Codex の `-o FILE` は、ファイルを書く。
- 端末（`crates/matsu-terminal/src/terminal.rs`）: `write(&self, bytes)` で端末に書ける。`encode_paste` は、括弧付き貼り付けを使い、ESC などの制御文字を除き、括弧付き貼り付けを求めないプログラムへの複数行は `needs_confirmation` にする。`read_lines(first, count)` で、スクロールバックを含む格子を読める。`TerminalEvent::ChildExit { code }` で、子の終了と終了コードが分かる。`Options.shell` で、起動するプログラムを選べる。
- ADR-0029 は、このリポジトリにまだない。「ワンライナーはシェルのタブのシェルで動く」の意味が、同じシェルのプログラム（`-c` で起動）なのか、動いている対話セッションなのかは、私の見た資料では決まらない【未確認。メインに確認】。

## 5. 作り方の比較

### (a) AI は、ワンライナーの一種。バックグラウンドで実行し、プロンプトと答えを読み取り専用のタブに記録する

- 実行: 今の `exec` のパイプ（端末を通さない）。stdin は選択か文書全体。指示は、設定に書いた文（例: `claude -p --tools "" "次の文を要約せよ"`）。
- 答え: プロセスの標準出力。「次の行に挿入」は ADR-0029 の出力先の 1 つで足りる。
- ログのタブ: コマンド、指示、stdin の大きさ、答え、終了コードを、メモリの中だけで持つ読み取り専用のタブ。ファイルには書かない。
- 「裏でシェルが動き、別タブでプロンプトも確認」: 別タブでプロンプトと答えを見ることは満たす。シェルのタブの中で動くわけではないので、文字どおりには満たさない。
- ADR-0028 の案 4（決めたこと 2）: 端末に文字を送らないので、そもそも関わらない。マクロとプラグインに口を作らない点も保てる。ADR-0028 の案 5（決めたこと 3）: ログをメモリだけにする限り、衝突しない。
- 良い点: 答えを正確に取れる。送る前に、送る文字（指示と本文）を見せる確認が作りやすい。ツールを切るフラグを MATSU が固定できる。シェルの履歴に本文が残らない。
- 弱い点: 標準エラーが混ざる。続きの会話は、ツールごとのセッション ID の扱いが要る。

### (b) 対話の AI ツールをシェルのタブで動かし、MATSU が、見せたあとでプロンプトを打ち込む

- 打ち込み: `Terminal::write` と `encode_paste`。ADR-0028 の案 4（決めたこと 2） の「利用者が選んだ MATSU 自身のコマンド」に当たる。送る文字を先に見せる。
- 答えを文書へ戻す: `read_lines` で格子を読むしかない。TUI の画面は、折り返し、罫線、色、書き直し（カーソル移動）、代替画面、同期出力で、答えの本文と飾りが混ざる。画面から出た部分は取れず、代替画面のあいだはスクロールバックにも残らないことがある。正確な本文を取るのは当てにならない【推測。ADR-0028 の調べに、これらのツールが代替画面などを使うとある】。
- 答えが終わったことを知る方法がない（OSC 133 は作らない決め）。対話でツールが動く状態は、プロンプト・インジェクションに最も弱い。
- 「裏でシェルが動き、別タブで確認」: 文字どおりに満たす。
- ADR-0028 の案 4（決めたこと 2）: 満たす（見せてから送る）。ADR-0028 の案 5（決めたこと 3）: 出力を読むだけで、書かない。
- 結論: 答えを文書に戻す部分が成り立たない。「見る・続ける」専用なら使える。

### (c) 非対話のコマンドを、見えるシェルのタブの中で動かし、出力も `tee` などで捕まえて、終わったら挿入

- 打ち込み: `claude -p ... | tee FILE` のような行を端末に書く。
- 本文の渡し方: 本文を端末に打つと、シェルの履歴（`~/.bash_history` など。ADR-0028 の案 5（決めたこと 3） は「履歴はシェルが書く」）に、文書の全文が平文で残る。ヒアドキュメントはシェルごとに書き方が違う（bash、zsh、PowerShell、cmd）。一時ファイルに本文を書いて渡すなら、文書の内容をファイルに書くことになる。
- 答え: `tee` が一時ファイルに書く。端末の出力をファイルに書くことになり、ADR-0028 の案 5（決めたこと 3）（出力をファイルに残さない）の精神に反する。端末が書くのではなくシェルのコマンドが書くが、結果は同じ。
- 「裏でシェルが動き、別タブで確認」: 文字どおりに満たす。
- 終わりの検知: `ChildExit` はシェルが終わるときのもの。コマンドの終わりは、別の仕組み（OSC 133 など）が要る。
- 結論: 本文の履歴、一時ファイル、終わりの検知で、(a) より弱い点が多い。推奨しない。

### (d) 見つけたもう 1 つの案: (a) を土台にして、確認と見える化を厚くする

(a) と同じ実行。ただし、確認と実行中の様子を見せる画面を厚くする。
- 送る前の確認の窓: 実行するコマンド、指示の文、本文の最初と終わり、文字数、送り先（利用者が付けた名前）を出し、「送る」を押すまで何も送らない。
- ログのタブ: 質問ごとに「指示・本文・答え・終了コード・かかった時間」を並べる。メモリだけ。閉じれば消える。
- 「シェルが動いている」ことは、ログのタブに、動いているコマンド行をそのまま出して見せる。同じコマンドを自分で試したいときは、コピーしてシェルのタブに貼る（利用者の操作なのでADR-0028 の案 4（決めたこと 2） の範囲）。

## 6. 安全とプライバシーの事実

- 文書の本文は、選択でも全文でも、提供元のサーバーに送られる。Anthropic と OpenAI の API 系は学習に使わない（上の表）。個人向けのプランと、Google の無料の API は、使う・人が読むことがある。
- 送る文字の確認: ほかのエディタは、差分か、インラインのプロンプトの確認が中心。送る内容の全文を事前に見せるのは gptel の dry-run くらい。MATSU は確認の窓を出すことで、これより強くできる。
- 秘密: 選択に、パスワード、API キー、個人情報が入りうる。Claude Code の `/feedback` のアップロードでは、既知のキーの形が消されるが、通常の送信にその説明はない。MATSU が送る前に文字列の形で見つけて警告することはできるが、確実ではない【推測】。
- 許す提供元とツール: MATSU が一覧を持たず、利用者が設定したコマンドだけを動かす形（ADR-0029 のワンライナーと同じ）が最も軽い。その場合、MATSU はどのサービスに送られるかを知らない（コマンドは何でもよい）ので、確認の窓の「送り先」は利用者が付けた名前になる。
- ローカルだけのモデル: Ollama は 127.0.0.1 だけで待つ。ただしクラウドのモデルがあり、`OLLAMA_NO_CLOUD=1` で切れる。クラウドのモデルを選ぶと外に出る【モデル名での見分け方は未確認】。
- コマンドを実行できるツール: 上の 1.2。文書の本文は信用できない入力。MATSU が、ツールを切るフラグを付けた設定の例を出すことは考えられる。ただし、ツールの版でフラグが変わりうる（Claude Code の `--permission-prompts` は 2.1.259 以降）。
- Claude Code の `-p` は、作業フォルダのプロジェクトの hooks と `.mcp.json` を、信頼の確認なしで動かす（`--bare` なしのとき）。AI の実行は、文書のフォルダでなく、空のフォルダを作業フォルダにするのが安全。
- ログ: ツール自身が履歴を残す（Claude: `~/.claude/projects/` に 30 日、Codex: セッションファイル、`llm`: SQLite）。`--no-session-persistence`（Claude）、`--ephemeral`（Codex）、`-n`（`llm`）で残さない。MATSU のログのタブは、メモリだけ。
- 起動の引数: Copilot のように本文を引数に入れるツールは、本文が他のプロセスから見える。stdin に渡せるツールを優先する。
