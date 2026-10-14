# The boot library of the Python macros: python -u boot.py user.py
#
# It reads and writes the editor's JSON-RPC lines synchronously (docs/poc/json-rpc.md, spec
# sections 2 and 3), and runs the user's file with `Editor` and `EditorError` in its globals, so
# that the file needs no import. The file runs as __main__ by runpy, so the line numbers of
# errors are the file's own.
import json
import os
import runpy
import sys

_RS = "\x1e"
_out = sys.__stdout__
_in = sys.__stdin__


class EditorError(Exception):
    """The editor answered a call with an error (a JSON-RPC error object)."""

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message


_next_id = 1


def _call(method, params):
    """Sends a request and waits for its response."""
    global _next_id
    request_id = _next_id
    _next_id += 1
    # json.dumps writes only ASCII by default (non-ASCII characters become \uXXXX).
    line = _RS + json.dumps({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}) + "\n"
    # Anything the user printed goes first. The bytes are written to the binary layer, so that
    # Windows does not turn the line break into CR LF.
    _out.flush()
    _out.buffer.write(line.encode("ascii"))
    _out.buffer.flush()
    while True:
        text = _in.readline()
        if text == "":
            raise EOFError("The editor closed the connection before it answered " + method)
        text = text.strip()
        if not text:
            continue
        message = json.loads(text)
        if message.get("id") != request_id:
            continue  # a notification, or an answer to something that is no longer waited for
        if "error" in message:
            error = message["error"]
            raise EditorError(error.get("code"), error.get("message"))
        return message.get("result")


def _main():
    # The first line is the initialize notification.
    info = None
    first = _in.readline().strip()
    if first:
        message = json.loads(first)
        if message.get("method") == "initialize":
            info = message.get("params")

    # Not through sys.path[0]: with PYTHONSAFEPATH or -P, Python does not put this directory
    # first.
    boot_dir = os.path.dirname(os.path.abspath(__file__))
    sys.path.insert(0, boot_dir)
    try:
        import api_gen
    finally:
        sys.path.remove(boot_dir)

    editor = api_gen.install(_call)
    editor.info = info

    user_file = os.path.abspath(sys.argv[1])
    # Normally Python puts the script's directory (the boot directory) first in the module
    # search path; the script is the user's, so make it the user's directory. In safe-path mode
    # nothing was put there, and the user's directory is not added either (as for a script run
    # with the same flag).
    if sys.path and os.path.abspath(sys.path[0] or ".") == boot_dir:
        sys.path[0] = os.path.dirname(user_file)
    sys.argv = sys.argv[1:]
    runpy.run_path(
        user_file,
        init_globals={"Editor": editor, "EditorError": EditorError},
        run_name="__main__",
    )


_main()
