//! `Entity<SharedDocument>`: the document layer of ADR-0013 section 3.
//!
//! `matsu_core::model::DocumentModel` is a plain struct that knows nothing of GPUI. GPUI's
//! `EventEmitter` cannot be implemented for it here (neither the trait nor the type is ours), so
//! the entity holds it in this small wrapper, together with what the views of one document share:
//! the path and a version number that counts the changes.
//!
//! The version solves a problem that section 3 does not mention. A view that runs a command
//! brings its own layout up to date at once (section 4). The `Changed` event that the same change
//! emits reaches that view only after the outermost update, and it must not lay the text out a
//! second time. So the view remembers the version its layout matches and ignores the events that
//! carry it; the other views of the document see a newer version and lay out again.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{Context, EventEmitter};
use matsu_core::bookmarks::TextBreaks;
use matsu_core::buffer::{Document, Eol, Pos};
use matsu_core::cursor::Cursor;
use matsu_core::encoding::Encoding;
use matsu_core::file::{Progress, SaveOptions, SaveReport, TextFile, UnmappableReport};
use matsu_core::model::DocumentModel;

use crate::file_ops::lock::{FileLockState, LockFailure};
use crate::file_watch::state::{self, DiskState, WatchMode};
use crate::settings::NewDocumentDefaults;

/// `m_nNonameSequences`: how many numbers were given to untitled windows.
#[cfg(not(test))]
static UNTITLED_SEQUENCE: AtomicU32 = AtomicU32::new(0);

// Every test runs on its own thread, so each one counts from 1 whatever the others do.
#[cfg(test)]
thread_local! {
    static UNTITLED_SEQUENCE: AtomicU32 = const { AtomicU32::new(0) };
}

fn next_untitled_number() -> u32 {
    #[cfg(not(test))]
    return UNTITLED_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    #[cfg(test)]
    return UNTITLED_SEQUENCE.with(|n| n.fetch_add(1, Ordering::Relaxed)) + 1;
}

/// What the document tells its views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentEvent {
    /// The text changed. `version` is the number [`SharedDocument::version`] has now.
    Changed { version: u64 },
    /// The document got the type (タイプ別設定) number `index`. Every view of the document takes
    /// its settings: Sakura keeps the type per document (`CEditDoc::m_cDocType`), not per pane.
    TypeChanged { index: usize },
}

/// The document of the editor with its undo history, shared by the views that show it.
pub struct SharedDocument {
    model: DocumentModel,
    path: Option<PathBuf>,
    version: u64,
    /// Typing overwrites instead of inserting. Sakura keeps the mode per document
    /// (`CDocEditor::m_bInsMode`) and starts in insert mode.
    overwrite: bool,
    /// View mode (`F_VIEWMODE`): every edit is refused. Sakura keeps it per window
    /// (`CAppMode::m_bViewMode`), clears it for a new document and keeps it when the file is
    /// reopened.
    view_mode: bool,
    /// The user chose the document type (`F_CHANGETYPE`): a new file name does not change it until
    /// a file is loaded (`m_nSettingTypeLocked`, `CDocType::LockDocumentType`). Per document, so
    /// all panes of it agree.
    type_locked: bool,
    /// What saving needs besides the text: the encoding, the BOM and the line ending. Its
    /// `document` is a placeholder; the text is `model`'s (see [`with_file`](Self::with_file)).
    file: TextFile,
    /// The encoding and the BOM the file had when it was read or last written
    /// (`eCharCodeLoad`, `bBomExistLoad`), to tell that `F_CHG_CHARSET` changed them.
    loaded_format: (Encoding, bool),
    /// The file on disk as this editor last saw it: the time stamp for the watching of changes and
    /// whether the file can be written (`crate::file_watch`).
    disk: DiskState,
    /// The lock that keeps other programs from writing (or reading) the file while it is open
    /// (`crate::file_ops::lock`).
    lock: FileLockState,
    /// The number of an untitled document (`EditNode::m_nId`), shown as `(無題)2` in the lists
    /// and as `$n` in the titles. See [`number_untitled`](Self::number_untitled).
    untitled_number: Option<u32>,
}

impl EventEmitter<DocumentEvent> for SharedDocument {}

