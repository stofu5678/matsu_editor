//! 「シェル」: the settings of the shell tab (ADR-0028, decision 7). Sakura has no such page, so it is
//! MATSU Editor's own, after the pages of Sakura, and its keys are in the section `[Terminal]` of the settings
//! file (`matsu_config::terminal`), not in `[Common]`.
//!
//! * the shell: one command line, the program and its arguments as they would be typed (empty: the default
//!   shell). A line that cannot be read (a quote that is not closed) is said so under the box; the tab starts
//!   with the default shell then.
//! * the length of the scrollback, in lines (`0` to `MAX_SCROLLBACK_LINES`; OK takes a number outside of it
//!   into the range, as the other pages do).
//! * on macOS only: 「Option を Meta として使う」.
//!
//! OK writes the section and tells the shell tabs: a tab that starts afterwards uses the shell and the
//! scrollback; the tabs that are open keep theirs, and "Option as Meta" reaches them at once
//! (`terminal::settings`).

use gpui::{AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, div, rgb};
use matsu_config::terminal::{
    DEFAULT_SCROLLBACK_LINES, KEY_OPTION_AS_META, KEY_SCROLLBACK_LINES, KEY_SHELL,
    MAX_SCROLLBACK_LINES, SECTION, TerminalSettings,
};
use matsu_terminal::{CommandLineError, ShellCommand};

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item, Values};

pub(in crate::common_settings) const SHELL: &str = "terminal_shell";
pub(in crate::common_settings) const SCROLLBACK: &str = "terminal_scrollback";
pub(in crate::common_settings) const OPTION_AS_META: &str = "terminal_option_as_meta";

/// The color of a warning.
const WARNING: u32 = 0xc00000;

/// An example of a command line, in the form of this OS.
const EXAMPLE: &str = if cfg!(windows) {
    "例: cmd.exe /K title \"マツエディタ\"    pwsh.exe -NoLogo"
} else {
    "例: /bin/zsh -l    /usr/bin/fish --login"
};

pub(in crate::common_settings) fn items() -> Vec<Item> {
    items_for(cfg!(target_os = "macos"))
}

/// The controls of the page; `mac` adds the one that only macOS has.
pub(in crate::common_settings) fn items_for(mac: bool) -> Vec<Item> {
    let mut items = vec![
        Item::Group(
            "シェル",
            vec![
                Control::text(SHELL, "コマンドライン(&C):", KEY_SHELL, "")
                    .in_section(SECTION)
                    .into(),
                Item::Custom(draw_shell_note),
                Item::Label(EXAMPLE),
                Item::Label(
                    "プログラムと引数を、コマンドラインのとおりに書きます。\n空のときは、この環境の既定のシェルです\n(Windows は PowerShell、macOS と Linux はログインシェル)。\nmacOS と Linux では、~ や $HOME は展開されません。",
                ),
            ],
        ),
        Item::Group(
            "スクロールバック",
            vec![
                Control::number(
                    SCROLLBACK,
                    "保持する行数(&L):",
                    KEY_SCROLLBACK_LINES,
                    DEFAULT_SCROLLBACK_LINES as i32,
                    0,
                    MAX_SCROLLBACK_LINES as i32,
                )
                .in_section(SECTION)
                .into(),
                Item::Label(
                    "0 から 50000 行まで。タブ 1 枚あたり、1 万行で約 29 MB のメモリを使います。",
                ),
            ],
        ),
        Item::Label(
            "シェルと行数は、新しく開くシェルのタブから使われます。\n開いているタブは、そのままです。",
        ),
    ];
    if mac {
        items.push(Item::Group(
            "キーボード",
            vec![
                Control::check(
                    OPTION_AS_META,
                    "Option を Meta として使う(&M)",
                    KEY_OPTION_AS_META,
                    false,
                )
                .in_section(SECTION)
                .into(),
                Item::Label(
                    "オンのとき、Option+キーで ESC とそのキーを送ります。\nオフのときは、Option で入る文字を入力します。",
                ),
            ],
        ));
    }
    items
}

/// Why the command line in the box cannot be the shell; `None` when it can, or when it is empty (the default).
pub(in crate::common_settings) fn problem(line: &str) -> Option<CommandLineError> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    ShellCommand::from_command_line(line).err()
}

/// The words under the box when the command line cannot be used.
pub(in crate::common_settings) fn problem_text(error: CommandLineError) -> &'static str {
    match error {
        CommandLineError::UnclosedQuote => {
            "引用符が閉じていません。このままでは、既定のシェルを使います。"
        }
        CommandLineError::Empty => "プログラムがありません。このままでは、既定のシェルを使います。",
    }
}

fn draw_shell_note(
    dialog: &CommonSettingsDialog,
    _cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    match problem(dialog.values().text(SHELL)) {
        Some(error) => div()
            .text_color(rgb(WARNING))
            .child(problem_text(error))
            .into_any_element(),
        None => div().into_any_element(),
    }
}

/// The settings that the values of the page say (the numbers are taken into their range).
pub(in crate::common_settings) fn settings_of(values: &Values) -> TerminalSettings {
    TerminalSettings::from_values(
        values.text(SHELL),
        i64::from(values.int(SCROLLBACK)),
        values.bool(OPTION_AS_META),
    )
}

/// What OK does besides writing `matsu.ini` (and what the start does with the file): the shell tabs that start
/// afterwards take the settings. The check box is only on the page of macOS; elsewhere the setting is off,
/// which is what it means there, and the file keeps what it says.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    crate::terminal::settings::set(env.cx, settings_of(after));
}

#[cfg(test)]
mod tests;
