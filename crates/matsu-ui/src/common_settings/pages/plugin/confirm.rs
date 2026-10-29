//! The confirmation screen for a plugin (ADR-0026, chapter 4, item 2): what the user is told
//! before a plugin is added or enabled.
//!
//! A plugin runs code by itself, with the rights of the user and no limit. The screen says who
//! wrote it, where it came from, when it runs, in which language and with which interpreter, and
//! that the only protection is the user's decision.

use std::path::Path;

use matsu_config::plugins::{Jack, PluginDef};
use matsu_plugin::session::Language;

/// The line that is always on the screen (ADR-0026, chapter 4).
pub(super) const WARNING: &str = "このプラグインはあなたの権限で、制限なしに動きます。\
信頼できる配布元のものだけを有効にしてください。";

/// When the jack runs the plugin, in plain words.
pub(super) fn jack_text(jack: Jack) -> &'static str {
    match jack {
        Jack::Command => "メニュー・ツールバー・キーで選んだとき（自分で選んだときだけ）",
        Jack::EditorStart => "ウィンドウを開いたときに自動で動く",
        Jack::EditorEnd => "ウィンドウを閉じるときに自動で動く",
        Jack::DocumentOpen => "ファイルを開いた後に自動で動く",
        Jack::DocumentClose => "ファイルを閉じる前に自動で動く",
        Jack::DocumentBeforeSave => "ファイルを保存する前に自動で動く",
        Jack::DocumentAfterSave => "ファイルを保存した後に自動で動く",
        Jack::Outline => "アウトライン解析のときに動く",
        Jack::SmartIndent => "改行などを入力した後に自動で動く",
        Jack::Complement | Jack::ComplementGlobal => "入力補完の候補を探すときに動く",
    }
}

/// The lines of "when it runs": one for each jack the plugin uses once, in the order of the
/// jacks, and the commands.
fn when_lines(def: &PluginDef) -> Vec<String> {
    let mut lines: Vec<String> = Jack::ALL
        .into_iter()
        .filter(|jack| match jack {
            Jack::Command => def.plugs.iter().any(|p| p.jack == *jack) || !def.commands.is_empty(),
            _ => def.plugs.iter().any(|p| p.jack == *jack),
        })
        .map(|jack| jack_text(jack).to_owned())
        .collect();
    if lines.is_empty() {
        lines.push("（動くときが定義にありません）".to_owned());
    }
    lines
}

/// The language and the interpreter of a handler file, as the user would call them.
fn language_of(handler: &str) -> String {
    match Language::of_file(Path::new(handler)) {
        Some(language) => format!(
            "{}（{}）",
            language.display_name(),
            language.default_interpreter().to_string_lossy()
        ),
        None => {
            let ext = handler.rsplit_once('.').map_or("", |(_, e)| e);
            format!("MATSU Editor が知らない種類（.{ext}）")
        }
    }
}

/// The languages of the handlers of the plugin, each once.
fn language_lines(def: &PluginDef) -> Vec<String> {
    let handlers = def
        .plugs
        .iter()
        .map(|p| p.handler.as_str())
        .chain(def.commands.iter().map(|c| c.handler.as_str()));
    let mut lines: Vec<String> = Vec::new();
    for handler in handlers {
        let line = language_of(handler);
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    lines
}

/// The text of the screen. `folder` is the place of the plugin on disk.
pub(super) fn describe(def: &PluginDef, folder: Option<&Path>) -> String {
    let field = |text: &str| {
        if text.is_empty() {
            "（なし）".to_owned()
        } else {
            text.to_owned()
        }
    };
    let mut out = String::new();
    out += &format!("名前: {}\n", field(&def.name));
    out += &format!("作者: {}\n", field(&def.author));
    out += &format!("バージョン: {}\n", field(&def.version));
    out += &format!("配布元: {}\n", field(&def.url));
    out += &format!(
        "フォルダー: {}\n",
        folder.map_or_else(
            || "（見つかりません）".to_owned(),
            |p| p.display().to_string()
        )
    );
    out += "\n動くとき:\n";
    for line in when_lines(def) {
        out += &format!("・{line}\n");
    }
    out += "\n使う言語（処理系）:\n";
    let languages = language_lines(def);
    if languages.is_empty() {
        out += "・（なし）\n";
    }
    for line in languages {
        out += &format!("・{line}\n");
    }
    out += &format!("\n{WARNING}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(text: &str) -> PluginDef {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        PluginDef::from_bytes(&bytes, None)
    }

    #[test]
    fn the_screen_says_who_when_where_and_the_warning() {
        let d = def(
            "[Plugin]\r\nName=名前\r\nAuthor=作者\r\nVersion=1.0\r\nUrl=https://example.com/\r\nType=wsh\r\n\
[Plug]\r\nDocumentBeforeSave=save.js\r\nDocumentOpen=open.js\r\n[Command]\r\nC[1]=cmd.js\r\n",
        );
        let text = describe(&d, Some(Path::new("/p/Sample")));
        for expected in [
            "名前: 名前",
            "作者: 作者",
            "バージョン: 1.0",
            "配布元: https://example.com/",
            "フォルダー: /p/Sample",
            "・ファイルを保存する前に自動で動く",
            "・ファイルを開いた後に自動で動く",
            "・メニュー・ツールバー・キーで選んだとき",
            "QuickJS (matsu-script)",
            WARNING,
        ] {
            assert!(text.contains(expected), "{expected}\n{text}");
        }
        // The jacks come in Sakura's order, and a language is said once.
        assert!(text.find("開いた後").unwrap() < text.find("保存する前").unwrap());
        assert_eq!(text.matches("QuickJS").count(), 1);
    }

    #[test]
    fn what_is_missing_is_said_and_an_unknown_extension_is_named() {
        let d = def("[Plugin]\r\nType=wsh\r\n[Plug]\r\nOutline=a.xyz\r\n");
        let text = describe(&d, None);
        assert!(text.contains("名前: （なし）"));
        assert!(text.contains("フォルダー: （見つかりません）"));
        assert!(text.contains("知らない種類（.xyz）"));
        assert!(text.contains("アウトライン解析のときに動く"));
        let none = describe(&def("[Plugin]\r\nType=wsh\r\n"), None);
        assert!(none.contains("動くときが定義にありません"));
    }

    #[test]
    fn every_jack_has_words() {
        for jack in Jack::ALL {
            assert!(!jack_text(jack).is_empty());
        }
    }
}
