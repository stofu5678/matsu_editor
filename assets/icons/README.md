# MATSU Editor のアイコン

MATSU Editor のアイコンです。ノート（リングノート）と松の枝（松かさ付き）の絵です。

## 由来

2026-10-22 に、ユーザーが claude.ai のチャットで Claude と一緒に作りました（ADR-0021 の改名で、梅の絵から替えました。梅の絵は 2026-10-21 に同じ方法で作ったもので、#198 で取り込みました）。

## ライセンス

コードと同じ MIT OR Apache-2.0 です（梅の絵のときに、ユーザーが 2026-10-21 に決めました。松の絵も同じです。LICENSE-MIT と LICENSE-APACHE を見てください）。

## ファイル

| ファイル | 大きさ | 使う場所 |
|----------|--------|----------|
| `matsu_editor.svg` | ベクター | 元の絵。文書や、あとで `.icns` などを作るときの元 |
| `matsu_editor_32.svg` | ベクター（32 px 向け） | 小さい大きさ用に簡単にした絵 |
| `matsu_editor_16.svg` | ベクター（16 px 向け） | 同上 |
| `matsu_editor_16.png` | 16 x 16 | 16 px 用に簡単にした絵 |
| `matsu_editor_32.png` | 32 x 32 | 32 px 用に簡単にした絵。バージョン情報ダイアログに出す |
| `matsu_editor_48.png` | 48 x 48 | 今は使っていない |
| `matsu_editor_256.png` | 256 x 256 | Linux（X11）のウィンドウのアイコン |
| `matsu_editor_512.png` | 512 x 512 | 大きい表示用（今は使っていない） |
| `matsu_editor_mac.svg` | ベクター（1024 x 1024） | macOS 用。四角そのものがノートの紙。元の絵の松の枝を 4 倍にして組み直した |
| `matsu_editor_mac_32.svg` | ベクター（32 px 向け） | 16・32・64 px 用に簡単にした絵 |
| `matsu_editor_mac_{16,32,64,128,256,512,1024}.png` | 左の大きさ | macOS 用の PNG。`scripts/package-mac-app.sh` が `.icns` にする（16・32・64 px は `_32.svg` から、128 px 以上は `matsu_editor_mac.svg` から作った） |
| `matsu_editor.ico` | 16, 32, 48, 64, 128, 256（PNG 形式） | Windows の `matsu.exe` にアイコンのリソース ID 1 として埋め込む |

16 px と 32 px は、小さくても見分けがつくように、細部を省いた別の絵にしてあります。

ファイルは、`matsu_editor_mac*` 以外はもらったものをそのまま置いています（描き直していません）。`matsu_editor_mac*` は、ユーザーの絵（元の SVG）から Claude が組み直したもので、ライセンスは同じ MIT OR Apache-2.0 です。

## 使っている場所

- Windows: `crates/matsu/build.rs` と `crates/matsu/matsu.rc` が `matsu_editor.ico` を `matsu.exe` にリソース ID 1 として埋め込みます。GPUI の Windows 版は ID 1 のアイコンをウィンドウのクラスに使うので、エクスプローラー、タイトルバー、タスクバー、Alt+Tab に出ます
- Linux（X11）: `crates/matsu-ui/src/app_icon.rs` が `matsu_editor_256.png` を読み、すべてのウィンドウのアイコンにします。Wayland は、アプリからアイコンを渡せません（`.desktop` ファイルで決まります）
- バージョン情報ダイアログ: `matsu_editor_32.png` を左上に出します（サクラの `IDC_STATIC_MYICON`）
- リリースのアーカイブ: `assets/icons` の SVG と PNG を同梱します（`.github/workflows/package.yml`）

- macOS: `scripts/package-mac-app.sh` が `matsu_editor_mac_*.png` から `matsu_editor.icns` を作り、`.app` バンドルに入れます（1024 px は `icon_512x512@2x.png`）。元の絵（`matsu_editor_16.png` など、`mac` の付かないもの）は Windows と Linux 用で、Mac には使いません

Linux の `.desktop` ファイルは、まだありません。

## macOS 用のアイコンの形

macOS 26 以降は、アイコンが決まった角の丸い四角の形でないと、灰色の四角の上に縮めて載せます。元の絵（透明な背景にノートだけ）は、そのため Dock で灰色の枠の中に小さく出ていました（2026-11-05 にユーザーが Mac で見つけました）。

Apple の型（macOS 11 以降）に従います。出典は Apple Developer Forums の Specs for app icon for macOS app（https://developer.apple.com/forums/thread/670578）と、同じ数字を引く https://developer.apple.com/forums/thread/761179 です。Apple の Production Templates（https://developer.apple.com/design/resources/）の原本は確かめていません。

- キャンバスは 1024 x 1024 px、本体は 824 x 824 px を中央に置く（まわりに 100 px の余白）
- 角の半径は 185.4 px（円弧。Apple の本体は連続曲線なので、わずかに違う）
- 影: Apple は数字を公開していない。見た目で、下に 12 px ずらし、ぼかし 12（標準偏差）、黒 30% とした（推測）

本体の四角いっぱいに紙（白）を敷き、上 190 px を緑の帯にして、リングと罫線を置きます。松の枝は元の SVG の部品をそのまま使っています。2026-11-05 に、ユーザーが 2 つの案（もう 1 つは、緑の四角の上にノートと松を置く案）から、この紙の案を選びました。

### 作り方

道具は新しく入れていません。Windows の Edge で SVG を PNG にし、WSL の Pillow で縮めました。

1. `matsu_editor_mac.svg`を、元の `matsu_editor.svg` の松の枝・松かさ・リングの部品を取り出して組み直して書く（Python で文字列を組み立てた）
2. Edge で 1024 px に描く:
   `msedge.exe --headless --disable-gpu --hide-scrollbars --screenshot=出力.png --window-size=1024,1024 --default-background-color=00000000 file:///SVG`
   （透明な背景のまま出る。SVG の `width` と `height` を 1024 にしてあるので、画面の大きさと合わせる）
3. 512・256・128 px は、1024 px の PNG を Pillow の `Image.resize(..., Image.LANCZOS)` で縮める
4. 16・32・64 px は、`*_32.svg` の `width` と `height` を 256 にして同じ方法で描き、LANCZOS で縮める。`_32.svg` は小さくても見分けがつくように、細部を省いた別の絵（四角、帯、罫線 2 本、松葉 3 つ）にしてある

### 残っていること

Icon Composer の `.icon`（macOS 26 の Liquid Glass。Xcode 26 の actool が要る）は、この変更では作っていません。
