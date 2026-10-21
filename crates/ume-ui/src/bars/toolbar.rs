//! The toolbar (`CMainToolBar`, `sakura_core/window/CMainToolBar.cpp`; the help is
//! `help/sakura/res/HLP000010.html`).
//!
//! * The buttons are those of `[Toolbar]` in the settings file (`nToolBarButtonNum`,
//!   `nTBB[000]`...: `CShareData_IO::ShareData_IO_Toolbar`), or Sakura's default list
//!   ([`tool_buttons::DEFAULT_BUTTONS`], `CShareData::InitToolButtons`) for what the file does not
//!   say. A button is a number; see [`tool_buttons`].
//! * Separators that follow each other count as one, and the virtual wrap button ends the line
//!   at the button before it (`CMainToolBar::CreateToolBar`).
//! * A click runs the button's function on the active view. A button whose function has no
//!   command here is grayed out. The tooltip is the name of the function and the keys bound to it
//!   (`CEditWnd::GetTooltipText`).
//! * The search box (`F_SEARCH_BOX`) is a field with the search history, where the list has it
//!   ([`super::search_box`]). The open button (`F_FILEOPEN_DROPDOWN`) has a drop-down arrow beside
//!   it that shows the recent files and folders ([`super::open_menu`]); the button itself opens a
//!   file, as `Command_FILEOPEN` does.
//! * Not built: the plugin buttons.
//! * A button is pressed while `IsFuncChecked` says so (`TBSTATE_CHECKED`, `CMainToolBar.cpp`) and
//!   gray while `IsFuncEnable` says no (`TBSTATE_ENABLED`): the state of the window the toolbar
//!   is in ([`crate::menu_state::FuncStates`]), read each time the window is drawn.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Context, Global, InteractiveElement, IntoElement, ParentElement,
    Pixels, Point, StatefulInteractiveElement, Styled, Window, canvas, div, img, point,
    prelude::FluentBuilder, px, rgb,
};
use gpui_component::tooltip::Tooltip;
use ume_config::Profile;

use super::tool_buttons::{DEFAULT_BUTTONS, FUNCTIONS_OF_BUTTONS, MAX_BUTTONS, WRAP};
use super::{display_style, icons, is_available};
use crate::app::Workspace;

/// `F_DUMMY_MAX_CODE`: the function codes up to this one are placeholders with no picture.
const DUMMY_MAX_CODE: u16 = 10;
/// `F_SEARCH_BOX`, which is a combo box, not a button.
const SEARCH_BOX: u16 = 30908;
/// `F_FILEOPEN_DROPDOWN`.
const FILEOPEN_DROPDOWN: u16 = 30108;
/// `F_FILEOPEN`.
const FILEOPEN: u16 = 30102;

/// The button numbers of the toolbar, as the settings file says (or Sakura's default).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolbarButtons(pub Vec<i32>);

impl Global for ToolbarButtons {}

impl Default for ToolbarButtons {
    fn default() -> Self {
        Self(DEFAULT_BUTTONS.iter().map(|&n| i32::from(n)).collect())
    }
}

impl ToolbarButtons {
    /// `ShareData_IO_Toolbar` reading: `nToolBarButtonNum` entries `nTBB[000]`...; an entry the file
    /// does not have keeps the default list's value at that place (0 past its end). A value with a
    /// `/` is a plugin command, which UME Editor has none of: it is an undefined button.
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        const SECTION: &str = "Toolbar";
        let default = Self::default().0;
        let count = profile
            .get_i32(SECTION, "nToolBarButtonNum")
            .map_or(default.len(), |n| n.clamp(0, MAX_BUTTONS as i32) as usize);
        let numbers = (0..count)
            .map(|i| match profile.get(SECTION, &format!("nTBB[{i:03}]")) {
                Some(text) if text.contains('/') => -1,
                // `_wtoi`: the number at the start, else 0.
                Some(text) => leading_int(text),
                None => default.get(i).copied().unwrap_or(0),
            })
            .collect();
        Self(numbers)
    }
}

