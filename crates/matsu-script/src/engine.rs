//! The script engines that the host can run. An engine takes the script and the connection to
//! the editor, and binds the macro API (`matsu_plugin::catalog::entries()`) into its language. The next
//! engines (QuickJS, MicroPython, mruby) add a module like `lua` and a line to `by_name`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::lua::Lua;
use crate::micropython::MicroPython;
use crate::mruby::Mruby;
use crate::quickjs::QuickJs;
use crate::rpc::{Editor, Initialize, PluginInit, PluginLink};

pub trait Engine {
    /// Runs the script to its end. The `Err` is the text for the standard error: an error of the
    /// script with its line number.
    fn run(&self, init: &Initialize, editor: Rc<RefCell<dyn Editor>>) -> Result<(), String>;

    /// Serves a plugin: answers each `plugin/invoke` until the editor closes the connection
    /// (ADR-0026). Only the engines that say so can; the others are for macros.
    fn serve(&self, _init: &PluginInit, _link: Rc<RefCell<dyn PluginLink>>) -> Result<(), String> {
        Err("this engine cannot run plugins yet (only quickjs can)".into())
    }
}

/// The names that `--engine` accepts.
pub const ENGINES: &[&str] = &["lua", "quickjs", "mruby", "micropython"];

pub fn by_name(name: &str) -> Option<Box<dyn Engine>> {
    match name {
        "lua" => Some(Box::new(Lua)),
        "quickjs" => Some(Box::new(QuickJs)),
        "mruby" => Some(Box::new(Mruby)),
        "micropython" => Some(Box::new(MicroPython)),
        _ => None,
    }
}
