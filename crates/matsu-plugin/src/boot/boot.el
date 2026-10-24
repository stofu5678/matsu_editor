;;; boot.el --- the boot library of the Emacs Lisp macros  -*- lexical-binding: t; coding: utf-8-unix -*-

;; The editor starts `emacs -Q --batch --load boot.el'.  The first line of the standard input is
;; the `initialize' notification; its params.script holds the name and the text of the user's
;; script.  The script is read one top-level form at a time and each form is evaluated, so an
;; error says on which line the form began (`--load' of the file would not say).  The functions
;; of the API (api_gen.el, `editor-ins-text' and so on) and `editor-call' are ready, so the
;; script needs no `require'.
;;
;; The rest follows docs/poc/json-rpc.md (sections 2 and 3): the lines are read and written
;; synchronously, a line to the editor starts with RS, and every line is ASCII.  Emacs 30 has
;; the JSON functions built in; Emacs 29 has them when it is built with JSON support.
;;
;; This file is ASCII.

(unless (and (fboundp 'json-parse-string) (fboundp 'json-serialize))
  (message "Emacs Lisp macros need Emacs 30 or later (this Emacs has no built-in JSON support)")
  (kill-emacs 1))

;; The user's output and the script are UTF-8, whatever the code page of the OS is.  The standard
;; output and the error output are encoded with `locale-coding-system' in batch mode.
(set-language-environment "UTF-8")
(setq locale-coding-system 'utf-8-unix)
(set-terminal-coding-system 'utf-8-unix)
(set-keyboard-coding-system 'utf-8-unix)
(setq coding-system-for-write 'utf-8-unix)
(setq debug-on-error nil)
(setq inhibit-message nil)

(require 'subr-x)

(define-error 'editor-error "Error of the editor")

(defvar editor-info nil
  "What the editor told the macro in `initialize', as an alist.")

(defvar editor--next-id 1)

(defconst editor--rs (string 30))

(defun editor--read-line ()
  "One line from the standard input, or nil at the end of the input."
  (condition-case nil
      (read-from-minibuffer "")
    (error nil)))

(defun editor--ascii-json (text)
  "TEXT with every character above ASCII written as \\uXXXX (a surrogate pair above U+FFFF)."
  (replace-regexp-in-string
   "[^[:ascii:]]"
   (lambda (match)
     (let ((c (aref match 0)))
       (if (< c #x10000)
           (format "\\u%04X" c)
         (let ((v (- c #x10000)))
           (format "\\u%04X\\u%04X"
                   (+ #xD800 (ash v -10))
                   (+ #xDC00 (logand v #x3FF)))))))
   text t t))

(defun editor--argument (value)
  "VALUE as something `json-serialize' writes the way the editor expects."
  (cond
   ((null value) :null)
   ((eq value t) t)
   ((memq value '(:null :false)) value)
   ((stringp value) value)
   ((integerp value) value)
   ((floatp value)
    (cond
     ((or (isnan value) (= value 1.0e+INF) (= value -1.0e+INF))
      (signal 'editor-error (list -32602 "cannot send NaN or infinity to the editor")))
     ;; A whole number goes as an integer (4.0 is 4), as in the other languages.
     ((and (= value (ffloor value)) (< (abs value) 9007199254740992.0))
      (truncate value))
     (t value)))
   (t (signal 'editor-error
              (list -32602 (format "cannot send %S to the editor" value))))))

(defun editor-call (method &rest args)
  "Call the editor's macro function METHOD (a string such as \"InsText\") with ARGS.
Returns the result.  An error answer of the editor is signaled as `editor-error', whose data
are the code and the message."
  (let* ((id editor--next-id)
         (params (vconcat (mapcar #'editor--argument args)))
         (json (json-serialize `((jsonrpc . "2.0") (id . ,id) (method . ,method)
                                 (params . ,params))))
         ;; json-serialize answers UTF-8 bytes (a unibyte string): back to characters, and then
         ;; every one above ASCII becomes \uXXXX.
         (request (editor--ascii-json
                   (if (multibyte-string-p json) json (decode-coding-string json 'utf-8)))))
    (setq editor--next-id (1+ id))
    (princ (concat editor--rs request "\n"))
    (let ((answer nil))
      (while (null answer)
        (let ((text (editor--read-line)))
          (unless text
            (signal 'editor-error
                    (list -32000 (format "The editor closed the connection before it answered %s"
                                         method))))
          (setq text (string-trim text))
          (unless (string-empty-p text)
            (let ((reply (condition-case nil
                             (json-parse-string text :object-type 'alist
                                                :null-object nil :false-object nil)
                           (error nil))))
              ;; A notification, or an answer to something that is no longer waited for.
              (when (and (consp reply) (eql (alist-get 'id reply) id))
                (setq answer reply))))))
      (let ((failure (alist-get 'error answer)))
        (if failure
            (signal 'editor-error
                    (list (alist-get 'code failure) (alist-get 'message failure)))
          (alist-get 'result answer))))))

(defun editor--fail (name line err)
  "Say on the error output that the script NAME failed at LINE with ERR, and end."
  (message "%s:%d: %s" name line
           (if (eq (car-safe err) 'editor-error)
               (format "EditorError %s: %s" (nth 1 err) (nth 2 err))
             (error-message-string err)))
  (kill-emacs 1))

(defun editor--skip-to-form ()
  "Move over white space and comment lines.  Non-nil if a form follows."
  (let ((more t))
    (while more
      (skip-chars-forward " \t\n\r\f")
      (if (eq (char-after) ?\;)
          (forward-line 1)
        (setq more nil)))
    (not (eobp))))

(defun editor--run (name source)
  "Evaluate the forms of SOURCE one at a time.  NAME is shown in the errors."
  ;; The forms are read from one buffer and evaluated in another, so that what the script does
  ;; to "the current buffer" (`goto-char', `set-buffer', `erase-buffer') cannot disturb the
  ;; reading of the rest of it.
  (with-temp-buffer
    (insert source)
    (goto-char (point-min))
    (let ((reader (current-buffer))
          (work (generate-new-buffer " *macro*"))
          (line 1))
      (condition-case err
          (progn
            (while (editor--skip-to-form)
              (setq line (line-number-at-pos))
              (let ((form (read reader)))
                (with-current-buffer work
                  (eval form t))))
            t)
        (error (editor--fail name line err))))))

;; The first line is the initialize notification.
(let* ((first (editor--read-line))
       (notification (and first
                          (condition-case nil
                              (json-parse-string first :object-type 'alist
                                                 :null-object nil :false-object nil)
                            (error nil))))
       (params (and (equal (alist-get 'method notification) "initialize")
                    (alist-get 'params notification)))
       (script (alist-get 'script params)))
  (unless script
    (message "the editor did not send the script in initialize")
    (kill-emacs 1))
  (setq editor-info params)
  (load (expand-file-name "api_gen.el" (file-name-directory load-file-name)) nil t t)
  (editor--run (alist-get 'name script) (alist-get 'source script))
  (kill-emacs 0))
