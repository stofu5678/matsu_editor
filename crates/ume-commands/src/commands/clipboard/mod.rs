//! Clipboard commands: copy, cut and paste.

pub mod copy;
pub mod copy_crlf;
pub mod copy_file_name;
pub mod copy_lines;
pub mod copy_lines_as_passage;
pub mod copy_lines_with_line_number;
pub mod copy_path;
pub mod copy_tag;
pub mod cut;
pub mod cut_line;
pub mod line_cut_to_end;
pub mod line_cut_to_start;
pub mod paste;

#[cfg(test)]
mod tests;
