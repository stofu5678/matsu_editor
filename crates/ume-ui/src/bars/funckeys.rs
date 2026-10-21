//! The function key bar (`CFuncKeyWnd`, `sakura_core/func/CFuncKeyWnd.cpp`).
//!
//! * 12 buttons, F1 to F12. The label of a button is the name of the function the key map gives
//!   that key with the modifiers (Shift, Ctrl, Alt) held now (`OnTimer`: `CKeyBind::GetFuncCode`
//!   and `Funccode2Name`); it changes as the modifiers change. A key with no function has an empty
//!   label.
//! * A click runs the function on the active view (`OnCommand`). A button whose function has no
//!   command here is grayed out (Sakura grays by `IsFuncEnable`, which UME Editor does not have
//!   yet: only the missing command is checked).
//! * A gap of 12 pixels separates the groups of `nFUNCKEYWND_GroupNum` buttons (`OnSize`).
//! * The bar is at the top (under the toolbar) or at the bottom (above the status bar):
//!   `nFUNCKEYWND_Place`.

use gpui::{
    AnyElement, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    Window, div, prelude::FluentBuilder, px, rgb,
};
use ume_commands::keymap::{Chord, Key, Modifiers, Resolved};

use super::is_available;
use crate::app::Workspace;

/// The number of buttons.
const COUNT: u8 = 12;
/// The gap between the groups (`nX += 12`).
const GROUP_GAP: f32 = 12.;

/// A button of the bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FuncKey {
    /// The function of the key (0: none).
    pub code: u16,
    pub label: String,
}

/// The modifiers of the keyboard in the terms of the key map.
pub(super) fn modifiers_of(held: gpui::Modifiers) -> Modifiers {
    Modifiers {
        shift: held.shift,
        ctrl: held.control,
        alt: held.alt,
        meta: held.platform,
    }
}

/// The 12 buttons for the modifiers held now, from `map` (`CFuncKeyWnd::OnTimer`).
pub(super) fn labels(
    map: &ume_commands::keymap::KeyMap,
    held: Modifiers,
    name: &dyn Fn(u16) -> String,
) -> Vec<FuncKey> {
    (1..=COUNT)
        .map(|n| {
            let code = Key::function(n)
                .map(|key| map.resolve(Chord::new(key, held)))
                .and_then(|resolved| match resolved {
                    Resolved::Code(code) => Some(code),
                    Resolved::Unbound | Resolved::PassThrough => None,
                })
                .unwrap_or(0);
            let label = if code == 0 { String::new() } else { name(code) };
            FuncKey { code, label }
        })
        .collect()
}

/// Whether a gap comes before button `index` (counted from 0) with groups of `group_num`
/// (`0 < i && 0 == i % m_nButtonGroupNum`).
pub(super) fn gap_before(index: usize, group_num: usize) -> bool {
    index > 0 && index.is_multiple_of(group_num)
}

