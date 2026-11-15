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
import traceback
import types

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

    if info and info.get("kind") == "plugin":
        _serve_plugin(editor, info)
        return

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


class _Plugin:
    """The `Plugin` object of a plugin (Sakura's CPluginIfObj): the editor answers the calls."""

    def __init__(self, identity):
        self.id = identity.get("id")
        self.dir = identity.get("dir")
        self.jack = ""
        self.args = None

    def GetPluginDir(self, *args):
        return _call("Plugin.GetPluginDir", _trim(args))

    def GetDef(self, *args):
        return _call("Plugin.GetDef", _trim(args))

    def GetOption(self, *args):
        return _call("Plugin.GetOption", _trim(args))

    def SetOption(self, *args):
        return _call("Plugin.SetOption", _trim(args))

    def GetString(self, *args):
        return _call("Plugin.GetString", _trim(args))

    def GetCommandNo(self, *args):
        return _call("Plugin.GetCommandNo", _trim(args))

    def AddCommand(self, *args):
        return _call("Plugin.AddCommand", _trim(args))


class _Outline:
    """The `Outline` object of the jack Outline (Sakura's COutlineIfObj)."""

    def AddFuncInfo(self, *args):
        return _call("Outline.AddFuncInfo", _trim(args))

    def AddFuncInfo2(self, *args):
        return _call("Outline.AddFuncInfo2", _trim(args))

    def AddFuncInfo3(self, *args):
        return _call("Outline.AddFuncInfo3", _trim(args))

    def AddFuncInfo4(self, *args):
        return _call("Outline.AddFuncInfo4", _trim(args))

    def SetTitle(self, *args):
        return _call("Outline.SetTitle", _trim(args))

    def SetListType(self, *args):
        return _call("Outline.SetListType", _trim(args))

    def SetLabel(self, *args):
        return _call("Outline.SetLabel", _trim(args))


class _Complement:
    """The `Complement` object of the completion jacks (Sakura's CComplementIfObj). The word and
    the option come with the call; the candidates go to the editor."""

    def __init__(self, plugin):
        self._plugin = plugin

    def GetCurrentWord(self):
        args = self._plugin.args
        word = args.get("word") if isinstance(args, dict) else None
        return word if isinstance(word, str) else ""

    def GetOption(self):
        args = self._plugin.args
        option = args.get("option") if isinstance(args, dict) else None
        return option if isinstance(option, int) and not isinstance(option, bool) else 0

    def AddList(self, *args):
        return _call("Complement.AddList", [str(a) for a in args[:1]])


class _Indent:
    """The `Indent` object of the jack SmartIndent: GetChar answers the typed char."""

    def __init__(self, typed):
        self._typed = typed

    def GetChar(self):
        return self._typed


def _trim(args):
    args = list(args)
    while args and args[-1] is None:
        args.pop()
    return args


def _respond(request_id, error):
    if error is None:
        message = {"jsonrpc": "2.0", "id": request_id, "result": {"ok": True}}
    else:
        message = {"jsonrpc": "2.0", "id": request_id, "error": {"code": -32000, "message": error}}
    _out.flush()
    _out.buffer.write((_RS + json.dumps(message) + "\n").encode("ascii"))
    _out.buffer.flush()


def _describe(exc):
    # The first frame is this boot (the call of exec): the traceback starts at the handler.
    return "".join(traceback.format_exception(type(exc), exc, exc.__traceback__.tb_next)).rstrip("\n")


def _serve_plugin(editor, info):
    """Answers `plugin/invoke` until the editor ends the conversation (ADR-0026). The text of the
    handler that the editor read runs in a namespace of its own for each handler file, which is kept
    for the next call of the same file (the plugin is resident); a file never sees another's."""
    identity = info.pop("plugin", None) or {}
    plugin = _Plugin(identity)
    outline = _Outline()
    complement = _Complement(plugin)
    # The folder of the plugin, so that a handler can import its own modules (as a macro can import
    # the ones next to it).
    if plugin.dir:
        sys.path.insert(0, plugin.dir)
    spaces = {}
    while True:
        text = _in.readline()
        if text == "":
            return
        text = text.strip()
        if not text:
            continue
        try:
            message = json.loads(text)
        except ValueError:
            continue
        method = message.get("method")
        if method == "plugin/shutdown":
            return
        if method != "plugin/invoke":
            continue
        params = message.get("params") or {}
        name = params.get("name") or "<plugin>"
        plugin.jack = params.get("jack") or ""
        plugin.args = params.get("args")
        module = spaces.get(name)
        if module is None:
            # A module object backs the namespace, so that what looks up `sys.modules[__name__]`
            # (pickle, dataclasses with postponed annotations) finds the handler's own names.
            module = types.ModuleType("__main__")
            module.__file__ = name
            module.__dict__.update(
                {
                    "Editor": editor,
                    "EditorError": EditorError,
                    "Plugin": plugin,
                    "Outline": outline,
                    "Complement": complement,
                }
            )
            spaces[name] = module
        space = module.__dict__
        if plugin.jack == "SmartIndent":
            args = plugin.args
            typed = args.get("char") if isinstance(args, dict) else None
            space["Indent"] = _Indent(typed if isinstance(typed, str) else "")
        else:
            space.pop("Indent", None)
        failure = None
        # While the handler runs, `__main__` is its module (the boot's own is put back after).
        boot_main = sys.modules.get("__main__")
        sys.modules["__main__"] = module
        try:
            exec(compile(params.get("source") or "", name, "exec"), space)
        except SystemExit:
            # sys.exit() in a handler ends the handler, not the host.
            pass
        except BaseException as exc:  # noqa: BLE001 - whatever the handler raised fails the call
            failure = _describe(exc)
        finally:
            sys.modules["__main__"] = boot_main
        sys.stdout.flush()
        sys.stderr.flush()
        _respond(message.get("id"), failure)


_main()
