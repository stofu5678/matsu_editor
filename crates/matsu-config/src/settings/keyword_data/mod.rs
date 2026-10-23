//! The words of Sakura's built-in keyword sets, one module per set (generated from the arrays
//! `g_ppszKeywords*` of `sakura_core/types/CType_*.cpp`). They are registered by [`super::keywords`].

pub mod awk;
pub mod bat;
pub mod cobol;
pub mod corba_idl;
pub mod cpp;
pub mod html;
pub mod java;
pub mod pascal;
pub mod perl;
pub mod perl2;
pub mod plsql;
pub mod python;
pub mod rtf;
pub mod tex;
pub mod tex2;
pub mod vb;
pub mod vb2;

// MATSU Editor's own sets (ADR-0017), written from language specifications; see `super::extra_types`.
pub mod css;
pub mod go;
pub mod javascript;
pub mod json;
pub mod python_builtins;
pub mod ruby;
pub mod rust;
pub mod shell;
pub mod typescript;
pub mod yaml;
