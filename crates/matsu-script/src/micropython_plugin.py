# The serving loop of a plugin in MicroPython (src/micropython.rs). It runs after the boot, in the
# same globals (MicroPython can be started once in a process, so the loop is in Python). It asks the
# host for each `plugin/invoke` through the reserved calls (src/plugin.rs) and runs the text of the
# handler in a namespace of its own for each handler file, kept for the next call of the same file
# (ADR-0026). The handler sees `Editor`, `EditorError`, `Plugin`, `Outline`, `Complement` and, for the
# jack SmartIndent, `Indent`, as in the boot of the external Python.
import sys as _sys
import io as _io


class _Calls:
    pass


def _host(name, args):
    return _json.loads(_matsu_call(name, args)[0])


def _calls(prefix, names):
    calls = _Calls()
    for name in names:
        setattr(calls, name, _bind(prefix + name))
    return calls


_respond = _bind("plugin/respond")
_identity = _host("plugin/identity", [])
Plugin = _calls("Plugin.", _identity["members"]["Plugin"])
Plugin.id = _identity.get("id")
Plugin.dir = _identity.get("dir")
Plugin.jack = ""
Plugin.args = None
Outline = _calls("Outline.", _identity["members"]["Outline"])


class _Complement:
    """The word to complete and the options come with the call; the candidates go to the editor."""

    def __init__(self):
        self._add = _bind("Complement.AddList")

    def GetCurrentWord(self):
        args = Plugin.args
        word = args.get("word") if isinstance(args, dict) else None
        return word if isinstance(word, str) else ""

    def GetOption(self):
        args = Plugin.args
        option = args.get("option") if isinstance(args, dict) else None
        return option if isinstance(option, int) and not isinstance(option, bool) else 0

    def AddList(self, *args):
        return self._add(*[str(a) for a in args[:1]])


class _Indent:
    """The `Indent` object of the jack SmartIndent: GetChar answers the typed char."""

    def __init__(self, typed):
        self._typed = typed

    def GetChar(self):
        return self._typed


Complement = _Complement()


def _main_of(space):
    """What `import __main__` gives while a handler runs: the names of the handler's own namespace
    (MicroPython cannot make a module out of a dictionary, and has no `__setattr__` hook, so a name
    that the handler sets on it stays on this object, which is its own for the file)."""

    class _Main:
        def __getattr__(self, name):
            try:
                return space[name]
            except KeyError:
                raise AttributeError(name)

    return _Main()


def _describe(exc):
    out = _io.StringIO()
    _sys.print_exception(exc, out)
    return out.getvalue().rstrip("\n")


def _serve():
    spaces = {}
    mains = {}
    while True:
        invoke = _host("plugin/next", [])
        if invoke is None:
            return
        # The token that proves this loop is the one that answers the call: a local of this
        # function only, and a small integer (not an object that a handler could find).
        token = invoke.pop("token")
        name = invoke["name"] or "<plugin>"
        Plugin.jack = invoke["jack"] or ""
        Plugin.args = invoke["args"]
        space = spaces.get(name)
        if space is None:
            space = {
                "__name__": "__main__",
                "__file__": name,
                "Editor": Editor,
                "EditorError": EditorError,
                "Plugin": Plugin,
                "Outline": Outline,
                "Complement": Complement,
            }
            spaces[name] = space
            mains[name] = _main_of(space)
        if Plugin.jack == "SmartIndent":
            args = Plugin.args
            typed = args.get("char") if isinstance(args, dict) else None
            space["Indent"] = _Indent(typed if isinstance(typed, str) else "")
        else:
            space.pop("Indent", None)
        failure = None
        # While the handler runs, `import __main__` finds its own names, not the boot's (which is
        # what `__main__` is without this entry).
        _sys.modules["__main__"] = mains[name]
        try:
            exec(compile(invoke["source"], name, "exec"), space)
        except SystemExit:
            # sys.exit() in a handler ends the handler, not the host.
            pass
        except BaseException as exc:
            failure = _describe(exc)
        _sys.modules.pop("__main__", None)
        try:
            _respond(invoke["id"], failure, token)
        except EditorError:
            # Refused: the host does not take the answer (it has answered already). That is no
            # reason to stop serving.
            pass
        token = None


_serve()
