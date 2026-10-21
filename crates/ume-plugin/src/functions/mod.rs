//! The macro functions that need the editor itself, not only a text model: the layout, the
//! clipboard, the state of the window, and the dialogs.
//!
//! `api::FUNCS` holds the functions that a [`MacroDocument`](crate::api::MacroDocument) can answer
//! (a text and a cursor). The functions here take an [`Editor`] (the command context and the
//! [`HostEnv`](crate::host::HostEnv)); the bridge runs them. Each module is one group of
//! Sakura's `m_MacroFuncInfoArr`, and every function names the Sakura source it follows
//! (`sakura_core/macro/CMacro.cpp`, `CMacro::HandleFunction`, unless it says otherwise).
//!
//! The catalog (`crate::catalog`) lists these functions with their parameter names, so the
//! generated code of every language and the bundled engines have them.
//!
//! # To add a function
//!
//! Add an [`EditorFunc`] to the `FUNCS` of the module of its group (or a new module listed in
//! [`all`]), and write its tests. The name must be a name of Sakura's table
//! (`ume_commands::sakura::macro_functions::MACRO_FUNCTIONS`), and its parameters must have the
//! types of the table row.

use serde_json::Value;

use crate::api::{Args, Param, Ty};
use crate::host::Editor;
use crate::protocol::RpcError;

mod clipboard;
mod cookie;
mod expand;
mod layout;
pub mod menu;
mod misc;
mod state;
pub mod window;

// The limits of the layout functions, which a key macro applies too.
pub use layout::{MAX_LINE_COLUMNS, MAX_TAB_WIDTH, MIN_LINE_COLUMNS};

/// One function that needs the editor. The fields are those of [`crate::api::Func`].
pub struct EditorFunc {
    pub name: &'static str,
    pub params: &'static [Param],
    pub result: Ty,
    pub doc: &'static str,
    pub run: fn(&mut Editor<'_, '_>, &Args) -> Result<Value, RpcError>,
}

/// Every function of this module, group by group.
pub fn all() -> impl Iterator<Item = &'static EditorFunc> {
    [
        state::FUNCS,
        layout::FUNCS,
        clipboard::FUNCS,
        cookie::FUNCS,
        misc::FUNCS,
        expand::FUNCS,
        window::FUNCS,
    ]
    .into_iter()
    .flatten()
}

/// The function with this name.
pub fn find(name: &str) -> Option<&'static EditorFunc> {
    all().find(|f| f.name == name)
}

/// Whether the function converts between layout and logical positions, or reports a position in
/// layout units, and so needs exact row counts: a window that lays a large document out in the
/// background completes the layout before it runs such a call (and only then).
pub fn needs_exact_layout(name: &str) -> bool {
    matches!(
        name,
        "GetSelectLineFrom"
            | "GetSelectColmFrom"
            | "GetSelectColumnFrom"
            | "GetSelectLineTo"
            | "GetSelectColmTo"
            | "GetSelectColumnTo"
            | "LayoutToLogicLineNum"
            | "LogicToLayoutLineNum"
            | "LineColumnToIndex"
            | "LineIndexToColumn"
            | "MoveCursor"
            | "MoveCursorLayout"
    )
}

/// Runs a call of one of these functions: `None` if the name is not one of them.
pub fn call(
    editor: &mut Editor<'_, '_>,
    name: &str,
    params: &[Value],
) -> Option<Result<Value, RpcError>> {
    let func = find(name)?;
    Some(
        crate::api::check_params(func.name, func.params, params)
            .and_then(|()| (func.run)(editor, &Args(params))),
    )
}

/// The error of a function that needs something this run does not have (no window, no view).
pub(crate) fn unavailable(what: &str) -> RpcError {
    RpcError::new(
        crate::bridge::UNAVAILABLE,
        format!("{what} is not available here: the macro runs without a window"),
    )
}

/// Turns a failure of the command context into the error of the call.
pub(crate) fn command(error: impl Into<ume_commands::command::CommandError>) -> RpcError {
    crate::bridge::command_error(error.into())
}

/// The length of `text` in UTF-16 code units: Sakura's columns and indexes of a line are
/// counted in `wchar_t` (UTF-16 on Windows), and so are this editor's macro functions.
pub(crate) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

#[cfg(test)]
pub(crate) mod testing {
    //! A document, a cursor and a layout for the tests of the functions.

    use serde_json::Value;
    use ume_commands::clipboard::MemoryClipboard;
    use ume_commands::context::CommandContext;
    use ume_commands::layout::PlainLayout;
    use ume_core::buffer::Document;
    use ume_core::cursor::Cursor;
    use ume_core::model::DocumentModel;

    use crate::host::{Editor, HostEnv, ViewInfo};
    use crate::protocol::RpcError;

    pub struct Fixture {
        pub model: DocumentModel,
        pub cursor: Cursor,
        pub layout: PlainLayout,
        pub clipboard: MemoryClipboard,
        pub env: HostEnv,
        pub path: Option<std::path::PathBuf>,
        /// The file a save will write, for the macro that runs before a save.
        pub save_path: Option<std::path::PathBuf>,
        pub overwrite: bool,
    }

    impl Fixture {
        pub fn new(text: &str) -> Self {
            let mut env = HostEnv::detached();
            env.view = Some(ViewInfo {
                rows: 30,
                columns: 80,
                top_row: 0,
            });
            env.char_code = Some(4);
            Self {
                model: DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap()),
                cursor: Cursor::default(),
                layout: PlainLayout::default(),
                clipboard: MemoryClipboard::default(),
                env,
                path: None,
                save_path: None,
                overwrite: false,
            }
        }

        /// Runs a function by name.
        pub fn call(&mut self, name: &str, params: Value) -> Result<Value, RpcError> {
            let params = params.as_array().expect("params are an array").clone();
            let mut cx = CommandContext::new(
                &mut self.model,
                &mut self.cursor,
                &mut self.layout,
                self.path.as_deref(),
            )
            .with_clipboard(&mut self.clipboard)
            .with_save_path(self.save_path.as_deref())
            .with_overwrite(self.overwrite);
            let mut editor = Editor {
                cx: &mut cx,
                env: &mut self.env,
            };
            super::call(&mut editor, name, &params)
                .unwrap_or_else(|| panic!("{name} is not an editor function"))
        }

        pub fn ok(&mut self, name: &str, params: Value) -> Value {
            self.call(name, params)
                .unwrap_or_else(|e| panic!("{name}: {}", e.message))
        }

        /// Selects from `(line, col)` to `(line, col)` (byte columns), the caret at the end.
        pub fn select(&mut self, from: (usize, usize), to: (usize, usize)) {
            use ume_core::buffer::Pos;
            self.cursor
                .select(Pos::new(from.0, from.1), Pos::new(to.0, to.1));
        }

        pub fn caret_at(&mut self, pos: (usize, usize)) {
            use ume_core::buffer::Pos;
            self.cursor.move_to(Pos::new(pos.0, pos.1), false);
        }
    }
}
