//! The submenu 「生成 AI」 and the functions after the fifty `F_AI_PROMPT_n` (ADR-0035, 1): the lists the menus expand,
//! `F_AI_ASK` before its modal exists (`F_AI_SETTINGS` opens the sheet: `common_settings::tests_polish`). The move at start has its tests in `crate::ai_send::startup`
//! (the pure change is `matsu_config::settings::ai::migrate`), the saved menus of versions 4 and 6 in `matsu-config`.

use gpui::TestAppContext;
use matsu_config::Profile;
use matsu_config::settings::ai::{
    AiDestinations, F_AI_ASK, F_AI_PROMPT_0, F_AI_PROMPT_LIST, F_AI_SETTINGS,
};

use super::Win;
use crate::ai_send::AiPromptList;
use crate::menu::{Entry, Item, Lists as _, MenuAction};

/// The bytes of `text` as a UTF-8 file with a byte order mark, which is how the profile knows the encoding.
fn bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// A table of `[AI]` with the destinations `(name, command, output)` in a row.
fn table(slots: &[(&str, &str, i32)]) -> AiDestinations {
    let mut text = String::from("[AI]\r\n");
    for (i, (name, command, output)) in slots.iter().enumerate() {
        text += &format!(
            "Name[{i:03}]={name}\r\nCommand[{i:03}]={command}\r\nOutput[{i:03}]={output}\r\n"
        );
    }
    AiDestinations::from_profile(&Profile::from_bytes(&bom(&text)))
}

fn open(cx: &mut TestAppContext, settings: &str) -> (tempfile::TempDir, Win) {
    cx.update(gpui_component::init);
    super::install_settings(cx, settings, None);
    super::open_in(cx, "abc\n")
}

fn labels(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .map(|e| match e {
            Entry::Item(i) => i.label.clone(),
            Entry::Separator => "-".to_owned(),
            Entry::Submenu { label, .. } => format!("> {label}"),
        })
        .collect()
}

#[test]
fn the_list_expands_to_the_registered_destinations_by_name() {
    let list = AiPromptList(std::sync::Arc::new(table(&[
        ("英訳", "claude -p", 1),
        ("", "codex exec -", 0),
    ])));
    let items = list.items(F_AI_PROMPT_LIST);
    let shape: Vec<_> = items
        .iter()
        .map(|i| (i.label.as_str(), i.action.clone(), i.enabled))
        .collect();
    // a destination without a name shows its command
    assert_eq!(
        shape,
        [
            ("英訳", MenuAction::Function(F_AI_PROMPT_0), true),
            (
                "codex exec -",
                MenuAction::Function(F_AI_PROMPT_0 + 1),
                true
            ),
        ]
    );
    assert!(list.items(0).is_empty());
    assert!(
        AiPromptList(Default::default())
            .items(F_AI_PROMPT_LIST)
            .is_empty()
    );
}

/// The entries of the right-click menu (the default one, which has the submenu) with `table` as the destinations.
fn right_click(table: AiDestinations) -> Vec<Entry> {
    use crate::custom_menu::model::{Context, entries};
    let menus = matsu_config::settings::CustomMenus::default();
    let keymap = matsu_commands::keymap::KeyMap::default();
    let list = AiPromptList(std::sync::Arc::new(table));
    let available = |_: u16| true;
    entries(
        0,
        &Context {
            menus: &menus,
            macros: &Default::default(),
            oneliners: &Default::default(),
            destinations: &Default::default(),
            keymap: &keymap,
            available: &available,
            lists: &list,
            parentheses: true,
        },
    )
}

