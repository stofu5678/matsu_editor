# ADR-0004: UI フレームワーク

- 状態: 決定（Linux の GPU での確認は残っている）
- 日付: 2026-10-08（2026-10-11 に、[試作](../poc/ui-framework.md)の結果で決定にした）

## 決定

- GPUI（Zed の UI フレームワーク。crates.io の `gpui-pre` `=0.3.8`）と、部品集の gpui-component（`=0.7.1`）を使う
- 予備は Iced

## 理由

- macOS への統合が深い。ネイティブのメニューバー、CoreText による文字描画、IME、Finder からファイルを開く処理を Zed で実用している
- Windows と Linux にも対応している
- gpui-component には 60 以上の部品があり、商用アプリでも使われている
- ライセンスが Apache-2.0

## 懸念と対策

- 1.0 前で、版ごとに破壊的な変更が多い → 版を固定し、上げるときはまとめて対応する。UI 層を薄くして、変更の影響をコアに及ぼさない
- 資料が少なく、AI の知識も古い → 依存先のソースと examples を読ませて実装する
- Zed のエディタ部分は GPL → コードを写さない（[ADR-0003](0003-language-and-license.md)）

## Iced に切り替える条件

試作で、IME・文字の見え方・起動時間のいずれかが基準を満たさない場合。または、API の変化による手戻りが大きすぎる場合。

## 試作の結果

詳しくは [UI フレームワークの試作の結果](../poc/ui-framework-result.md) と [印刷の試作の結果](../poc/printing.md)。

- 試作（`poc/gpui-textview`）は、Windows ARM64、Windows x64、Mac、Linux（WSLg）でビルドして動いた。WSL で作ったものは、Hyper-V の Ubuntu 26.04.1（GNOME の Wayland）でも動いた
- 日本語入力と文字の見え方は、Windows と Mac と Linux（WSLg の X11 で fcitx5 と Mozc）で、ユーザーが確かめて問題なかった
- Wayland での日本語入力（IBus と Mozc。GPUI は text-input-v3 でつなぐ）と、デスクトップとの統合（タイトルバー、ウィンドウの端での大きさの変更、拡大率、ファイルと印刷のダイアログ）は、Hyper-V の Ubuntu 26.04.1 で、ユーザーが確かめて問題なかった（2026-10-14）
- 起動時間は、最初の描画まで Windows ARM64 で 181〜206 ms、Mac で 203〜233 ms、Linux x64（WSLg）で 158〜189 ms だった（基準は 0.3 秒以内）。Windows x64 は 440〜542 ms で基準を超えた。同じ PC で消す前の `poc/gpui` も 439〜477 ms だったので、この PC の環境によるとみられるが、中身は調べていない
- 100 万行のテキストを作って、末尾への移動やページ送りを正しく描けた
- 文字を書き込むたびにメモリが約 4 MB 増える問題は、Qualcomm Adreno のドライバーだけで起きた（NVIDIA GeForce GTX 1650 と Mac では起きなかった）。GPUI の Windows 部分に当てる回避策を `poc/gpui-textview/patches/` に残した。当てると、起動直後で約 54 MB になる。本番では回避策を当てた GPUI を使い、GPUI の版を上げるたびに当て直す
- Windows と Linux では `cx.set_menus` が画面に何も出さない。メニューバーは gpui-component の `AppMenuBar` で自分で描く。`AppMenuBar` にはキーボードで開く手段がないので、写して手を加えた。`PopupMenu` は、キーボードで開いたときの先頭項目の選択と、項目の文字での選択がなく、本番では自前で用意する
- 印刷は GPUI にない機能だが、3 OS とも OS の仕組みで作れた（[ADR-0011](0011-printing.md)）
- 資料が少なくても、依存先のソースと examples を読めば API を調べられ、手戻りは少なかった

## 残っている確認

- 実機の Linux での確認は、2026-10-11 にユーザーが Phase 0 の条件から外し、次の 2 つに分けた。結果が悪ければ、「Iced に切り替える条件」で見直す
  - Wayland での日本語入力と、デスクトップとの統合（タイトルバー、ウィンドウの端での大きさの変更、拡大率）: 済み。2026-10-14 に、開発用の PC（Windows 11 ARM64）の Hyper-V で Ubuntu 26.04.1 のデスクトップ（GNOME 50 の Wayland、基本のセッション）を動かして確かめ、問題なかった（[試作の結果](../poc/ui-framework-result.md)の「Hyper-V の Ubuntu（Wayland）での確認」）
  - GPU での描画の速さとメモリ。WSLg では GPU を使えず、CPU で描いていた（Vulkan の llvmpipe）。Hyper-V の仮想マシンでも GPU は使えない見込み。Linux 版を利用者に出す前に、実機（USB から起動する Ubuntu など）か、利用者からの報告で確かめる
- Windows x64 の起動時間が基準を超えた件の原因（上の「試作の結果」）。測った PC は以前使っていた環境で、アプリがたくさん入ったままになっている。そのため環境によるものとみて、この決定を妨げないことにした（2026-10-14、ユーザー）。原因は別のセッションで調べる（[#23](https://github.com/stofu5678/ume_editor/issues/23)）
- Intel・AMD の GPU で、メモリが増える問題が起きるか
- Linux の X11（XIM）の日本語入力で、起動して最初の変換だけ候補の窓がウィンドウの左下に出る件の原因。Wayland（text-input-v3）では起きなかった（GPUI はフォーカスを得たときにカーソルの位置を IME に送る）
- MPL-2.0 の依存（`cbindgen`、`option-ext`、`dwrote`）の配布前の扱い。MPL-2.0 は使ってよい（2026-10-10 のユーザーの判断）が、改変したファイルは公開する。Windows 版の試作の依存（gpui-component を含む）のライセンスは調べていない

## 見送った候補

- Unity / Godot / Bevy: ランタイムが重く、起動が遅い。テキスト入力と IME が弱い。状態がシーンやエディタに散り、AI がコードだけで全体を追えない
- Tauri: WebView で描くため、軽さが落ちる
- egui: 日本語フォントを自前で登録する必要があり、IME の品質も要検証
- Slint: ライセンスが GPL / 表示義務付きの無償 / 商用の 3 択で、MIT OR Apache-2.0 の方針と合わせにくい
