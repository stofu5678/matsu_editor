//! Clipboard commands: copy, cut and paste.

pub mod copy;
pub mod copy_crlf;
pub mod copyfname;
pub mod copylines;
pub mod copylinesaspassage;
pub mod copylineswithlinenumber;
pub mod copypath;
pub mod copytag;
pub mod cut;
pub mod cut_line;
pub mod line_cut_to_end;
pub mod line_cut_to_start;
pub mod paste;

#[cfg(test)]
mod tests;
