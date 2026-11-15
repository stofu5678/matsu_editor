;; The jack DocumentOpen: counts the opens in a variable, which the resident process keeps for the
;; next call (`defvar' does not reset a variable that has a value).
(defvar hello-sample-opens 0)
(setq hello-sample-opens (1+ hello-sample-opens))