/// `_wtoi`: the integer at the start of `text` (after spaces), 0 when there is none.
fn leading_int(text: &str) -> i32 {
    let text = text.trim_start();
    let end = text
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map_or(text.len(), |(i, _)| i);
    text[..end].parse().unwrap_or(0)
}

/// Starts the buttons from the file that was read.
pub(crate) fn install(cx: &mut App, user: &crate::settings::UserSettings) {
    cx.set_global(
        user.profile()
            .map_or_else(ToolbarButtons::default, ToolbarButtons::from_profile),
    );
}

/// The buttons now.
fn buttons(cx: &App) -> ToolbarButtons {
    cx.try_global::<ToolbarButtons>()
        .cloned()
        .unwrap_or_default()
}

/// A button of the toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Button {
    /// The number in the settings file.
    pub number: u16,
    /// The function the button stands for (0: none, a button that is always gray).
    pub code: u16,
    /// The picture: the index of the icon in `mytool.bmp`.
    pub icon: Option<usize>,
}

impl Button {
    /// The function a click runs. The open button's drop-down function only differs in the arrow,
    /// which is not built, so it runs `F_FILEOPEN`.
    pub(super) fn click_code(&self) -> u16 {
        if self.code == FILEOPEN_DROPDOWN {
            FILEOPEN
        } else {
            self.code
        }
    }
}

/// How a button looks: gray when its function cannot run, pressed when `IsFuncChecked` says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Look {
    pub enabled: bool,
    pub pressed: bool,
}

/// `CMainToolBar::UpdateToolbar`: a function without a command is gray, and so is one that
/// `IsFuncEnable` refuses; a button is pressed while `IsFuncChecked` is true.
pub(crate) fn look(code: u16, states: &crate::menu_state::FuncStates) -> Look {
    Look {
        enabled: is_available(code) && states.is_enabled(code),
        pressed: states.is_checked(code),
    }
}

/// An item of a line of the toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Item {
    Separator,
    Button(Button),
    /// The search box (`F_SEARCH_BOX`).
    SearchBox,
}

/// `CMenuDrawer::getButton`: what a number stands for. A number outside the table is an undefined
/// button (gray, no picture).
fn item_of(number: i32) -> Item {
    if number == 0 {
        return Item::Separator;
    }
    let undefined = Item::Button(Button {
        number: 0,
        code: 0,
        icon: None,
    });
    let Some(index) = usize::try_from(number - 1)
        .ok()
        .filter(|&i| i < FUNCTIONS_OF_BUTTONS.len())
    else {
        return undefined;
    };
    let code = FUNCTIONS_OF_BUTTONS[index];
    Item::Button(Button {
        number: number as u16,
        code,
        // `(F_DUMMY_MAX_CODE < funcCode) ? imageIndex : -1`
        icon: (code > DUMMY_MAX_CODE).then_some(index),
    })
}

/// The lines of the toolbar for the button numbers (`CMainToolBar::CreateToolBar`): separators
/// that follow each other become one, the wrap button ends the line at the button before it
/// (and removes the separator in between), and only the first search box is made
/// (`if( m_hwndSearchBox ) break;`).
pub(super) fn arrange(numbers: &[i32]) -> Vec<Vec<Item>> {
    // Each item with whether a line ends after it.
    let mut cells: Vec<(Item, bool)> = Vec::new();
    for &number in numbers {
        let mut item = item_of(number);
        if matches!(item, Item::Button(b) if b.code == SEARCH_BOX) {
            if cells.iter().any(|(cell, _)| *cell == Item::SearchBox) {
                continue;
            }
            item = Item::SearchBox;
        }
        let is_separator = matches!(item, Item::Separator) || number == i32::from(WRAP);
        if is_separator && matches!(cells.last(), Some((Item::Separator, _))) {
            cells.pop();
        }
        if number == i32::from(WRAP) {
            if let Some(last) = cells.last_mut() {
                last.1 = true;
            }
            continue;
        }
        cells.push((item, false));
    }
    let mut lines = vec![Vec::new()];
    for (item, ends_line) in cells {
        if let Some(line) = lines.last_mut() {
            line.push(item);
        }
        if ends_line {
            lines.push(Vec::new());
        }
    }
    lines.retain(|line| !line.is_empty());
    lines
}

