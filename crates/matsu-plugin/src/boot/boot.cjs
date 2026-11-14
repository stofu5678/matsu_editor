// The boot library of the JavaScript (Node.js) macros: node --require boot.cjs user.js
//
// It reads and writes the editor's JSON-RPC lines synchronously (docs/poc/json-rpc.md, spec
// sections 2 and 3), and puts the `Editor` object (api.gen.cjs) and `EditorError` in the global
// scope, so that the user's file needs no require.
'use strict';

const fs = require('fs');

// A `.ts` macro needs a Node.js that strips the types by itself (22.18 or later, 23.6 or later).
// An older one would stop with "Unknown file extension", so say what is wrong. This runs before
// Node.js opens the macro: --require files come first. (Mirrored by node_strips_types in
// session.rs.)
{
  const main = process.argv[1] || '';
  const [major, minor] = process.versions.node.split('.').map(Number);
  const strips = major > 23 || (major === 23 && minor >= 6) || (major === 22 && minor >= 18);
  if (/\.ts$/i.test(main) && !strips) {
    fs.writeSync(
      2,
      `TypeScript のマクロには Node.js 22.18 以降が要ります (見つかったのは v${process.versions.node})\n`,
    );
    process.exit(1);
  }
}

const RS = '\x1e';

function sleep(ms) {
  // A blocking sleep that does not spin the CPU.
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

// Writes all of the buffer to the file descriptor. A pipe that Node has made non-blocking
// answers EAGAIN when it is full: wait and try again. (process.stdout.write is asynchronous
// there, and a blocked event loop never flushes it.)
function writeAll(fd, buffer) {
  let offset = 0;
  while (offset < buffer.length) {
    try {
      offset += fs.writeSync(fd, buffer, offset, buffer.length - offset);
    } catch (err) {
      if (err.code === 'EAGAIN') {
        sleep(1);
      } else {
        throw err;
      }
    }
  }
}

// The user's output goes to the same descriptors synchronously, so that it stays in order with
// the requests (a request starts with RS, which the editor splits the line at).
function syncWrite(fd) {
  return function write(chunk, encoding, callback) {
    if (typeof encoding === 'function') {
      callback = encoding;
      encoding = undefined;
    }
    try {
      writeAll(fd, Buffer.isBuffer(chunk) || chunk instanceof Uint8Array ? chunk : Buffer.from(String(chunk), encoding));
    } catch (err) {
      // The editor has gone away; there is nobody to read it.
    }
    if (typeof callback === 'function') {
      process.nextTick(callback);
    }
    return true;
  };
}
process.stdout.write = syncWrite(1);
process.stderr.write = syncWrite(2);

// Escapes every code unit above ASCII as \uXXXX. A JS string is UTF-16, so a character above
// U+FFFF becomes a surrogate pair, which is what JSON wants.
function toAscii(json) {
  return json.replace(/[\u007f-￿]/g, (c) => '\\u' + c.charCodeAt(0).toString(16).padStart(4, '0'));
}

// Reads one line from the standard input, without its line break. null at the end of the input.
// Bytes that came after the line break stay in `pending` for the next call.
let pending = Buffer.alloc(0);
let scanned = 0;
let eof = false;
const readBuffer = Buffer.allocUnsafe(65536);
function nextLine() {
  for (;;) {
    const lf = pending.indexOf(10, scanned);
    if (lf >= 0) {
      const line = pending.toString('utf8', 0, lf);
      pending = pending.subarray(lf + 1);
      scanned = 0;
      return line.replace(/\r$/, '');
    }
    scanned = pending.length;
    if (eof) {
      if (pending.length === 0) {
        return null;
      }
      const rest = pending.toString('utf8');
      pending = Buffer.alloc(0);
      scanned = 0;
      return rest.replace(/\r$/, '');
    }
    let n;
    try {
      n = fs.readSync(0, readBuffer, 0, readBuffer.length, null);
    } catch (err) {
      if (err.code === 'EAGAIN') {
        sleep(1);
        continue;
      }
      if (err.code !== 'EOF') {
        throw err;
      }
      n = 0;
    }
    if (n === 0) {
      eof = true;
    } else {
      pending = Buffer.concat([pending, readBuffer.subarray(0, n)]);
    }
  }
}

class EditorError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'EditorError';
    this.code = code;
  }
}

let nextId = 1;

