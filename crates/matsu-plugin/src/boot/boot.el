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

(defun editor--encode (object)
  "OBJECT as one line of ASCII JSON (every character above ASCII is written as \\uXXXX)."
  (let ((json (json-serialize object)))
    ;; json-serialize answers UTF-8 bytes (a unibyte string): back to characters, and then every
    ;; one above ASCII becomes \uXXXX.
    (editor--ascii-json
     (if (multibyte-string-p json) json (decode-coding-string json 'utf-8)))))

(defun editor-call (method &rest args)
  "Call the editor's macro function METHOD (a string such as \"InsText\") with ARGS.
Returns the result.  An error answer of the editor is signaled as `editor-error', whose data
are the code and the message."
  (let* ((id editor--next-id)
         (params (vconcat (mapcar #'editor--argument args)))
         (request (editor--encode `((jsonrpc . "2.0") (id . ,id) (method . ,method)
                                    (params . ,params)))))
    (setq editor--next-id (1+ id))
    ;; To `t' (the standard output) whatever `standard-output' is bound to: a script may capture
    ;; its own output with `with-output-to-string'.
    (princ (concat editor--rs request "\n") t)
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

;; Evaluates the forms of SOURCE one at a time.  The forms are read from one buffer and evaluated
;; in another, so that what the script does to "the current buffer" (`goto-char', `set-buffer',
;; `erase-buffer') cannot disturb the reading of the rest of it.  `(eval form t)' has an empty
;; lexical environment: the script sees none of the variables of this function.
(defun editor--eval-forms (source)
  "Evaluate the forms of SOURCE.  Nil if all went well, else (LINE . ERROR-DATA): the line
where the form that failed began, and what it signaled."
  (with-temp-buffer
    (insert source)
    (goto-char (point-min))
    (let ((reader (current-buffer))
          (work (generate-new-buffer " *macro*"))
          (line 1))
      (unwind-protect
          (condition-case err
              (progn
                (while (editor--skip-to-form)
                  (setq line (line-number-at-pos))
                  (let ((form (read reader)))
                    (with-current-buffer work
                      (eval form t))))
                nil)
            ((error quit) (cons line err)))
        (when (buffer-live-p work)
          (kill-buffer work))))))

(defun editor--run (name source)
  "Evaluate the forms of SOURCE.  NAME is shown in the errors."
  (let ((failure (editor--eval-forms source)))
    (when failure
      (editor--fail name (car failure) (cdr failure)))
    t))

;; ---- Plugin host -----------------------------------------------------------------------------
;;
;; A plugin host (ADR-0026): `initialize' says kind "plugin" and brings no script.  The editor
;; sends `plugin/invoke' (the jack, the name and the text of the handler, the arguments) and
;; waits for the answer; the process stays for the next call when the plugin is resident.  The
;; handler is evaluated form by form like a macro (`editor--eval-forms'), in a work buffer of its
;; own.  It sees the variables `plugin-id', `plugin-dir', `plugin-jack' and `plugin-args' and the
;; functions `plugin-*' below.  The host's own state is in `plugin--' variables, and the locals of
;; its loop are lexical, which the handler cannot see.

(defvar plugin-id "" "The ID of the plugin whose handler runs.")
(defvar plugin-dir "" "The folder of the plugin.")
(defvar plugin-jack "" "The name of the jack that is called (\"Command\", \"Outline\", ...).")
(defvar plugin-args nil "The arguments of the call: an alist (a JSON object) or a vector.")
(defvar plugin--typed "" "The char that was typed, in the jack SmartIndent.")

(defun plugin--trim (args)
  "ARGS without the trailing nils (a trailing nil is a missing argument, not a null)."
  (let ((rest (reverse args)))
    (while (and rest (null (car rest)))
      (pop rest))
    (nreverse rest)))

;; The calls of the objects: (plugin-get-option "Section" "Key") is the call "Plugin.GetOption".
(dolist (spec '((plugin-get-plugin-dir . "Plugin.GetPluginDir")
                (plugin-get-def . "Plugin.GetDef")
                (plugin-get-option . "Plugin.GetOption")
                (plugin-set-option . "Plugin.SetOption")
                (plugin-get-string . "Plugin.GetString")
                (plugin-get-command-no . "Plugin.GetCommandNo")
                (plugin-add-command . "Plugin.AddCommand")
                (plugin-outline-add-func-info . "Outline.AddFuncInfo")
                (plugin-outline-add-func-info2 . "Outline.AddFuncInfo2")
                (plugin-outline-add-func-info3 . "Outline.AddFuncInfo3")
                (plugin-outline-add-func-info4 . "Outline.AddFuncInfo4")
                (plugin-outline-set-title . "Outline.SetTitle")
                (plugin-outline-set-list-type . "Outline.SetListType")
                (plugin-outline-set-label . "Outline.SetLabel")))
  (let ((method (cdr spec)))
    (defalias (car spec)
      (lambda (&rest args)
        (apply #'editor-call method (plugin--trim args))))))

(defun plugin--arg (key)
  "The value of KEY in the arguments of the call, or nil."
  (and (consp plugin-args) (alist-get key plugin-args)))

(defun plugin-complement-get-current-word ()
  "The word to complete (the jack Complement)."
  (let ((word (plugin--arg 'word)))
    (if (stringp word) word "")))

(defun plugin-complement-get-option ()
  "The option of the call (the jack Complement), an integer."
  (let ((option (plugin--arg 'option)))
    (if (integerp option) option 0)))

(defun plugin-complement-add-list (&optional word)
  "Add WORD to the candidates (the jack Complement).  The editor says whether it is new."
  (apply #'editor-call "Complement.AddList" (and word (list word))))

(defun plugin-indent-get-char ()
  "The char that was typed (the jack SmartIndent); \"\" in the other jacks."
  plugin--typed)

(defun plugin--describe (name line err)
  "The text of a failure for the editor: the kind, the message, and the place in the handler."
  (let ((text (condition-case nil
                  (if (eq (car-safe err) 'editor-error)
                      (format "EditorError %s: %s" (nth 1 err) (nth 2 err))
                    (format "%s: %s" (car-safe err) (error-message-string err)))
                (error (format "%S" err)))))
    (format "%s\n  at %s:%d" text name line)))

(defun plugin--respond (id failure)
  "Answer the invoke ID: ok, or the text FAILURE."
  ;; The marker: a line on standard error that says everything the handler wrote there before its
  ;; answer has been written (the editor reads the two streams on two threads).  `message' in batch
  ;; mode writes the line to standard error and flushes it.
  (message "%c%s" 31 (format "matsu-stderr-done %s" (or id "null")))
  (let* ((id (or id :null))
         (answer (if failure
                     `((jsonrpc . "2.0") (id . ,id)
                       (error . ((code . -32000) (message . ,failure))))
                   `((jsonrpc . "2.0") (id . ,id) (result . ((ok . t)))))))
    (princ (concat editor--rs
                   (condition-case nil
                       (editor--encode answer)
                     ;; A text that cannot be written as JSON (raw bytes): the answer is still sent.
                     (error
                      (editor--encode
                       `((jsonrpc . "2.0") (id . ,id)
                         (error . ((code . -32000)
                                   (message . "the handler failed (its text cannot be shown)")))))))
                   "\n")
           t)))

(defvar native-comp-enable-subr-trampolines)

(defun plugin--run-handler (name source)
  "Run the handler SOURCE named NAME.  Nil, or the text of its failure.
Its own endings end only the handler: `kill-emacs' and (throw 'plugin-exit nil) leave it, and
a `quit' or any error is its failure."
  (let ((failure nil)
        (original (symbol-function 'kill-emacs)))
    (unwind-protect
        (progn
          ;; No trampoline for the primitive that is redefined: the handler is interpreted, which
          ;; calls through the symbol, and a trampoline would start the native compiler.
          (let ((native-comp-enable-subr-trampolines nil))
            (fset 'kill-emacs (lambda (&rest _) (throw 'plugin-exit nil))))
          (catch 'plugin-exit
            (let ((found (editor--eval-forms source)))
              (when found
                (setq failure (plugin--describe name (car found) (cdr found)))))))
      (let ((native-comp-enable-subr-trampolines nil))
        (fset 'kill-emacs original)))
    failure))

(defun plugin--invoke (message)
  "Serve the `plugin/invoke' MESSAGE: run the handler, and answer."
  (let* ((params (let ((p (alist-get 'params message))) (and (consp p) p)))
         (text (lambda (key default)
                 (let ((value (alist-get key params)))
                   (if (stringp value) value default))))
         (name (funcall text 'name "<plugin>"))
         (source (funcall text 'source "")))
    (setq plugin-jack (funcall text 'jack "")
          plugin-args (alist-get 'args params)
          plugin--typed (let ((char (and (equal plugin-jack "SmartIndent") (plugin--arg 'char))))
                          (if (stringp char) char "")))
    (plugin--respond (alist-get 'id message) (plugin--run-handler name source))))

(defun plugin--serve (info)
  "Serve the calls of the editor until it says shutdown or closes the input."
  (let ((identity (alist-get 'plugin info)))
    (when (consp identity)
      (let ((id (alist-get 'id identity))
            (dir (alist-get 'dir identity)))
        (setq plugin-id (if (stringp id) id "")
              plugin-dir (if (stringp dir) dir ""))))
    ;; The folder of the plugin, so that a handler can `require' and `load' its own files.
    (unless (string-empty-p plugin-dir)
      (push plugin-dir load-path))
    (catch 'plugin--done
      (while t
        (let ((line (editor--read-line)))
          (unless line
            (throw 'plugin--done nil))
          (setq line (string-trim line))
          (unless (string-empty-p line)
            (let* ((message (condition-case nil
                                (json-parse-string line :object-type 'alist
                                                   :null-object nil :false-object nil)
                              (error nil)))
                   (method (and (consp message) (alist-get 'method message))))
              (cond
               ((equal method "plugin/shutdown") (throw 'plugin--done nil))
               ((equal method "plugin/invoke") (plugin--invoke message))))))))))

;; The first line is the initialize notification.
(let* ((first (editor--read-line))
       (notification (and first
                          (condition-case nil
                              (json-parse-string first :object-type 'alist
                                                 :null-object nil :false-object nil)
                            (error nil))))
       (params (and (equal (alist-get 'method notification) "initialize")
                    (alist-get 'params notification)))
       (script (alist-get 'script params))
       (plugin-host (equal (alist-get 'kind params) "plugin")))
  (unless (or script plugin-host)
    (message "the editor did not send the script in initialize")
    (kill-emacs 1))
  (setq editor-info params)
  (load (expand-file-name "api_gen.el" (file-name-directory load-file-name)) nil t t)
  (if plugin-host
      (plugin--serve params)
    (editor--run (alist-get 'name script) (alist-get 'source script)))
  (kill-emacs 0))
