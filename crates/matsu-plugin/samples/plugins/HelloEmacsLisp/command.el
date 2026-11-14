;; Command 1 of the sample plugin: inserts the greeting at the caret.
;; The functions of the API (editor-ins-text ...) and of the plugin (plugin-get-option ...) are there
;; without a `require'. The handler is read one form at a time. A variable that has to stay for the
;; next call (the plugin is resident) is made with `defvar'; a `let' variable stays inside the call.
(let ((greeting (plugin-get-option "Hello" "Greeting")))
  (when (or (null greeting) (string-empty-p greeting))
    (setq greeting "Hello"))
  (editor-ins-text (format "%s (%s)" greeting plugin-jack)))
