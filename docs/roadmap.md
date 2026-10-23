# 開発の計画

最終更新: 2026-10-14

サクラエディタの仕様を一度に取り込むと破綻するので、メモ帳相当の基本機能から作り、あとでサクラの機能をまとめて実装する（[ADR-0001](adr/0001-project-approach.md)）。

## フェーズ

| Phase | 内容 | 完了の目安 |
|---|---|---|
| 0. 土台 | 決定事項の記録、ライセンス、UI・印刷・JSON-RPC の試作、Cargo ワークスペースと CI | UI フレームワークが決まり、3 OS の CI が通る。印刷とスクリプトの作り方の見通しが立つ |
| 1. メモ帳相当 | 開く・保存（文字コード、BOM、改行コード）、編集、Undo/Redo、IME、検索・置換（文字列のみ）、行番号、タブ・全角空白・改行マークの表示、折り返し、サクラのキー割り当てとメニュー、サクラと互換の設定ファイル（[ADR-0010](adr/0010-sakura-compatible-settings.md)）。扱うファイルの大きさに上限はない（64 ビット版のサクラと同じ。[ADR-0012](adr/0012-file-size-limit.md)） | 日常のメモ書きに使える。サクラの設定ファイルと同じ形式（`matsu.ini`）で読み書きできる |
| 2. 正規表現 | Onigmo の組み込み、サクラと同じ検索・置換オプション | bregonig との互換テストに合格する |
| 3. マクロ・プラグイン | キーマクロの記録と実行、プラグインホスト（JSON-RPC）、外部の言語（Node.js・Python・Ruby・PHP）用ライブラリ、同梱の処理系（Lua・QuickJS・MicroPython・mruby を `matsu-script` にまとめる）、イベント（[ADR-0007](adr/0007-macros-and-plugins.md)）。ほかの言語は第 2 弾以降 | サクラのマクロでできることを、JavaScript と Python で書ける |
| 4. 印刷 | 共通のページ割り付け、印刷プレビュー、OS ごとの印刷（[ADR-0011](adr/0011-printing.md)） | 3 OS で印刷とプレビューができる |
| 5. 並列実装 | サクラの機能台帳から、複数のサブエージェントで機能をまとめて実装する | — |
| 6. 巨大ファイル | 巨大なファイルをメモリに全部読まずに、範囲ごとに読んで速く開く方式、改行位置のバックグラウンド走査、ファイル全体を順に読みながらの検索 | 10GB のファイルを開いてすぐスクロールできる |

## Phase 1 から用意しておく差し込み口

実装は後のフェーズでも、入口は Phase 1 で作る。

- 元データの持ち方: 「全部読む」と「範囲ごとに読む」を差し替えられる形にする（[ADR-0005](adr/0005-text-buffer-and-large-files.md)）
- 検索エンジン: Phase 1 は文字列検索だけ。Phase 2 で Onigmo を差し込む（[ADR-0006](adr/0006-regex-engine.md)）
- コマンドとイベント: すべての操作をコマンドにする（[ADR-0008](adr/0008-commands-and-keybindings.md)）
- 選択範囲: 矩形選択を扱える形にする
- 印刷: 表示とページの割り付けを分けておき、Phase 4 で割り付けを足せるようにする

## Phase 5（並列実装）に向けた準備

- 1 コマンド 1 ファイルにし、登録は自動で集める
- 編集コマンドのテストは、カーソル位置付きのテキストで期待値を書く形式にする（形式は Phase 1 で決める）
- サクラの機能台帳（コマンド・設定・ヘルプを、依存関係と優先度付きで一覧にしたもの）を作る
- バッファ、カーソル・選択、検索の API を Phase 5 の前に固める
- エージェントごとに別の git worktree で作業し、テストとレビューを通ったものだけを取り込む
- 人のレビューを挟まずに実装・テスト・レビュー・マージを繰り返すループのルールを、`CLAUDE.md` に書く（[ADR-0014](adr/0014-development-process.md)）
- 毎日版の公開と、版の自動付与を動かしておく（Phase 1 で実行ファイルができたら作る）

## Phase 0 の作業

- [x] 決定事項の記録（docs/adr/）
- [x] ライセンス、README、CLAUDE.md
- [x] GitHub リポジトリを作る（https://github.com/stofu5678/matsu_editor 、公開）
- [x] Onigmo の取り込み元を決める（当面は上流をそのまま使い、改修が要るときに fork する。2026-10-11、ユーザー。[ADR-0006](adr/0006-regex-engine.md)）
- [x] UI を試作し、ADR-0004 を確定する（2026-10-11 に GPUI で決定。まず Windows ARM64 で行う。[ADR-0009](adr/0009-initial-development-target.md)、[試作の手順](poc/ui-framework.md)）
  - GPUI の試作（[poc/gpui-textview](../poc/gpui-textview/README.md)、[結果](poc/ui-framework-result.md)）が Windows ARM64・x64、Mac、Linux（WSLg）で動いた。文字を書き込むたびにメモリが 4 MB 増える問題は Qualcomm Adreno のドライバーだけで起き、NVIDIA と Mac では起きなかった。Mac 対応を入れた後のコードも、2026-10-11 に印刷の試作で Windows ARM64 でビルドして動かした。実機の Linux での確認は、ユーザーが Phase 0 の条件から外した（2026-10-11）。Wayland での日本語入力とデスクトップとの統合は、2026-10-14 に Hyper-V の仮想マシン（Ubuntu 26.04.1、GNOME の Wayland）で確かめて問題なかった。GPU での描画は Linux 版を出す前に実機で確かめる
- [x] 印刷を試作する（3 OS でダイアログを出し、日本語の 1 ページを印刷する。共通の割り付けからプレビューも描く。[ADR-0011](adr/0011-printing.md)）
  - 3 OS の分を作った（[poc/gpui-textview](../poc/gpui-textview/README.md)、[結果](poc/printing.md)）。共通の割り付けから、GPUI のプレビューと、Mac（Core Graphics）・Windows（GDI）・Linux（PDF）の印刷を描けて、PDF に出力すると同じ位置に文字が出た。3 OS とも、印刷ダイアログから紙に印刷できた（2026-10-11）。この結果で ADR-0011 を決定にした
- [x] JSON-RPC を試作する（別プロセスの JavaScript（Node.js）と Python から、試作のコマンドを呼ぶ。[ADR-0007](adr/0007-macros-and-plugins.md)、[通信の仕様の案](poc/json-rpc.md)）
  - 試作のエディタから、別プロセスの Node.js と Python のマクロが、サクラの関数 14 個を JSON-RPC で呼べた（[poc/gpui-textview](../poc/gpui-textview/README.md)、[結果](poc/json-rpc.md)）。Linux と Windows ARM64 で、日本語と絵文字、遅れた応答、1 MB を超える行、エラーの行番号、タイムアウト、中断を確かめた。Windows では、ユーザーもウィンドウで確かめた（2026-10-14）。macOS では動かしていない
- [ ] モジュールの構造を決める（[ADR-0013](adr/0013-module-structure.md)。上の試作で確かめ、Phase 1 の始めに確定する）
  - 構造は ADR-0013 のままでよいと、ユーザーが判断した（2026-10-11）。コマンドの集め方（`inventory`）など 3 つの確認の提案を、2026-10-14 にユーザーが採用した。`CommandContext` は、JSON-RPC の試作の結果で詰めた。Phase 1 の始めに確定する
- [x] Cargo ワークスペースと CI（Windows x64・ARM64 / macOS / Linux x64・ARM64）