/// The tooltip of function `code`: its name and, on the lines after it, the keys bound to it
/// (`CEditWnd::GetTooltipText`).
pub(super) fn tooltip_text(code: u16, cx: &App) -> String {
    let mut text = crate::func_name::FuncNames::new().name(cx, code);
    let style = display_style();
    let chords = crate::app::with_key_map(cx, |map| map.shortcuts_for(code));
    for chord in chords {
        text.push_str("\n        ");
        text.push_str(&crate::menu::shortcut_text(chord, style));
    }
    text
}

impl Workspace {
    /// The toolbar at 100 %, if it is shown (the tests; the window uses the scale of its screen).
    #[cfg(test)]
    pub(crate) fn render_toolbar(
        &self,
        topmost: bool,
        cx: &mut gpui::Context<Self>,
    ) -> Option<AnyElement> {
        self.render_toolbar_at_scale(topmost, 1.0, cx)
    }

    /// The toolbar for a screen with `scale_factor`: the icons are made sharp for it
    /// ([`icons::pixel_factor`]).
    pub(crate) fn render_toolbar_at_scale(
        &self,
        topmost: bool,
        scale_factor: f32,
        cx: &mut gpui::Context<Self>,
    ) -> Option<AnyElement> {
        if !super::current(cx).toolbar {
            return None;
        }
        let lines = arrange(&buttons(cx).0);
        let states = self.func_states(topmost, cx.entity_id(), cx);
        let mut bar = super::semantics::toolbar(div().id("tool-bar"))
            .debug_selector(|| "tool-bar".into())
            .flex()
            .flex_col()
            .w_full()
            .px_0p5()
            .bg(rgb(0xf0f0f0))
            .border_b_1()
            .border_color(rgb(0xd0d0d0));
        for (row, line) in lines.into_iter().enumerate() {
            let mut line_el = div().flex().flex_row().items_center().h(px(24.));
            for (column, item) in line.into_iter().enumerate() {
                line_el = line_el.child(match item {
                    Item::Separator => div().w(px(8.)).into_any_element(),
                    Item::SearchBox => self.search_box.clone().into_any_element(),
                    Item::Button(button) => {
                        self.render_tool_button(row, column, button, &states, scale_factor, cx)
                    }
                });
            }
            bar = bar.child(line_el);
        }
        Some(bar.into_any_element())
    }

