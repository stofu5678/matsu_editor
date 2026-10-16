//! The script host: the JSON-RPC side of a macro process (`rpc`), the engines (`engine`, `lua`)
//! and the `ume-script` program (`main.rs`) that ties them together.

pub mod engine;
pub mod lua;
pub mod quickjs;
pub mod rpc;
