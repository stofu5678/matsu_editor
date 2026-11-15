;; The jack Complement: two candidates that begin with the word before the caret.
(let ((word (plugin-complement-get-current-word)))
  (plugin-complement-add-list (concat word "_hello"))
  (plugin-complement-add-list (concat word "_world")))
