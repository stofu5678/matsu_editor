;;; sample.el --- sample macro (Emacs Lisp: .el, Emacs 30 or later)  -*- lexical-binding: t; -*-

;; The functions of the API are there without a `require': the Sakura macro name in lisp case
;; with the prefix `editor-' (`InsText' is `editor-ins-text'), and `editor-call' for any name
;; ((editor-call "1PageUp")). The script is read one top-level form at a time, so an error says
;; the line where its form began. The editor's own functions (`insert') do not touch the
;; document; use these.

(editor-go-file-top)
(editor-ins-text "# サンプルマクロ 😀\n")

(let ((count (editor-get-line-count 0)))
  (princ (format "行数: %d ファイル: %s\n" count
                 (let ((name (editor-get-filename)))
                   (if (string-empty-p name) "(無題)" name))))
  (dotimes (i count)
    (princ (format "%d: %s\n" (1+ i)
                   (string-trim-right (editor-get-line-str (1+ i)) "\n")))))

;; Select line 3 (a line number and a column both start at 1) and replace it.
(editor-move-cursor 3 1 0)
(editor-move-cursor 3 1000 1)           ; past the end of the line: the end of the line
(princ (format "選択: %s / IsTextSelected = %s\n"
               (editor-get-selected-string 0) (editor-is-text-selected)))
(editor-ins-text "みかん😀 (置き換えました)")

;; An error response is an `editor-error': its data are the code and the message.
(condition-case err
    (editor-get-line-str "not a number")
  (editor-error (princ (format "エラー: %s\n" (cadr err)))))

(editor-add-tail "-- マクロの終わり --\n")
(editor-jump 1)
