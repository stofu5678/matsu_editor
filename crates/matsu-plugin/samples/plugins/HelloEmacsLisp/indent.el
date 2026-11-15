;; The jack SmartIndent: after Enter, four more spaces when the line above ends with a colon.
(when (member (plugin-indent-get-char) '("\r" "\n"))
  (let ((line (string-to-number (editor-expand-parameter "$y"))))
    (when (>= line 2)
      (let ((above (string-trim-right (editor-get-line-str (1- line)) "[\r\n]+")))
        (when (string-suffix-p ":" above)
          (editor-ins-text "    "))))))
