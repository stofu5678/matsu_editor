//! Sakura's file tree (`F_FILETREE`, ファイルツリー): the folders and files of the settings, shown
//! in the outline pane.
//!
//! * [`source`] picks the items: the settings of all types or of the type, or a project file
//!   found above the document (`LoadFileTreeSetting`).
//! * [`model`] is the tree the items make (`SetTreeFile`) and the nodes a listing adds (`SetTreeFileSub`).
//! * [`list`] lists one folder with the patterns of an item. It runs in the background: a node
//!   is listed when it is opened, and the window is not stopped by a large folder.
//!
//! The pane that shows the tree is [`crate::outline::pane::OutlinePane`]; the settings dialog
//! (`CDlgFileTree`) is [`crate::outline::file_tree_dialog`].

pub(crate) mod list;
pub(crate) mod model;
pub(crate) mod source;
#[cfg(test)]
mod tests;