/// Gives the document the setting `bEnableExtEol` as it is now. Sakura reads the setting where it
/// sets a line (`CDocLine::SetDocLineString`), so text edited after a change of the setting follows
/// it and the existing lines stay as they are.
fn sync_ext_eol(model: &DocumentModel) {
    model
        .doc()
        .set_ext_eol(matsu_commands::edit_options::get().ext_eol);
}

impl SharedDocument {
    pub fn new(model: DocumentModel, path: Option<PathBuf>) -> Self {
        sync_ext_eol(&model);
        let file = TextFile::new_untitled(Encoding::Utf8, false, model.new_line());
        Self {
            model,
            path,
            version: 0,
            overwrite: false,
            view_mode: false,
            type_locked: false,
            loaded_format: (file.encoding, file.bom),
            file,
            disk: DiskState::default(),
            lock: FileLockState::default(),
            untitled_number: None,
        }
    }

    /// A document from an opened file ([`crate::open::open_text_file`]): the text goes into the
    /// model, and the encoding, BOM and line ending stay for the status bar and for saving.
    pub fn from_text_file(mut file: TextFile) -> Self {
        let doc = std::mem::take(&mut file.document);
        let mut model = DocumentModel::new(doc);
        sync_ext_eol(&model);
        model.set_new_line(file.new_line);
        let disk = DiskState::of_path(file.path.as_deref());
        Self {
            model,
            path: file.path.clone(),
            version: 0,
            overwrite: false,
            view_mode: false,
            type_locked: false,
            loaded_format: (file.encoding, file.bom),
            file,
            disk,
            lock: FileLockState::default(),
            untitled_number: None,
        }
    }

    /// Replaces the whole document (open, close, reopen) and tells the views. The version keeps
    /// counting up across the replacement: a read that started before cannot mistake the new
    /// document for the one it saw, and a view cannot skip the re-layout for a reused number.
    pub fn replace_with(&mut self, new: SharedDocument, cx: &mut Context<Self>) -> u64 {
        let version = self.version;
        // The number belongs to the window, not to the file (`EditNode::m_nId` stays when the
        // file is closed or another is opened): the new document keeps it, and an untitled one
        // that has none gets it now (`CDocFileOperation::FileClose`, `GetNoNameNumber`).
        let number = self.untitled_number;
        *self = new;
        self.version = version;
        self.untitled_number = number.or(self.untitled_number);
        if self.path.is_none() {
            self.number_untitled();
        }
        self.changed(cx)
    }

    /// The encoding, BOM and line ending of the file (its `document` is empty: use [`doc`](Self::doc)).
    pub fn file(&self) -> &TextFile {
        &self.file
    }

    /// Gives an untitled, empty document the encoding, BOM and line ending of a new document
    /// ([`NewDocumentDefaults`]): what the status bar shows and what saving writes.
    pub fn set_new_document_defaults(&mut self, defaults: &NewDocumentDefaults) {
        self.number_untitled();
        self.set_format(defaults.encoding, defaults.bom);
        self.file.new_line = defaults.eol;
        self.model.set_new_line(defaults.eol);
    }

    /// The number of an untitled document, once it has one.
    pub fn untitled_number(&self) -> Option<u32> {
        self.untitled_number
    }

    /// `CAppNodeManager::GetNoNameNumber`: the first time a window needs it, it gets the next
    /// number of the sequence (`m_nNonameSequences`), and keeps it. The sequence only counts up:
    /// the number of a closed window is not used again. It is per process here (Sakura shares it
    /// by all the processes of the editor).
    pub fn number_untitled(&mut self) -> u32 {
        *self
            .untitled_number
            .get_or_insert_with(next_untitled_number)
    }

    /// Whether the encoding or the BOM was changed since the file was read or written, for a file
    /// that is on disk (`CDocFile::IsChgCodeSet`). Such a document is saved and asked about on
    /// close even when its text is unmodified.
    pub fn format_changed(&self) -> bool {
        self.disk.stamp.is_some() && (self.file.encoding, self.file.bom) != self.loaded_format
    }

    /// `CDocFile::CancelChgCodeSet`: gives up the change of the encoding and the BOM.
    pub(crate) fn cancel_format_change(&mut self) {
        (self.file.encoding, self.file.bom) = self.loaded_format;
    }

