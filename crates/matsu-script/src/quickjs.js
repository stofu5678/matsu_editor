// The boot of the QuickJS engine. It is a function expression, called once with
// (raw, names, info, write):
//
// - raw: one Rust function per function of the macro API; it takes the array of arguments and
//   answers [true, result] or [false, code, message];
// - names: the names of those functions;
// - info: the `initialize` params without `script`;
// - write(fd, text): writes to the standard output (1) or error (2) and flushes at once.
//
// It puts `Editor`, `EditorError` and `console` in the global scope, as boot.cjs does for Node.js,
// and answers an object for Rust:
//   describe(value): the text for the standard error for an uncaught error or thrown value;
//   track(promise, reason, handled): called when a promise is rejected unhandled (handled false)
//     or when a handler is added later (true);
//   unhandled(): [reason] of the first promise that is still rejected and unhandled, or [].
(function (raw, names, info, write) {
  'use strict';

  const BOOT_FILE = 'matsu-script:boot';

  class EditorError extends Error {
    constructor(code, message) {
      super(message);
      this.name = 'EditorError';
      this.code = code;
    }
  }

  const Editor = {};
  for (const name of names) {
    const call = raw[name];
    Editor[name] = function (...args) {
      // A trailing undefined is a missing argument, not a null.
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      const answer = call(args);
      if (answer[0]) return answer[1];
      const error = new EditorError(answer[1], answer[2]);
      // As boot.cjs does for Node.js: the stack starts at the script's call.
      if (typeof Error.captureStackTrace === 'function') {
        Error.captureStackTrace(error, Editor[name]);
      }
      throw error;
    };
  }
  Editor.info = info;

  // What console.log shows for a value (a small version of Node.js's util.inspect).
  function show(value) {
    if (typeof value === 'string') return value;
    if (value instanceof Error) return stackOf(value);
    if (typeof value === 'symbol') return value.toString();
    if (typeof value === 'bigint') return value + 'n';
    if (typeof value === 'function') return '[Function: ' + (value.name || '(anonymous)') + ']';
    if (value !== null && typeof value === 'object') {
      try {
        const json = JSON.stringify(value);
        if (json !== undefined) return json;
      } catch (e) {
        // A cycle, or a toJSON that throws: fall through.
      }
    }
    try {
      return String(value);
    } catch (e) {
      return Object.prototype.toString.call(value);
    }
  }
  const line = (args) => args.map(show).join(' ') + '\n';
  const toStdout = (...args) => write(1, line(args));
  const toStderr = (...args) => write(2, line(args));
  globalThis.console = {
    log: toStdout,
    info: toStdout,
    debug: toStdout,
    error: toStderr,
    warn: toStderr,
    trace: toStderr,
  };

  // The error and its stack without the frames of this boot and of the native functions: the
  // first frame is the place in the user's script.
  function stackOf(error) {
    const head = error instanceof EditorError ? 'EditorError ' + error.code + ': ' + error.message : String(error);
    const frames = String(error.stack === undefined ? '' : error.stack)
      .split('\n')
      .filter((l) => l.trim() !== '' && !l.includes(BOOT_FILE) && !l.includes('(native)'));
    return [head].concat(frames).join('\n');
  }

  globalThis.Editor = Editor;
  globalThis.EditorError = EditorError;

  const rejected = new Map();
  return {
    describe(value) {
      return value instanceof Error ? stackOf(value) : 'Uncaught ' + show(value);
    },
    track(promise, reason, handled) {
      if (handled) rejected.delete(promise);
      else rejected.set(promise, reason);
    },
    unhandled() {
      for (const reason of rejected.values()) return [reason];
      return [];
    },
  };
})
