# UME Editor のアイコン

UME Editor のアイコンです。ノート（リングノート）と梅の枝の絵です。

## 由来

2026-10-21 に、ユーザーが claude.ai のチャットで Claude と一緒に作りました。

## ライセンス

コードと同じ MIT OR Apache-2.0 です（ユーザーが 2026-10-21 に決めました。LICENSE-MIT と LICENSE-APACHE を見てください）。

## ファイル

| ファイル | 大きさ | 使う場所 |
|----------|--------|----------|
| `ume_editor.svg` | ベクター | 元の絵。文書や、あとで `.icns` などを作るときの元 |
| `ume_editor_32.svg` | ベクター（32 px 向け） | 小さい大きさ用に簡単にした絵 |
| `ume_editor_16.svg` | ベクター（16 px 向け） | 同上 |
| `ume_editor_16.png` | 16 x 16 | 16 px 用に簡単にした絵 |
| `ume_editor_32.png` | 32 x 32 | 32 px 用に簡単にした絵。バージョン情報ダイアログに出す |
| `ume_editor_48.png` | 48 x 48 | 今は使っていない |
| `ume_editor_256.png` | 256 x 256 | Linux（X11）のウィンドウのアイコン |
| `ume_editor_512.png` | 512 x 512 | 大きい表示用（今は使っていない） |
| `ume_editor.ico` | 16, 32, 48, 64, 128, 256（PNG 形式） | Windows の `ume.exe` にアイコンのリソース ID 1 として埋め込む |

16 px と 32 px は、小さくても見分けがつくように、細部を省いた別の絵にしてあります。

ファイルは、もらったものをそのまま置いています（描き直していません）。

## 使っている場所

- Windows: `crates/ume/build.rs` と `crates/ume/ume.rc` が `ume_editor.ico` を `ume.exe` にリソース ID 1 として埋め込みます。GPUI の Windows 版は ID 1 のアイコンをウィンドウのクラスに使うので、エクスプローラー、タイトルバー、タスクバー、Alt+Tab に出ます
- Linux（X11）: `crates/ume-ui/src/app_icon.rs` が `ume_editor_256.png` を読み、すべてのウィンドウのアイコンにします。Wayland は、アプリからアイコンを渡せません（`.desktop` ファイルで決まります）
- バージョン情報ダイアログ: `ume_editor_32.png` を左上に出します（サクラの `IDC_STATIC_MYICON`）
- リリースのアーカイブ: `assets/icons` の SVG と PNG を同梱します（`.github/workflows/package.yml`）

macOS の `.icns`（`.app` バンドル）と、Linux の `.desktop` ファイルは、まだありません。