fn ai_submenu(rows: &[Entry]) -> &[Entry] {
    rows.iter()
        .find_map(|e| match e {
            Entry::Submenu { label, entries } if label.starts_with("生成 AI") => Some(entries),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no submenu 「生成 AI」 in {:?}", labels(rows)))
}

#[test]
fn the_right_click_menu_has_the_ai_submenu_with_the_destinations_the_modal_the_log_and_the_settings()
 {
    let rows = right_click(table(&[
        ("英訳", "claude -p", 1),
        ("要約", "codex exec -", 2),
    ]));
    // after the submenu of the one-liners, with the access key 8
    let names = labels(&rows);
    let at = names
        .iter()
        .position(|n| n.starts_with("> 生成 AI"))
        .unwrap();
    assert_eq!(names[at], "> 生成 AI(8)");
    assert!(names[at - 1].starts_with("> ワンライナー"), "{names:?}");
    let inner = ai_submenu(&rows);
    let shape = labels(inner);
    assert_eq!(shape.len(), 5, "{shape:?}");
    assert_eq!(shape[0], "> そのまま送る(P)");
    assert_eq!(shape[1], "指示をつけて送る(I)...");
    assert_eq!(shape[2], "-");
    assert_eq!(shape[3], "AI ログを表示(L)");
    assert_eq!(shape[4], "生成 AI の設定(S)...");
    let Entry::Submenu { entries, .. } = &inner[0] else {
        panic!("no submenu of the destinations");
    };
    assert_eq!(labels(entries), ["英訳", "要約"]);
    let action = |entry: &Entry| match entry {
        Entry::Item(Item { action, .. }) => action.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(action(&inner[1]), MenuAction::Function(F_AI_ASK));
    assert_eq!(
        action(&inner[3]),
        MenuAction::Function(matsu_config::settings::oneliner::F_AI_LOG_SHOW)
    );
    assert_eq!(action(&inner[4]), MenuAction::Function(F_AI_SETTINGS));
}

#[test]
fn with_no_destination_the_submenu_of_the_destinations_says_none_and_is_gray() {
    let rows = right_click(AiDestinations::default());
    let inner = ai_submenu(&rows);
    let Entry::Submenu { entries, .. } = &inner[0] else {
        panic!("no submenu of the destinations");
    };
    assert_eq!(entries.len(), 1);
    let Entry::Item(none) = &entries[0] else {
        panic!("{entries:?}");
    };
    assert_eq!(none.label, "(なし)");
    assert!(!none.enabled);
    assert_eq!(none.action, MenuAction::Placeholder);
    // the rest of the submenu is there all the same
    assert_eq!(inner.len(), 5);
}

#[test]
fn the_main_menu_has_the_ai_submenu_in_the_tool_menu_after_the_oneliners() {
    use crate::menu::main_menu_with;
    let keymap = matsu_commands::keymap::KeyMap::default();
    let list = AiPromptList(std::sync::Arc::new(table(&[("英訳", "claude -p", 1)])));
    let menus = main_menu_with(&keymap, &|_| true, &list);
    let tool = menus
        .iter()
        .find(|m| m.label.starts_with("ツール"))
        .expect("the tool menu");
    let names = labels(&tool.entries);
    let at = names
        .iter()
        .position(|n| n.starts_with("> 生成 AI"))
        .unwrap_or_else(|| panic!("{names:?}"));
    assert_eq!(names[at], "> 生成 AI(G)");
    assert!(names[at - 1].starts_with("> ワンライナー"), "{names:?}");
    let inner = ai_submenu(&tool.entries);
    let shape = labels(inner);
    assert_eq!(shape[0], "> そのまま送る(P)");
    assert_eq!(shape[1], "指示をつけて送る(I)...");
    assert_eq!(shape[3], "AI ログを表示(L)");
    assert_eq!(shape[4], "生成 AI の設定(S)...");
    let Entry::Submenu { entries, .. } = &inner[0] else {
        panic!("no submenu of the destinations");
    };
    assert_eq!(labels(entries), ["英訳"]);
    // and the submenu of the one-liners no longer ends with the log
    let oneliner = tool
        .entries
        .iter()
        .find_map(|e| match e {
            Entry::Submenu { label, entries } if label.starts_with("ワンライナー") => {
                Some(entries)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        labels(oneliner).iter().all(|n| !n.starts_with("AI ログ")),
        "{:?}",
        labels(oneliner)
    );
}

#[gpui::test]
fn the_function_ask_says_on_the_status_bar_that_mode_b_is_not_ready(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    w.call_code(cx, F_AI_ASK);
    assert_eq!(
        w.notice(cx).as_deref(),
        Some(crate::ui_commands::ai::ai_ask::NOT_READY)
    );
    // nothing was started, and no window of the flow opened
    assert!(!w.running(cx));
    assert!(w.view.read_with(cx, |view, _| view.ai_confirm.is_none()));
}

#[test]
fn the_macro_names_reach_the_functions() {
    let row = matsu_commands::macro_names::by_name("AiAsk").unwrap();
    assert_eq!(row.code, F_AI_ASK);
    // the macro runs the function through the registry like a key does
    assert!(
        crate::ui_command::UiRegistry::global()
            .get_by_code(row.code)
            .is_some()
    );
    assert!(
        crate::ui_command::UiRegistry::global()
            .get_by_code(F_AI_SETTINGS)
            .is_some()
    );
}

#[test]
fn the_codes_of_the_config_are_the_ones_of_the_function_table() {
    use matsu_config::settings::ai::{MAX_AI, index_of_prompt_function};
    let code = |name: &str| {
        matsu_commands::sakura::FUNCTIONS
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} is not a function"))
            .code
    };
    for n in 0..MAX_AI {
        let c = code(&format!("F_AI_PROMPT_{n}"));
        assert_eq!(c, F_AI_PROMPT_0 + n as u16);
        assert_eq!(index_of_prompt_function(c), Some(n));
    }
    assert_eq!(code("F_AI_PROMPT_LIST"), F_AI_PROMPT_LIST);
    assert_eq!(code("F_AI_ASK"), F_AI_ASK);
    assert_eq!(code("F_AI_SETTINGS"), F_AI_SETTINGS);
    // the list is a special item of the menus, and it is not one of the destinations
    assert_eq!(index_of_prompt_function(F_AI_PROMPT_LIST), None);
    assert_eq!(index_of_prompt_function(F_AI_PROMPT_0 - 1), None);
}

#[gpui::test]
fn recording_the_ask_function_writes_ai_ask_and_no_instruction(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "");
    let code = |name: &str| {
        matsu_commands::sakura::FUNCTIONS
            .iter()
            .find(|f| f.name == name)
            .unwrap()
            .code
    };
    w.call_code(cx, code("F_RECKEYMACRO"));
    w.call_code(cx, F_AI_ASK);
    w.call_code(cx, F_AI_PROMPT_0);
    w.call_code(cx, code("F_RECKEYMACRO"));
    let text = w
        .view
        .read_with(cx, |view, _| view.recorded_key_macro_text())
        .expect("a macro was recorded");
    assert!(text.contains("AiAsk()"), "{text}");
    assert!(text.contains("AiPrompt0()"), "{text}");
    // the call has no argument: the instruction is never part of the macro
    assert!(!text.contains("AiAsk(\""), "{text}");
}
