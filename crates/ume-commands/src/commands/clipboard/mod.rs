//! Clipboard commands: copy, cut and paste.

pub mod add_tail;
pub mod copy;
pub mod copy_addcrlf;
pub mod copy_cr;
pub mod copy_crlf;
pub mod copy_file_name;
pub mod copy_lf;
pub mod copy_lines;
pub mod copy_lines_as_passage;
pub mod copy_lines_with_line_number;
pub mod copy_path;
pub mod copy_tag;
pub mod copydirpath;
pub mod cut;
pub mod cut_line;
pub mod ins_text;
pub mod insboxtext;
pub mod line_cut_to_end;
pub mod line_cut_to_start;
pub mod paste;
pub mod pastebox;

#[cfg(test)]
mod copy_eol_tests;
#[cfg(test)]
mod tests;
