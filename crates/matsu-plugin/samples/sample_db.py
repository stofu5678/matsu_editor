# Sample macro (Python): runs the SQL of the document (or of the selection) on a database and
# writes the result to the output panel (ADR-0023). Run it from the macro menu.
#
# This one uses SQLite, which Python has built in. Another database is the same few lines with
# its own driver; the driver is installed by the user (`pip install ...`), MATSU Editor does not
# bring it. These were written from the drivers' documentation and not tried:
#
#   PostgreSQL   import psycopg;                 conn = psycopg.connect("dbname=app")   # ~/.pgpass for the password
#   MySQL        import mysql.connector;         conn = mysql.connector.connect(option_files="~/.my.cnf")
#   BigQuery     from google.cloud import bigquery; client = bigquery.Client()           # `gcloud auth application-default login`
#
# Never write a password in a macro: use the driver's own way (a password file, an option file,
# the environment, the cloud's login).

import re
import sqlite3

DATABASE = ":memory:"  # a file, e.g. r"C:\data\app.db"; ":memory:" is gone when the macro ends

# The SQL: the selection if there is one, else the whole document. A line number counts from 1.
# A statement ends at the end of a line: put each statement on lines of its own.
first_line = 1
if Editor.IsTextSelected():
    sql = Editor.GetSelectedString(0)
    # so that an error names the line of the document (not the row of a wrapped line)
    first_line = Editor.LayoutToLogicLineNum(Editor.GetSelectLineFrom())
else:
    count = Editor.GetLineCount(0)
    sql = "".join(Editor.GetLineStr(i) for i in range(1, count + 1))

name = Editor.GetFilename() or "(無題)"
# autocommit: the BEGIN, COMMIT and ROLLBACK of the document are the ones that count
conn = sqlite3.connect(DATABASE, isolation_level=None)

# Split into statements the way SQLite does (a ";" inside a string or a trigger does not end
# one), and keep the line each statement starts at for the error line.
statement, start, number = "", first_line, first_line - 1
for line in sql.splitlines(keepends=True):
    number += 1
    if not statement.strip():
        statement, start = "", number
    statement += line
    if sqlite3.complete_statement(statement):
        try:
            cursor = conn.execute(statement)
            if cursor.description:  # a statement that returns rows
                print("\t".join(column[0] for column in cursor.description))
                for row in cursor:
                    print("\t".join("" if value is None else str(value) for value in row))
            else:
                print("OK" if cursor.rowcount < 0 else f"OK ({cursor.rowcount} rows)")
        except sqlite3.Error as error:
            # FILE:LINE: message, the form that a tag jump understands
            print(f"{name}:{start}: ERROR: {error}")
        statement = ""

# what is left may be only a comment
left = re.sub(r"/\*.*?\*/", "", statement, flags=re.S)
if "\n".join(line.split("--")[0] for line in left.splitlines()).strip():
    print(f"{name}:{start}: ERROR: the last statement does not end with ';'")
# nothing is committed here: only the COMMIT of the document commits a transaction it began
conn.close()