    /// Sets what a plain save writes (reopening or `F_FILE_REOPEN_*` of a file that is not there).
    pub fn set_format(&mut self, encoding: Encoding, bom: bool) {
        self.file.encoding = encoding;
        self.file.bom = bom;
    }

    /// Runs `f` on the file with the current text in it. The text is a cheap copy (the buffer
    /// shares its pieces), and the placeholder goes back afterwards.
    fn with_file<R>(&mut self, f: impl FnOnce(&mut TextFile) -> R) -> R {
        self.file.document = self.model.doc().clone();
        self.file.path = self.path.clone();
        let result = f(&mut self.file);
        self.file.document = Document::empty();
        result
    }

    /// Finds the characters that `options` cannot write ([`TextFile::check_save`]).
    pub fn check_save(
        &mut self,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> matsu_core::file::Result<UnmappableReport> {
        self.with_file(|file| file.check_save(options, progress))
    }

    /// Whether the line endings are mixed ([`TextFile::has_mixed_line_endings`]).
    pub fn has_mixed_line_endings(&mut self) -> matsu_core::file::Result<bool> {
        self.with_file(|file| file.has_mixed_line_endings())
    }

    /// Writes the document to `path` ([`TextFile::save_as`]). On success the document takes the
    /// path, the encoding, the BOM and the line ending of `options`. The caller marks the
    /// undo history as saved, because the cursor is the view's.
    pub fn save_as(
        &mut self,
        path: PathBuf,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> matsu_core::file::Result<SaveReport> {
        let before = self.path.clone();
        let report = self.with_file(|file| file.save_as(path, options, progress))?;
        self.path = self.file.path.clone();
        self.loaded_format = (self.file.encoding, self.file.bom);
        self.disk_after_save(before.as_deref());
        if let Some(eol) = options.eol {
            self.model.set_new_line(eol);
        }
        Ok(report)
    }

    /// `CDocVisitor::SetAllEol` (`sakura_core/doc/CDocVisitor.cpp`), which `CSaveAgent::OnBeforeSave`
    /// runs for a save that names a line ending: every line ending that is not `eol` is replaced in
    /// the document, as one undo step, and the line ending that Enter inserts becomes `eol`. Undo
    /// after such a save therefore undoes the conversion first, not the typing before it. Nothing
    /// is recorded when no line differs. Returns whether the text changed (the caller then calls
    /// [`changed`](Self::changed)).
    ///
    /// The file is written with the endings converted already (`TextFile::save_as`), so this only
    /// brings the document to what was written; the file keeps no memory of the conversion
    /// afterwards (`eol_on_save`), because the document says it.
    ///
    /// The cursor is moved along with the text when an ending that is no line break joins two lines
    /// (the caret and the selection follow their text, and the undo step stores the moved cursor).
    ///
    /// `None` when an edit failed: the document is then partly converted and differs from the
    /// file, so the caller must not mark it as saved.
    pub(crate) fn unify_line_endings(&mut self, eol: Eol, cursor: &mut Cursor) -> Option<bool> {
        self.file.eol_on_save = None;
        unify_line_endings(&mut self.model, eol, cursor)
    }

    /// `CAutoReloadAgent::OnAfterSave` and `CDocLocker::OnAfterSave`: the file is what this editor
    /// wrote, so its time stamp is not a change by another program, and it can be written. Saving
    /// under another name starts the watching of that file in the default mode.
    fn disk_after_save(&mut self, before: Option<&Path>) {
        let mode = if before == self.path.as_deref() {
            self.disk.mode
        } else {
            WatchMode::Query
        };
        self.disk = DiskState {
            stamp: self.path.as_deref().and_then(state::modified_time),
            mode,
            ..DiskState::default()
        };
    }

    /// The file on disk as this editor last saw it.
    pub(crate) fn disk(&self) -> &DiskState {
        &self.disk
    }

    pub(crate) fn disk_mut(&mut self) -> &mut DiskState {
        &mut self.disk
    }

    /// For a document that replaces this one (reopen): the watching goes on in the same mode
    /// (`CAutoReloadAgent` belongs to the window, not to the file it holds).
    pub(crate) fn keep_watch_mode(&mut self, from: &DiskState) {
        self.disk.mode = from.mode;
        self.disk.delay_count = from.delay_count;
    }

    /// `DoFileLock`: locks the file in the mode of the setting `mode` (`nFileShareMode`) against
    /// other programs. Nothing is locked for a document without a file or in view mode
    /// (`CDocFileOperation::_ToDoLock`). A failure is kept for
    /// [`take_lock_failure`](Self::take_lock_failure).
    pub(crate) fn lock_file(&mut self, mode: i32) {
        let path = self.path.clone().filter(|_| !self.view_mode);
        self.lock.lock(path.as_deref(), mode);
    }

    /// `DoFileUnlock`.
    pub(crate) fn unlock_file(&mut self) {
        self.lock.release();
    }

    /// `CFile::IsFileLocking`.
    pub(crate) fn is_file_locked(&self) -> bool {
        self.lock.is_locked()
    }

    /// The mode of the lock that is held, for the file property dialog.
    pub(crate) fn held_lock_mode(&self) -> Option<matsu_platform::file_lock::ShareMode> {
        self.lock.held_mode()
    }

    /// The setting the document last locked with (`CDocFile::GetShareMode`).
    pub(crate) fn lock_mode(&self) -> i32 {
        self.lock.mode()
    }

    /// Before something else writes `target` (`F_PUTFILE`): lets go of the lock if `target` is the
    /// file of this document, whose own lock would refuse the write (Windows) or be lost with the
    /// replaced file. Returns whether it did, for [`restore_lock`](Self::restore_lock).
    pub(crate) fn release_lock_for(&mut self, target: &Path) -> bool {
        let same = self.lock.is_locked()
            && self
                .path
                .as_deref()
                .is_some_and(|path| matsu_platform::file_id::same_file(path, target));
        if same {
            self.lock.release();
        }
        same
    }

    /// Takes the lock back after [`release_lock_for`](Self::release_lock_for) said it let go.
    pub(crate) fn restore_lock(&mut self, released: bool) {
        if released {
            self.lock_file(self.lock.mode());
        }
    }

    /// `CDocLocker::OnAfterLoad`: a file that cannot be written is not locked.
    pub(crate) fn lock_after_load(&mut self, mode: i32) {
        if self.disk.unwritable {
            self.lock.release();
        } else {
            self.lock_file(mode);
        }
    }

    /// The change of a setting or of view mode (`CEditDoc::OnChangeSetting`, `Command_VIEWMODE`):
    /// release, check again whether the file can be written, lock.
    pub(crate) fn relock_checking_writable(&mut self, mode: i32) {
        self.lock.release();
        self.disk.unwritable = self.path.as_deref().is_some_and(state::is_unwritable);
        self.lock_after_load(mode);
    }

    /// A lock that failed and was not shown yet.
    pub(crate) fn take_lock_failure(&mut self) -> Option<LockFailure> {
        self.lock.take_failure()
    }

    /// Drops the failure without showing it (`DoFileLock(false)`).
    pub(crate) fn forget_lock_failure(&mut self) {
        self.lock.forget_failure();
    }

    /// Whether editing is refused because the file cannot be written
    /// (`CEditDoc::IsEditable`, with 上書き禁止検出時は編集禁止にする on).
    pub(crate) fn locked_by_unwritable(&self, uneditable_if_unwritable: bool) -> bool {
        self.disk.unwritable && uneditable_if_unwritable
    }

    /// Writes the document to `path` for `F_PUTFILE`: like [`save_as`](Self::save_as), but the
    /// document keeps its own path, encoding, BOM and line ending (Sakura: "現在開いているファイル
    /// には影響しない").
    pub fn put_file(
        &mut self,
        path: PathBuf,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> matsu_core::file::Result<SaveReport> {
        let (encoding, bom, new_line, eol_on_save) = (
            self.file.encoding,
            self.file.bom,
            self.file.new_line,
            self.file.eol_on_save,
        );
        let result = self.with_file(|file| file.save_as(path, options, progress));
        self.file.path = self.path.clone();
        self.file.encoding = encoding;
        self.file.bom = bom;
        self.file.new_line = new_line;
        self.file.eol_on_save = eol_on_save;
        result
    }

    /// The line ending that Enter inserts, as the status bar names it.
    pub fn new_line(&self) -> Eol {
        self.model.new_line()
    }

    pub fn empty() -> Self {
        Self::new(DocumentModel::empty(), None)
    }

    /// (Reading the document syncs `bEnableExtEol` too: a copy of it taken to plan an edit, as the
    /// replace all does, must split text as the edit will.)
    pub fn model(&self) -> &DocumentModel {
        sync_ext_eol(&self.model);
        &self.model
    }

    pub fn doc(&self) -> &Document {
        sync_ext_eol(&self.model);
        self.model.doc()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file name for the title of the window.
    pub fn file_name(&self) -> Option<String> {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
    }

    /// Whether typing overwrites (overwrite mode) instead of inserting.
    pub fn overwrite(&self) -> bool {
        self.overwrite
    }

    /// Sets the typing mode. The views repaint (the caret and the status bar show it).
    pub fn set_overwrite(&mut self, overwrite: bool, cx: &mut Context<Self>) {
        if self.overwrite != overwrite {
            self.overwrite = overwrite;
            cx.notify();
        }
    }

    /// For a document that replaces this one (reopen): starts in the same mode.
    pub(crate) fn keep_view_mode(&mut self, view_mode: bool) {
        self.view_mode = view_mode;
    }

    /// Whether view mode is on.
    pub fn view_mode(&self) -> bool {
        self.view_mode
    }

    /// Turns view mode on or off. The views repaint (the title shows it).
    pub fn set_view_mode(&mut self, view_mode: bool, cx: &mut Context<Self>) {
        if self.view_mode != view_mode {
            self.view_mode = view_mode;
            cx.notify();
        }
    }

    /// Whether the user chose the document type (`F_CHANGETYPE`).
    pub fn type_locked(&self) -> bool {
        self.type_locked
    }

    pub fn set_type_locked(&mut self, locked: bool) {
        self.type_locked = locked;
    }

    /// Tells the views that the document has type `index`; each applies the settings of the type.
    pub fn announce_type(&mut self, index: usize, cx: &mut Context<Self>) {
        cx.emit(DocumentEvent::TypeChanged { index });
    }

    /// Counts the changes of the text. A view compares it with the version its layout matches.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The model and the path, for a [`CommandContext`](matsu_commands::context::CommandContext).
    /// Whoever edits through it calls [`changed`](Self::changed) afterwards.
    pub fn parts_mut(&mut self) -> (&mut DocumentModel, Option<&Path>) {
        sync_ext_eol(&self.model);
        (&mut self.model, self.path.as_deref())
    }

    /// Tells the views that the text changed, and returns the new version.
    pub fn changed(&mut self, cx: &mut Context<Self>) -> u64 {
        self.version += 1;
        cx.emit(DocumentEvent::Changed {
            version: self.version,
        });
        cx.notify();
        self.version
    }
}

/// Moves the caret and the anchor after the ending of line `at`, which has `len` bytes of text,
/// was replaced with `ending_len` bytes of text that is no line break: line `at + 1` is part of
/// line `at` now, after the new text. A box selection becomes a normal one (its columns are not
/// the columns of the joined line).
fn follow_join(cursor: &mut Cursor, at: usize, len: usize, ending_len: usize) {
    let map = |p: Pos| {
        if p.line <= at {
            p
        } else if p.line == at + 1 {
            Pos::new(at, len + ending_len + p.col)
        } else {
            Pos::new(p.line - 1, p.col)
        }
    };
    let caret = map(cursor.caret());
    match cursor.anchor() {
        Some(anchor) => cursor.select(map(anchor), caret),
        None => cursor.move_to(caret, false),
    }
}

// The bytes of line text that the loop below has read (tests only), to see that it does not
// read a growing joined line again and again.
#[cfg(test)]
thread_local! {
    static LINE_BYTES_READ: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The loop of [`SharedDocument::unify_line_endings`], on the model.
fn unify_line_endings(model: &mut DocumentModel, eol: Eol, cursor: &mut Cursor) -> Option<bool> {
    model.set_new_line(eol);
    let wanted = matsu_core::model::eol_text(eol);
    let mut changed = false;
    let mut failed = false;
    // Sakura changes the line objects one by one, but here a changed ending is joined with
    // its neighbours by the text: an LF turned into CR in front of a line that starts with an
    // LF would become a CRLF. Going from the end keeps the endings that are not changed yet
    // apart from it. An ending that is not a line break for the document (NEL, LS and PS while
    // they are not recognized) joins the next line to this one; from the end that line has been
    // converted already (Sakura's loop goes on to the next number and skips the joined line, and
    // from the front the growing line would be read again for every ending). For LF and CRLF
    // the front is safe, and keeps the order of Sakura.
    let is_break = TextBreaks::of(wanted, model.doc().ext_eol()).count == 1;
    let backwards = eol == Eol::Cr || !is_break;
    let mut line = if backwards {
        model.doc().line_count().checked_sub(1)
    } else {
        Some(0)
    };
    while let Some(at) = line {
        if at >= model.doc().line_count() {
            break;
        }
        let step = |at: usize| {
            if backwards {
                at.checked_sub(1)
            } else {
                Some(at + 1)
            }
        };
        let Ok(Some(current)) = model.doc().line(at) else {
            line = step(at);
            continue;
        };
        #[cfg(test)]
        LINE_BYTES_READ.with(|n| n.set(n.get() + current.text.len()));
        if current.eol.is_none_or(|e| e == eol) {
            line = step(at);
            continue;
        }
        if !changed {
            model.begin_block(cursor);
            changed = true;
        }
        let rows_before = model.doc().row_count();
        if model.replace_line_ending(at, wanted).is_err() {
            failed = true;
            break;
        }
        // The rows, not the lines: the final ending joins the empty row after it, and the
        // lines stay the same then (`a\n` and `a<NEL>` are one line each).
        let joined = model.doc().row_count() < rows_before;
        if joined {
            follow_join(cursor, at, current.text.len(), wanted.len());
        }
        // From the end the joined line has been converted already; from the front a join leaves
        // the next ending on the same line.
        if backwards || !joined {
            line = step(at);
        }
    }
    if changed {
        // never leave a position of the old text in the version that Redo brings back
        let _ = cursor.clamp_if_outside(model.doc());
        model.end_block(cursor);
    }
    (!failed).then_some(changed)
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[test]
    fn a_window_gets_its_number_once_and_the_sequence_goes_up() {
        let mut a = SharedDocument::empty();
        let mut b = SharedDocument::empty();
        assert_eq!(a.untitled_number(), None);
        let first = a.number_untitled();
        let second = b.number_untitled();
        assert!(second > first, "{first} then {second}");
        // Asking again gives the same number (`GetNoNameNumber` when `m_nId` is set).
        assert_eq!(a.number_untitled(), first);
        assert_eq!(a.untitled_number(), Some(first));
    }

    #[gpui::test]
    fn the_number_stays_with_the_window_when_the_document_is_replaced(cx: &mut TestAppContext) {
        let document = cx.new(|_| SharedDocument::empty());
        let number = document.update(cx, |shared, _| shared.number_untitled());
        // A file opened in the window: the number stays, for the next time it is untitled.
        let named = SharedDocument::new(DocumentModel::empty(), Some(PathBuf::from("/tmp/a.txt")));
        document.update(cx, |shared, cx| shared.replace_with(named, cx));
        assert_eq!(
            document.read_with(cx, |shared, _| shared.untitled_number()),
            Some(number)
        );
        document.update(cx, |shared, cx| {
            shared.replace_with(SharedDocument::empty(), cx)
        });
        assert_eq!(
            document.read_with(cx, |shared, _| shared.untitled_number()),
            Some(number)
        );
    }

    #[gpui::test]
    fn a_window_that_showed_a_file_gets_a_number_when_it_becomes_untitled(cx: &mut TestAppContext) {
        let named = SharedDocument::new(DocumentModel::empty(), Some(PathBuf::from("/tmp/a.txt")));
        let document = cx.new(|_| named);
        assert_eq!(
            document.read_with(cx, |shared, _| shared.untitled_number()),
            None
        );
        document.update(cx, |shared, cx| {
            shared.replace_with(SharedDocument::empty(), cx)
        });
        assert!(
            document
                .read_with(cx, |shared, _| shared.untitled_number())
                .is_some()
        );
    }

    #[test]
    fn unifying_to_an_ending_that_is_no_break_converts_every_line() {
        // NEL is no line break while the extended endings are off (the default): each change
        // joins the next line, and the ending of the joined line must be changed too, so that the
        // document is what the file got.
        let mut model = DocumentModel::new(Document::from_utf8(b"a\r\nb\nc".to_vec()).unwrap());
        let changed = unify_line_endings(&mut model, Eol::Nel, &mut Cursor::default());
        assert_eq!(changed, Some(true));
        assert_eq!(model.doc().line_count(), 1);
        let line = model.doc().line(0).unwrap().unwrap();
        assert_eq!((line.text.as_str(), line.eol), ("a\u{85}b\u{85}c", None));
    }

    #[test]
    fn unifying_keeps_the_bookmarks_of_the_lines() {
        let mut model = DocumentModel::new(Document::from_utf8(b"a\r\nb\r\nc".to_vec()).unwrap());
        model.bookmarks_mut().set(1, true);
        assert_eq!(
            unify_line_endings(&mut model, Eol::Lf, &mut Cursor::default()),
            Some(true)
        );
        assert_eq!(model.bookmarks().iter().collect::<Vec<_>>(), [1]);
        // and to CR, from the end
        let mut model = DocumentModel::new(Document::from_utf8(b"a\n\nb\n\n".to_vec()).unwrap());
        model.bookmarks_mut().set(2, true);
        assert_eq!(
            unify_line_endings(&mut model, Eol::Cr, &mut Cursor::default()),
            Some(true)
        );
        assert_eq!(model.doc().line_count(), 4);
        assert_eq!(model.bookmarks().iter().collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn unifying_to_an_ending_that_is_no_break_reads_each_line_once() {
        // Counted in bytes read, not in time: a loop that read the joined line again after every
        // join would read about lines^2 / 2 bytes.
        let lines = 3000;
        let text = "abc\r\n".repeat(lines);
        let mut model = DocumentModel::new(Document::from_utf8(text.into_bytes()).unwrap());
        LINE_BYTES_READ.with(|n| n.set(0));
        assert_eq!(
            unify_line_endings(&mut model, Eol::Ls, &mut Cursor::default()),
            Some(true)
        );
        let read = LINE_BYTES_READ.with(|n| n.get());
        assert!(read <= 4 * lines * 3 + 100, "{read} bytes read");
        assert_eq!(model.doc().line_count(), 1);
        let line = model.doc().line(0).unwrap().unwrap();
        assert_eq!(line.text, "abc\u{2028}".repeat(lines));
    }

    #[test]
    fn the_caret_follows_its_text_when_the_conversion_joins_lines() {
        // `a\nb\nc` to NEL (no line break while the extended endings are off): the caret before
        // `b` stays before `b`, and Undo and Redo bring back the cursors of their versions.
        let mut model = DocumentModel::new(Document::from_utf8(b"a\nb\nc".to_vec()).unwrap());
        let mut cursor = Cursor::new(Pos::new(1, 0));
        let before = cursor.clone();
        assert_eq!(
            unify_line_endings(&mut model, Eol::Nel, &mut cursor),
            Some(true)
        );
        assert_eq!(model.doc().line_count(), 1);
        // `a` and the NEL (2 bytes) come first
        assert_eq!((cursor.caret(), cursor.anchor()), (Pos::new(0, 3), None));
        let after = cursor.clone();
        assert!(model.undo(&mut cursor));
        assert_eq!(cursor, before);
        assert_eq!(model.doc().line_count(), 3);
        assert!(model.redo(&mut cursor));
        assert_eq!(cursor, after);
    }

    #[test]
    fn a_selection_over_joined_endings_follows_its_text() {
        let mut model = DocumentModel::new(Document::from_utf8(b"a\nb\nc".to_vec()).unwrap());
        let mut cursor = Cursor::new(Pos::new(0, 0));
        cursor.select(Pos::new(0, 0), Pos::new(2, 1));
        assert_eq!(
            unify_line_endings(&mut model, Eol::Nel, &mut cursor),
            Some(true)
        );
        // a NEL b NEL c: 1 + 2 + 1 + 2 + 1 bytes
        assert_eq!(cursor.selection(), Some((Pos::new(0, 0), Pos::new(0, 7))));
        assert_eq!(cursor.caret(), Pos::new(0, 7));
        // a selection that ends inside the second line
        let mut model = DocumentModel::new(Document::from_utf8(b"a\nbcd\ne".to_vec()).unwrap());
        let mut cursor = Cursor::new(Pos::new(0, 0));
        cursor.select(Pos::new(1, 1), Pos::new(2, 0));
        assert_eq!(
            unify_line_endings(&mut model, Eol::Ls, &mut cursor),
            Some(true)
        );
        // a LS(3 bytes) bcd LS e: the selection is `cd` and the ending
        assert_eq!(cursor.selection(), Some((Pos::new(0, 5), Pos::new(0, 10))));
    }

    /// Every position of the document, the empty row after a final ending included.
    fn all_positions(doc: &Document) -> Vec<Pos> {
        let mut out = Vec::new();
        for row in 0..doc.row_count() {
            let text = doc.line(row).unwrap().map(|l| l.text).unwrap_or_default();
            out.extend(text.char_indices().map(|(i, _)| Pos::new(row, i)));
            out.push(Pos::new(row, text.len()));
        }
        out
    }

    /// The number of chars before `pos` that are not part of a line ending.
    fn text_chars_before(doc: &Document, pos: Pos) -> usize {
        doc.text_between(Pos::new(0, 0), pos)
            .unwrap()
            .chars()
            .filter(|c| !matches!(c, '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}'))
            .count()
    }

    #[test]
    fn every_cursor_follows_its_text_through_every_conversion() {
        // The caret and the anchor keep the text they were at, the version after the conversion
        // holds positions of the new document only, and Undo, Redo and Undo again bring back
        // exactly the cursors of their versions.
        let docs: [&[u8]; 9] = [
            b"",
            b"\n",
            b"\n\n",
            b"a",
            b"a\n",
            b"a\nb\nc",
            b"a\r\nb\nc\r\n",
            b"\r\n\r\nx\r",
            b"\xc3\xa9\n\xe6\x97\xa5\r\n\n",
        ];
        for bytes in docs {
            for eol in [Eol::CrLf, Eol::Lf, Eol::Cr, Eol::Nel, Eol::Ls, Eol::Ps] {
                let fresh = || DocumentModel::new(Document::from_utf8(bytes.to_vec()).unwrap());
                let positions = all_positions(fresh().doc());
                for &caret in &positions {
                    for anchor in [None, positions.first().copied(), positions.last().copied()] {
                        let mut model = fresh();
                        let mut cursor = Cursor::new(caret);
                        if let Some(anchor) = anchor {
                            cursor.select(anchor, caret);
                        }
                        let (chars_caret, chars_anchor) = (
                            text_chars_before(model.doc(), caret),
                            cursor.anchor().map(|a| text_chars_before(model.doc(), a)),
                        );
                        let before = cursor.clone();
                        let what =
                            format!("{bytes:?} to {eol:?}, caret {caret:?}, anchor {anchor:?}");
                        let changed = unify_line_endings(&mut model, eol, &mut cursor)
                            .unwrap_or_else(|| panic!("failed: {what}"));
                        let mut after = cursor.clone();
                        after.clamp_if_outside(model.doc()).unwrap();
                        assert_eq!(after, cursor, "outside the document: {what}");
                        assert_eq!(
                            text_chars_before(model.doc(), cursor.caret()),
                            chars_caret,
                            "caret: {what}"
                        );
                        if let Some(chars) = chars_anchor {
                            if let Some(a) = cursor.anchor() {
                                assert_eq!(
                                    text_chars_before(model.doc(), a),
                                    chars,
                                    "anchor: {what}"
                                );
                            } else {
                                assert_eq!(chars, chars_caret, "anchor gone: {what}");
                            }
                        }
                        if changed {
                            for _ in 0..2 {
                                assert!(model.undo(&mut cursor), "{what}");
                                assert_eq!(cursor, before, "undo: {what}");
                                assert!(model.redo(&mut cursor), "{what}");
                                assert_eq!(cursor, after, "redo: {what}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_final_ending_joins_the_empty_row_after_it() {
        // `a\n` and `a<NEL>` are one line each, but the caret on the empty row has to move
        let mut model = DocumentModel::new(Document::from_utf8(b"a\n".to_vec()).unwrap());
        let mut cursor = Cursor::new(Pos::new(1, 0));
        assert_eq!(
            unify_line_endings(&mut model, Eol::Nel, &mut cursor),
            Some(true)
        );
        assert_eq!(cursor.caret(), Pos::new(0, 3));
    }
}
