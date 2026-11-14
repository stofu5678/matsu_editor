-- The jack DocumentOpen: counts the opens in a property, which the resident process keeps for the
-- next call (the compiled program of the handler is run again, so its properties stay).
property openCount : 0
set openCount to openCount + 1
editorPrint("opened " & openCount)