    fn render_tool_button(
        &self,
        row: usize,
        column: usize,
        button: Button,
        states: &crate::menu_state::FuncStates,
        scale_factor: f32,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        let code = button.click_code();
        // A function without a command is gray, and so is one that `IsFuncEnable` refuses.
        let Look { enabled, pressed } = look(code, states);
        let image = button
            .icon
            .and_then(|index| icons::icon(index, icons::pixel_factor(scale_factor)));
        let name = crate::func_name::FuncNames::new().name(cx, code);
        let main = super::semantics::tool_button(
            div().id(("tool-button", row * 1000 + column)),
            &name,
            crate::menu_state::is_checkable(code).then_some(pressed),
        )
        .debug_selector(|| format!("tool-button-{}", button.number))
        .flex()
        .items_center()
        .justify_center()
        .w(px(24.))
        .h(px(22.))
        .rounded_sm()
        .when(pressed, |this| {
            this.bg(rgb(0xc4d8f0))
                .border_1()
                .border_color(rgb(0x7fa3d1))
        })
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|s| s.bg(rgb(0xd8e6f5)))
                .active(|s| s.bg(rgb(0xb8d0ec)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.run_after_event(code, window, cx);
                }))
        })
        .when(!enabled, |this| this.opacity(0.4))
        .when_some(image, |this, image| {
            this.child(
                img(image)
                    .w(px(icons::logical_size(scale_factor)))
                    .h(px(icons::logical_size(scale_factor))),
            )
        })
        .when(button.code > DUMMY_MAX_CODE, |this| {
            let shown = button.code;
            this.tooltip(move |window, cx| Tooltip::new(tooltip_text(shown, cx)).build(window, cx))
        });
        let main = crate::a11y::disabled(main, !enabled).into_any_element();
        if button.code != FILEOPEN_DROPDOWN {
            return main;
        }
        // `TBSTYLE_DROPDOWN`: the arrow beside the button shows the menu under the whole button
        // (`CreateFileDropDownMenu`: `rc.left`, `rc.bottom`).
        let bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
        let place = bounds.clone();
        let arrow = super::semantics::drop_down_arrow(
            div().id(("tool-dropdown", row * 1000 + column)),
            &name,
        )
        .debug_selector(|| format!("tool-dropdown-{}", button.number))
        .flex()
        .items_center()
        .justify_center()
        .w(px(12.))
        .h(px(22.))
        .rounded_sm()
        .text_size(px(7.))
        .child("▼")
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|s| s.bg(rgb(0xd8e6f5)))
                .active(|s| s.bg(rgb(0xb8d0ec)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    let rect = bounds.get();
                    this.leave_search_box(window, cx);
                    this.open_file_dropdown(point(rect.left(), rect.bottom()), window, cx);
                }))
        })
        .when(!enabled, |this| this.opacity(0.4));
        let arrow = crate::a11y::disabled(arrow, !enabled);
        div()
            .relative()
            .flex()
            .items_center()
            .child(main)
            .child(arrow)
            // Tells the arrow where the button is.
            .child(
                canvas(move |rect, _, _| place.set(rect), |_, (), _, _| {})
                    .absolute()
                    .size_full(),
            )
            .into_any_element()
    }

    /// `CEditWnd::OnCommand`: a command from the toolbar or a menu moves the focus from the search
    /// box to the view first, so that the keys that follow go to the text.
    pub(crate) fn leave_search_box(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_box.read(cx).is_focused(window, cx) {
            let editor = self.editor().clone();
            self.search_box
                .update(cx, |b, cx| b.leave(&editor, window, cx));
        }
    }

    /// Whether this window shows the search box: the toolbar is shown and has `F_SEARCH_BOX`.
    pub(crate) fn has_search_box(&self, cx: &App) -> bool {
        super::current(cx).toolbar
            && arrange(&buttons(cx).0)
                .iter()
                .flatten()
                .any(|item| *item == Item::SearchBox)
    }

    /// `Command_SEARCH_BOX`: moves the focus into the search box, if there is one.
    pub(crate) fn focus_search_box(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_search_box(cx) {
            let editor = self.editor().clone();
            self.search_box
                .update(cx, |b, cx| b.focus_with(&editor, window, cx));
        }
    }

    /// `TBN_DROPDOWN`: the menu of the recent files and folders with its corner at `at`, once this
    /// event is handled.
    fn open_file_dropdown(&self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor().clone();
        window.defer(cx, move |window, app| {
            editor.update(app, |view, cx| view.show_file_dropdown(at, window, cx));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buttons_of(numbers: &[i32]) -> Vec<Vec<Option<u16>>> {
        arrange(numbers)
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|item| match item {
                        Item::Separator => None,
                        Item::Button(b) => Some(b.code),
                        Item::SearchBox => Some(SEARCH_BOX),
                    })
                    .collect()
            })
            .collect()
    }

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    #[test]
    fn without_settings_the_toolbar_is_sakuras_default() {
        let buttons = ToolbarButtons::from_profile(&Profile::new());
        assert_eq!(buttons, ToolbarButtons::default());
        let lines = buttons_of(&buttons.0);
        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        // New, open (drop-down), save, save as, | undo, redo, | ... | command list.
        assert_eq!(
            &line[..4],
            &[Some(30101), Some(30108), Some(30103), Some(30104)]
        );
        assert_eq!(line[4], None);
        assert_eq!(&line[5..7], &[Some(30210), Some(30211)]);
        assert_eq!(line.last(), Some(&Some(31445)));
        // 25 numbers, 6 of them separators.
        assert_eq!(line.iter().filter(|b| b.is_none()).count(), 6);
        assert_eq!(line.len(), 25);
    }

    #[test]
    fn a_toolbar_list_in_the_file_replaces_the_default() {
        let p = profile(
            "[Toolbar]\r\nnToolBarButtonNum=4\r\nnTBB[000]=3\r\nnTBB[001]=0\r\nnTBB[002]=33\r\n\
             nTBB[003]=plugin/command\r\n",
        );
        let buttons = ToolbarButtons::from_profile(&p);
        assert_eq!(buttons.0, [3, 0, 33, -1]);
        let lines = buttons_of(&buttons.0);
        // Save, separator, undo, and an undefined button (no function, gray).
        assert_eq!(lines, [vec![Some(30103), None, Some(30210), Some(0)]]);
    }

    #[test]
    fn an_entry_the_file_lacks_keeps_the_default_one() {
        let p = profile("[Toolbar]\r\nnToolBarButtonNum=3\r\nnTBB[001]=4\r\n");
        assert_eq!(ToolbarButtons::from_profile(&p).0, [1, 4, 3]);
        let p = profile("[Toolbar]\r\nnToolBarButtonNum=0\r\n");
        assert!(ToolbarButtons::from_profile(&p).0.is_empty());
        let p = profile("[Toolbar]\r\nnToolBarButtonNum=3\r\nnTBB[000]= 25abc\r\nnTBB[001]=x\r\n");
        assert_eq!(ToolbarButtons::from_profile(&p).0, [25, 0, 3]);
    }

    #[test]
    fn separators_merge_and_the_wrap_button_ends_the_line() {
        // save, | |, undo  -> one separator
        assert_eq!(
            buttons_of(&[3, 0, 0, 33]),
            [vec![Some(30103), None, Some(30210)]]
        );
        // save, |, wrap, undo -> the separator before the wrap goes, the line ends after save
        assert_eq!(
            buttons_of(&[3, 0, i32::from(WRAP), 33]),
            [vec![Some(30103)], vec![Some(30210)]]
        );
        // a wrap at the start does nothing; a wrap at the end leaves no empty line
        assert_eq!(buttons_of(&[i32::from(WRAP), 3]), [vec![Some(30103)]]);
        assert_eq!(buttons_of(&[3, i32::from(WRAP)]), [vec![Some(30103)]]);
    }

    #[test]
    fn numbers_outside_the_table_are_undefined_buttons_and_only_the_first_search_box_is_made() {
        assert_eq!(
            buttons_of(&[-5, 9999, 3]),
            [vec![Some(0), Some(0), Some(30103)]]
        );
        let search_box_number = FUNCTIONS_OF_BUTTONS
            .iter()
            .position(|&c| c == SEARCH_BOX)
            .map(|i| i as i32 + 1)
            .unwrap();
        assert_eq!(
            buttons_of(&[3, search_box_number, 33, search_box_number]),
            [vec![Some(30103), Some(SEARCH_BOX), Some(30210)]]
        );
    }

    #[test]
    fn a_button_has_the_icon_of_its_place_in_the_table() {
        let Item::Button(save) = item_of(3) else {
            panic!()
        };
        assert_eq!((save.code, save.icon), (30103, Some(2)));
        // The open button's picture is the 25th icon; a click opens a file.
        let Item::Button(open) = item_of(25) else {
            panic!()
        };
        assert_eq!((open.code, open.icon), (30108, Some(24)));
        assert_eq!(open.click_code(), FILEOPEN);
        // A free place in the table has no function and no picture.
        let Item::Button(free) = item_of(481) else {
            panic!()
        };
        assert_eq!((free.code, free.icon), (0, None));
    }
}