impl Workspace {
    /// The function key bar, if it is shown at `place`.
    pub(crate) fn render_funckeys(
        &self,
        place: super::FuncKeyPlace,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) -> Option<AnyElement> {
        let settings = super::current(cx);
        if !settings.funckeys || settings.funckey_place != place {
            return None;
        }
        let held = modifiers_of(window.modifiers());
        let names = crate::func_name::FuncNames::new();
        let keys = crate::app::with_key_map(cx, |map| labels(map, held, &|c| names.name(cx, c)));
        let mut bar = super::semantics::funckey_bar(div().id("function-key-bar"))
            .debug_selector(|| "function-key-bar".into())
            .flex()
            .flex_row()
            .w_full()
            .h(px(26.))
            .py(px(1.))
            .px(px(1.))
            .bg(rgb(0xf0f0f0))
            .text_size(px(12.))
            .text_color(rgb(0x1e1e1e));
        bar = match place {
            super::FuncKeyPlace::Top => bar.border_b_1(),
            super::FuncKeyPlace::Bottom => bar.border_t_1(),
        }
        .border_color(rgb(0xd0d0d0));
        for (i, key) in keys.into_iter().enumerate() {
            let code = key.code;
            let enabled = is_available(code);
            bar = bar.child(
                super::semantics::funckey(div().id(("function-key", i)), i + 1, &key.label)
                    .debug_selector(|| format!("function-key-{i}"))
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .px_1()
                    .mr(px(1.))
                    .when(gap_before(i, settings.funckey_group_num), |this| {
                        this.ml(px(GROUP_GAP))
                    })
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(rgb(0xc0c0c0))
                    .rounded_sm()
                    .bg(rgb(0xfafafa))
                    .when(enabled, |this| {
                        this.cursor_pointer()
                            .hover(|s| s.bg(rgb(0xd8e6f5)))
                            .active(|s| s.bg(rgb(0xb8d0ec)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.run_after_event(code, window, cx);
                            }))
                    })
                    .when(!enabled, |this| this.text_color(rgb(0x9a9a9a)))
                    .child(key.label),
            );
        }
        // The labels follow the modifiers held: draw again when they change.
        Some(
            bar.on_modifiers_changed(cx.listener(|_, _, _, cx| cx.notify()))
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use ume_commands::keymap::{KeyMap, KeyMapOptions, Platform};

    use super::*;

    fn map() -> KeyMap {
        KeyMap::new(KeyMapOptions::for_platform(Platform::Other))
    }

    fn names(code: u16) -> String {
        crate::func_name::funccode_to_name(
            code,
            &ume_config::settings::CustomMenus::default(),
            &ume_config::settings::macros::RegisteredMacros::default(),
        )
    }

    fn name_of(keys: &[FuncKey], n: usize) -> &str {
        &keys[n - 1].label
    }

    #[test]
    fn the_labels_are_the_functions_of_f1_to_f12() {
        let keys = labels(&map(), Modifiers::NONE, &names);
        assert_eq!(keys.len(), 12);
        // Sakura's default table: F3 is "next search", F4 is "close"... whichever the table says.
        let f3 = map().resolve(Chord::new(Key::function(3).unwrap(), Modifiers::NONE));
        let Resolved::Code(code) = f3 else { panic!() };
        assert_eq!(keys[2].code, code);
        assert_eq!(name_of(&keys, 3), names(code));
        assert!(!name_of(&keys, 3).is_empty());
    }

    #[test]
    fn the_labels_change_with_shift_and_ctrl() {
        let none = labels(&map(), Modifiers::NONE, &names);
        let shift = labels(&map(), Modifiers::SHIFT, &names);
        let ctrl = labels(&map(), Modifiers::CTRL, &names);
        // Sakura's default: F3 searches forward, Shift+F3 backward, Ctrl+F3 clears the marks.
        assert_ne!(none[2].code, shift[2].code);
        assert_ne!(none[2].code, ctrl[2].code);
        assert_ne!(shift[2].code, ctrl[2].code);
        assert_eq!(shift[2].code, 30903); // F_SEARCH_PREV
        assert_eq!(none[2].code, 30902); // F_SEARCH_NEXT
    }

    #[test]
    fn a_key_without_a_function_has_an_empty_label() {
        // Sakura's default table has nothing on Ctrl+Shift+Alt+F1.
        let all = Modifiers::CTRL.with_shift().with_alt();
        let keys = labels(&map(), all, &names);
        assert_eq!(
            keys[0],
            FuncKey {
                code: 0,
                label: String::new()
            }
        );
    }

    #[test]
    fn the_key_map_of_the_user_decides() {
        use ume_commands::keymap::UserRow;
        let mut map = map();
        let mut functions = [0u16; 8];
        functions[0] = 30101; // F_FILENEW on F1
        map.apply_user_rows(&[UserRow {
            key: Key::function(1).unwrap(),
            functions,
        }]);
        let keys = labels(&map, Modifiers::NONE, &names);
        assert_eq!(keys[0].code, 30101);
        assert_eq!(keys[0].label, "新規作成");
        assert_eq!(labels(&map, Modifiers::SHIFT, &names)[0].code, 0);
    }

    #[test]
    fn a_gap_comes_between_the_groups() {
        let gaps: Vec<usize> = (0..12).filter(|&i| gap_before(i, 4)).collect();
        assert_eq!(gaps, [4, 8]);
        let gaps: Vec<usize> = (0..12).filter(|&i| gap_before(i, 1)).collect();
        assert_eq!(gaps, (1..12).collect::<Vec<_>>());
        assert!((0..12).all(|i| !gap_before(i, 12)));
    }

    #[test]
    fn the_modifiers_of_the_keyboard_are_translated() {
        let held = gpui::Modifiers {
            shift: true,
            control: true,
            ..Default::default()
        };
        assert_eq!(modifiers_of(held), Modifiers::CTRL.with_shift());
    }
}
