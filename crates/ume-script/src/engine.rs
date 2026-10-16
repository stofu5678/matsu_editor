//! The script engines that the host can run. An engine takes the script and the connection to
//! the editor, and binds the macro API (`ume_plugin::api::FUNCS`) into its language. The next
//! engines (QuickJS, MicroPython, mruby) add a module like `lua` and a line to `by_name`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::lua::Lua;
use crate::quickjs::QuickJs;
use crate::rpc::{Editor, Initialize};

pub trait Engine {
    /// Runs the script to its end. The `Err` is the text for the standard error: an error of the
    /// script with its line number.
    fn run(&self, init: &Initialize, editor: Rc<RefCell<dyn Editor>>) -> Result<(), String>;
}

/// The names that `--engine` accepts.
pub const ENGINES: &[&str] = &["lua", "quickjs"];

pub fn by_name(name: &str) -> Option<Box<dyn Engine>> {
    match name {
        "lua" => Some(Box::new(Lua)),
        "quickjs" => Some(Box::new(QuickJs)),
        _ => None,
    }
}
