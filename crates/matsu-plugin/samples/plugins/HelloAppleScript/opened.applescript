-- The jack DocumentOpen: it only has to end quietly. A value cannot be kept in AppleScript between
-- calls (each call is a program of its own); use editorCall("Plugin.SetOption", ...) for one.
editorPrint("opened")
