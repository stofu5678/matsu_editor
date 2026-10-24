// Sample macro (JavaScript, Node.js ES module): runs the SQL of the document (or of the
// selection) on a database and writes the result to the output panel (ADR-0023).
//
// This one uses SQLite through `node:sqlite` (Node.js 22.5 or later; Node.js 22 needs
// `--experimental-sqlite` before 22.13). Another database is the same few lines with its own
// package, installed by the user (`npm install ...`); these were written from the packages'
// documentation and not tried:
//
//   PostgreSQL   import pg from 'pg';                  const client = new pg.Client();   // PG* variables, ~/.pgpass
//   MySQL        import mysql from 'mysql2/promise';   const conn = await mysql.createConnection({ uri })
//   BigQuery     import { BigQuery } from '@google-cloud/bigquery'; const bq = new BigQuery();  // gcloud auth
//
// Never write a password in a macro: use the package's own way (a password file, the
// environment, the cloud's login).

import { DatabaseSync } from 'node:sqlite';

const DATABASE = ':memory:'; // a file, e.g. 'C:\\data\\app.db'; ':memory:' is gone when the macro ends

// The SQL: the selection if there is one, else the whole document. A statement ends at the end
// of a line: put each statement on lines of its own.
let sql;
let firstLine = 1;
if (Editor.IsTextSelected()) {
  sql = Editor.GetSelectedString(0);
  // so that an error names the line of the document (not the row of a wrapped line)
  firstLine = Editor.LayoutToLogicLineNum(Editor.GetSelectLineFrom());
} else {
  const count = Editor.GetLineCount(0);
  const lines = [];
  for (let i = 1; i <= count; i++) lines.push(Editor.GetLineStr(i));
  sql = lines.join('');
}

const name = Editor.GetFilename() || '(無題)';
const db = new DatabaseSync(DATABASE);

// A statement ends at a line that ends with ";" (simple: a ";" at the end of a line inside a
// string or a trigger is taken for the end), and the line it starts at is kept for the error.
let statement = '';
let start = firstLine;
let number = firstLine - 1;
const run = () => {
  try {
    const prepared = db.prepare(statement);
    // rows as arrays keep the order of the columns (Node.js 24 and later; older ones give objects)
    if (typeof prepared.setReturnArrays === 'function') prepared.setReturnArrays(true);
    if (prepared.columns().length > 0) {
      const rows = prepared.all();
      console.log(prepared.columns().map((c) => c.name).join('\t'));
      for (const row of rows) {
        const values = Array.isArray(row) ? row : Object.values(row);
        console.log(values.map((v) => (v ?? '')).join('\t'));
      }
    } else {
      const result = prepared.run();
      console.log(`OK (${result.changes} rows)`);
    }
  } catch (error) {
    // FILE:LINE: message, the form that a tag jump understands
    console.log(`${name}:${start}: ERROR: ${error.message}`);
  }
};
for (const line of sql.split(/(?<=\n)/)) {
  number++;
  if (statement.trim() === '') {
    statement = '';
    start = number;
  }
  statement += line;
  // a "--" comment after the ";" is not part of the line's end
  if (line.replace(/--.*$/, '').trimEnd().endsWith(';')) {
    run();
    statement = '';
  }
}
if (statement.trim() !== '') {
  console.log(`${name}:${start}: ERROR: the last statement does not end with ';'`);
}
db.close();
