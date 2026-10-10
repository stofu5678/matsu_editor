# 開発の計画

最終更新: 2026-10-10

サクラエディタの仕様を一度に取り込むと破綻するので、メモ帳相当の基本機能から作り、あとでサクラの機能をまとめて実装する（[ADR-0001](adr/0001-project-approach.md)）。

## フェーズ

| Phase | 内容 | 完了の目安 |
|---|---|---|
| 0. 土台 | 決定事項の記録、ライセンス、UI・印刷・スクリプトの試作、Cargo ワークスペースと CI | UI フレームワークが決まり、3 OS の CI が通る。印刷とスクリプトの作り方の見通しが立つ |
| 1. メモ帳相当 | 開く・保存（文字コード、BOM、改行コード）、編集、Undo/Redo、IME、検索・置換（文字列のみ）、行番号、タブ・全角空白・改行マークの表示、折り返し、サクラのキー割り当てとメニュー、サクラと互換の設定ファイル（[ADR-0010](adr/0010-sakura-compatible-settings.md)）。扱うファイルは 2GB まで（[ADR-0012](adr/0012-file-size-limit.md)） | 日常のメモ書きに使える。サクラの設定ファイルをそのまま読み書きできる |
| 2. 正規表現 | Onigmo の組み込み、サクラと同じ検索・置換オプション | bregonig との互換テストに合格する |
| 3. マクロ・プラグイン | サクラ互換のマクロ（キーマクロと JScript）、プラグインホスト、Python / Ruby 用ライブラリ（JSON-RPC）、イベント、キーマクロの記録（[ADR-0007](adr/0007-macros-and-plugins.md)） | サクラの JScript マクロが動き、Python と Ruby でマクロが書ける |
| 4. 印刷 | 共通のページ割り付け、印刷プレビュー、OS ごとの印刷（[ADR-0011](adr/0011-printing.md)） | 3 OS で印刷とプレビューができる |
| 5. 並列実装 | サクラの機能台帳から、複数のサブエージェントで機能をまとめて実装する | — |
| 6. 巨大ファイル | 2GB を超えるファイルを範囲ごとに読む方式、改行位置のバックグラウンド走査、ファイル全体を順に読みながらの検索 | 10GB のファイルを開いてすぐスクロールできる |

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

## Phase 0 の作業

- [x] 決定事項の記録（docs/adr/）
- [x] ライセンス、README、CLAUDE.md
- [x] GitHub リポジトリを作る（https://github.com/stofu5678/ume_editor 、公開）
- [ ] Onigmo を fork する（ユーザー）
- [ ] UI を試作し、ADR-0004 を確定する（まず Windows ARM64 で行う。[ADR-0009](adr/0009-initial-development-target.md)、[試作の手順](poc/ui-framework.md)）
  - GPUI の試作（[poc/gpui-textview](../poc/gpui-textview/README.md)、[結果](poc/ui-framework-result.md)）が Windows ARM64・x64、Mac、Linux（WSLg）で動いた。文字を書き込むたびにメモリが 4 MB 増える問題は Qualcomm Adreno のドライバーだけで起き、NVIDIA と Mac では起きなかった。残りは、実機の Linux での確認と、Mac 対応を入れた後のコードの Windows ARM64 でのビルド
- [ ] 印刷を試作する（3 OS でダイアログを出し、日本語の 1 ページを印刷する。共通の割り付けからプレビューも描く。[ADR-0011](adr/0011-printing.md)）
- [ ] スクリプトの処理系を試作する（JScript のマクロを組み込みの処理系で動かし、試作のコマンドを呼ぶ。[ADR-0007](adr/0007-macros-and-plugins.md)）
- [ ] JSON-RPC を試作する（別プロセスの Python から試作のコマンドを呼ぶ。[ADR-0007](adr/0007-macros-and-plugins.md)）
- [x] Cargo ワークスペースと CI（Windows x64・ARM64 / macOS / Linux x64・ARM64）