// Sends a request and waits for its response.
function call(method, params) {
  const id = nextId++;
  const line = RS + toAscii(JSON.stringify({ jsonrpc: '2.0', id, method, params })) + '\n';
  writeAll(1, Buffer.from(line, 'latin1'));
  for (;;) {
    const text = nextLine();
    if (text === null) {
      throw new Error('The editor closed the connection before it answered ' + method);
    }
    if (text === '') {
      continue;
    }
    const message = JSON.parse(text);
    if (message.id !== id) {
      continue; // a notification, or an answer to something that is no longer waited for
    }
    if (message.error) {
      const err = new EditorError(message.error.code, message.error.message);
      Error.captureStackTrace(err, call);
      throw err;
    }
    return message.result;
  }
}

// The first line is the initialize notification.
let info = null;
{
  const text = nextLine();
  if (text) {
    const message = JSON.parse(text);
    if (message.method === 'initialize') {
      info = message.params;
    }
  }
}

const Editor = require('./api.gen.cjs')(call);
Editor.info = info;
globalThis.Editor = Editor;
globalThis.EditorError = EditorError;

// A plugin host (ADR-0026): `initialize` says kind "plugin", and there is no script. The editor
// sends `plugin/invoke` (the jack, the file of the handler, its text, the arguments) and waits for
// the answer; the process stays for the next call when the plugin is resident. The QuickJS host has
// the same objects (crates/matsu-script/src/quickjs.js): `Plugin`, `Outline`, `Complement` and, for
// the jack SmartIndent, `Indent`.
// The calls that the editor answers itself; each takes the arguments of Sakura's plugin object.
const PLUGIN_CALLS = [
  'GetPluginDir',
  'GetDef',
  'GetOption',
  'SetOption',
  'GetString',
  'GetCommandNo',
  'AddCommand',
];
const OUTLINE_CALLS = [
  'AddFuncInfo',
  'AddFuncInfo2',
  'AddFuncInfo3',
  'AddFuncInfo4',
  'SetTitle',
  'SetListType',
  'SetLabel',
];

function bindCalls(target, prefix, names) {
  for (const name of names) {
    target[name] = function (...args) {
      // A trailing undefined is a missing argument, not a null.
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call(prefix + name, args);
    };
  }
}

function respond(id, error) {
  const message =
    error === undefined
      ? { jsonrpc: '2.0', id, result: { ok: true } }
      : { jsonrpc: '2.0', id, error: { code: -32000, message: error } };
  writeAll(1, Buffer.from(RS + toAscii(JSON.stringify(message)) + '\n', 'latin1'));
}

