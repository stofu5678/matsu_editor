// The boot library of the JavaScript (Node.js) macros: node --require boot.cjs user.js
//
// It reads and writes the editor's JSON-RPC lines synchronously (docs/poc/json-rpc.md, spec
// sections 2 and 3), and puts the `Editor` object (api.gen.cjs) and `EditorError` in the global
// scope, so that the user's file needs no require.
'use strict';

const fs = require('fs');

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