// The text of a failure for the editor: the error with the frames of the handler. The frames of
// this boot and of Node.js itself are left out.
function describeError(err) {
  const clean = (text) =>
    text
      .split('\n')
      .filter((line) => !/boot\.cjs:\d+|\(node:internal\//.test(line) && !/^\s+at node:internal\//.test(line))
      .join('\n');
  if (err instanceof EditorError) {
    const frames = clean(String(err.stack || '').split('\n').slice(1).join('\n'));
    return 'EditorError ' + err.code + ': ' + err.message + (frames ? '\n' + frames : '');
  }
  return clean(String((err && err.stack) || err));
}

async function servePlugin(identity) {
  const path = require('path');
  const Module = require('module');

  const Plugin = { id: identity.id, dir: identity.dir, jack: '', args: null };
  bindCalls(Plugin, 'Plugin.', PLUGIN_CALLS);
  const Outline = {};
  bindCalls(Outline, 'Outline.', OUTLINE_CALLS);
  const Complement = {
    // The word to complete, and the options, come with the call.
    GetCurrentWord: () => (Plugin.args && typeof Plugin.args.word === 'string' ? Plugin.args.word : ''),
    GetOption: () => (Plugin.args && Number.isInteger(Plugin.args.option) ? Plugin.args.option : 0),
    AddList(word) {
      return call('Complement.AddList', word === undefined ? [] : [String(word)]);
    },
  };
  globalThis.Plugin = Plugin;
  globalThis.Outline = Outline;
  globalThis.Complement = Complement;

  // The timers that the handler starts are known, so that a failed call can cancel them. The
  // boot's own waiting uses the real functions.
  const realSetImmediate = setImmediate;
  const realSetTimeout = setTimeout;
  const pending = new Set();
  const track = (start, release) =>
    function (callback, ...rest) {
      let handle;
      // A `function`, so that the receiver (Node.js hands the timer to a callback as `this`) and
      // the arguments reach the handler's callback as they would without the wrapper.
      const wrapped = function (...args) {
        if (release === 'once') pending.delete(handle);
        return callback.apply(this, args);
      };
      handle = start(wrapped, ...rest);
      pending.add(handle);
      return handle;
    };
  const realClearTimeout = clearTimeout;
  const realClearImmediate = clearImmediate;
  globalThis.setTimeout = track(realSetTimeout, 'once');
  globalThis.setInterval = track(setInterval, 'repeat');
  globalThis.setImmediate = track(realSetImmediate, 'once');
  function cancelPending() {
    for (const handle of pending) {
      realClearTimeout(handle);
      realClearImmediate(handle);
    }
    pending.clear();
  }

  // The files of the plugin folder, as the editor checked them (the text the user agreed to),
  // by their file URL. `module.registerHooks` (22.15, 23.5) gives every `import` and `require`
  // from a plugin file these texts and nothing from the disk; a file that is not here (outside
  // the folder, a package, a file added later) is refused.
  const { pathToFileURL } = require('url');
  const verified = new Map();
  const modules = new Map();
  let hooked = false;
  const [nodeMajor, nodeMinor] = process.versions.node.split('.').map(Number);
  const strips =
    nodeMajor > 23 || (nodeMajor === 23 && nodeMinor >= 6) || (nodeMajor === 22 && nodeMinor >= 18);

  function packageType(url) {
    let dir = new URL('.', url);
    for (;;) {
      const text = verified.get(new URL('package.json', dir).href);
      if (text !== undefined) {
        try {
          return JSON.parse(text).type;
        } catch (err) {
          return undefined;
        }
      }
      const up = new URL('..', dir);
      if (up.href === dir.href) return undefined;
      dir = up;
    }
  }

  function formatOf(url) {
    const file = url.slice(url.lastIndexOf('/') + 1);
    const ext = file.slice(file.lastIndexOf('.')).toLowerCase();
    switch (ext) {
      case '.mjs':
        return 'module';
      case '.cjs':
        return 'commonjs';
      case '.json':
        return 'json';
      case '.js':
        return packageType(url) === 'module' ? 'module' : 'commonjs';
      case '.ts':
      case '.mts':
      case '.cts':
        if (!strips) {
          throw new Error(
            'TypeScript には Node.js 22.18 以降（23 系は 23.6 以降）が要ります (見つかったのは v' +
              process.versions.node +
              ')',
          );
        }
        // A `.ts` file is an ES module, as the examples of the help; `.cts` is CommonJS.
        return ext === '.cts' ? 'commonjs-typescript' : 'module-typescript';
      default:
        throw new Error(url + ' は読み込めません');
    }
  }

  const EXTENSIONS = ['.js', '.cjs', '.json', '.ts'];
  function resolveVerified(specifier, context, nextResolve) {
    if (specifier.startsWith('node:') || Module.isBuiltin(specifier)) {
      return nextResolve(specifier, context);
    }
    if (!/^(\.{1,2}\/|\/|file:)/.test(specifier)) {
      throw Object.assign(
        new Error(
          specifier + ' はパッケージです。プラグインは、自分のフォルダーのファイルと Node.js の標準のモジュールだけ読み込めます',
        ),
        { code: 'ERR_MODULE_NOT_FOUND' },
      );
    }
    const base = context.parentURL || pathToFileURL(identity.dir + path.sep).href;
    const target = new URL(specifier, base);
    target.search = '';
    target.hash = '';
    const candidates = [target.href];
    if (Array.isArray(context.conditions) && context.conditions.includes('require')) {
      // The CommonJS way: no extension, or a folder.
      for (const ext of EXTENSIONS) candidates.push(target.href + ext);
      for (const ext of EXTENSIONS) candidates.push(target.href.replace(/\/?$/, '/index') + ext);
    }
    for (const url of candidates) {
      if (verified.has(url)) return { url, shortCircuit: true };
    }
    throw Object.assign(
      new Error(specifier + ' は、同意したプラグインのフォルダーにありません。同意したファイルだけ読み込めます'),
      { code: 'ERR_MODULE_NOT_FOUND' },
    );
  }

  function loadVerified(url, context, nextLoad) {
    if (url.startsWith('node:')) return nextLoad(url, context);
    const source = verified.get(url);
    if (source === undefined) {
      throw Object.assign(new Error(url + ' は、同意したプラグインのフォルダーにありません'), {
        code: 'ERR_MODULE_NOT_FOUND',
      });
    }
    return { format: formatOf(url), source, shortCircuit: true };
  }

  function installVerifiedFiles(files) {
    for (const file of files) {
      verified.set(pathToFileURL(file.path).href, file.source);
    }
    if (!hooked && typeof Module.registerHooks === 'function') {
      Module.registerHooks({ resolve: resolveVerified, load: loadVerified });
      hooked = true;
    }
  }

  // A promise that is rejected and that nobody handles fails the call (as in the QuickJS host).
  let unhandled = null;
  process.on('unhandledRejection', (reason) => {
    if (unhandled === null) unhandled = { reason };
  });
  // An error thrown in a callback of a timer is the failure of the call that started the timer.
  process.on('uncaughtException', (reason) => {
    if (unhandled === null) unhandled = { reason };
  });

  async function run(invoke) {
    Plugin.jack = invoke.jack;
    Plugin.args = invoke.args === undefined ? null : invoke.args;
    if (invoke.jack === 'SmartIndent') {
      const typed = Plugin.args && typeof Plugin.args.char === 'string' ? Plugin.args.char : '';
      globalThis.Indent = { GetChar: () => typed };
    } else {
      delete globalThis.Indent;
    }
    const name = invoke.name;
    // The editor sends the module files of the folder, as it checked them, with the first call.
    if (Array.isArray(invoke.files)) {
      installVerifiedFiles(invoke.files);
    }
    if (/\.(mjs|ts)$/.test(name)) {
      // An ES module or a TypeScript file: loaded once (the first call), and its default export,
      // a function, is called for each call. Nothing is read from the disk: the hooks serve the
      // text the editor checked.
      if (!hooked) {
        throw new Error(
          name +
            ' は動かせません。.mjs と .ts のプラグインには、Node.js 22.15 以降（23 系は 23.5 以降）が要ります (見つかったのは v' +
            process.versions.node +
            ')',
        );
      }
      const href = pathToFileURL(name).href;
      let loaded = modules.get(href);
      if (loaded === undefined) {
        loaded = await import(href);
        modules.set(href, loaded);
      }
      if (typeof loaded.default !== 'function') {
        throw new Error(name + ' は、関数を default で export してください (export default function () { ... })');
      }
      await loaded.default(Plugin.args);
    } else {
      if (!/\.cjs$/i.test(name)) {
        throw new Error(name + ' は動かせません。Node.js のプラグインは .cjs、.mjs、.ts のどれかです');
      }
      // The text the editor read (the one that the user agreed to) is what runs. A fresh module
      // for each call, so that it runs again; what it keeps goes in `globalThis`.
      const mod = new Module(name, null);
      mod.filename = name;
      mod.paths = Module._nodeModulePaths(path.dirname(name));
      mod._compile(invoke.source, name);
    }
    // The call ends when the handler's own work is done: the timers that it started (a callback of
    // `setTimeout` may call `Editor`) run first, within the time limit that the editor holds (a
    // timer that never ends, a `setInterval`, makes the call time out). Otherwise the callback would
    // wait while this loop blocks for the next call, and then run in that call, on its document.
    // An error (a rejection that nobody handles, an exception in a callback) ends the call at once,
    // and the callbacks that are still waiting are cancelled.
    for (;;) {
      await new Promise((resolve) => realSetImmediate(resolve));
      if (unhandled !== null) break;
      const timers = process
        .getActiveResourcesInfo()
        .some((type) => type === 'Timeout' || type === 'Immediate');
      if (!timers) break;
      await new Promise((resolve) => realSetTimeout(resolve, 1));
      if (unhandled !== null) break;
    }
    cancelPending();
    if (unhandled !== null) {
      const { reason } = unhandled;
      unhandled = null;
      throw reason;
    }
  }

  for (;;) {
    const text = nextLine();
    if (text === null) break;
    if (text === '') continue;
    let message;
    try {
      message = JSON.parse(text);
    } catch (err) {
      continue;
    }
    if (message.method === 'plugin/shutdown') break;
    if (message.method !== 'plugin/invoke') continue;
    let failure;
    try {
      await run(message.params || {});
    } catch (err) {
      failure = describeError(err);
    }
    unhandled = null;
    respond(message.id, failure);
  }
  process.exit(0);
}

if (info && info.kind === 'plugin') {
  const identity = info.plugin || {};
  delete info.plugin;
  servePlugin(identity).catch((err) => {
    process.stderr.write(String((err && err.stack) || err) + '\n');
    process.exit(1);
  });
}
