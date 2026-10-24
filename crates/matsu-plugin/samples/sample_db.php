<?php
// Sample macro (PHP): runs the SQL of the document (or of the selection) on a database and
// writes the result to the output panel (ADR-0023).
//
// This one uses SQLite through PDO (the pdo_sqlite extension). Another database is the same few
// lines with its own PDO driver (pdo_pgsql, pdo_mysql, pdo_sqlsrv), enabled by the user in
// php.ini; these were written from the PHP manual and not tried:
//
//   PostgreSQL   new PDO('pgsql:host=localhost;dbname=app')     // ~/.pgpass for the password
//   MySQL        new PDO('mysql:host=localhost;dbname=app;charset=utf8mb4', null, null)   // option file
//
// Never write a password in a macro: use the driver's own way (a password file, an option
// file, the environment).

$database = ':memory:'; // a file, e.g. 'C:\data\app.db'; ':memory:' is gone when the macro ends

// The SQL: the selection if there is one, else the whole document. A statement ends at the end
// of a line: put each statement on lines of its own.
$firstLine = 1;
if (Editor::IsTextSelected()) {
    $sql = Editor::GetSelectedString(0);
    // so that an error names the line of the document (not the row of a wrapped line)
    $firstLine = Editor::LayoutToLogicLineNum(Editor::GetSelectLineFrom());
} else {
    $sql = '';
    $count = Editor::GetLineCount(0);
    for ($i = 1; $i <= $count; $i++) {
        $sql .= Editor::GetLineStr($i);
    }
}

$name = Editor::GetFilename() ?: '(無題)';
$pdo = new PDO('sqlite:' . $database);
$pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

// A statement ends at a line that ends with ";" (simple: a ";" at the end of a line inside a
// string or a trigger is taken for the end), and the line it starts at is kept for the error.
$statement = '';
$start = $firstLine;
$number = $firstLine - 1;
$run = function () use ($pdo, &$statement, &$start, $name) {
    try {
        $result = $pdo->query($statement);
        if ($result->columnCount() > 0) {
            $names = [];
            for ($c = 0; $c < $result->columnCount(); $c++) {
                $names[] = $result->getColumnMeta($c)['name'];
            }
            echo implode("\t", $names) . "\n";
            foreach ($result->fetchAll(PDO::FETCH_NUM) as $row) {
                echo implode("\t", array_map(fn($v) => $v ?? '', $row)) . "\n";
            }
        } else {
            echo 'OK (' . $result->rowCount() . " rows)\n";
        }
    } catch (PDOException $e) {
        // FILE:LINE: message, the form that a tag jump understands
        echo $name . ':' . $start . ': ERROR: ' . $e->getMessage() . "\n";
    }
};
foreach (preg_split('/(?<=\n)/', $sql, -1, PREG_SPLIT_NO_EMPTY) as $line) {
    $number++;
    if (trim($statement) === '') {
        $statement = '';
        $start = $number;
    }
    $statement .= $line;
    // a "--" comment after the ";" is not part of the line's end
    if (str_ends_with(rtrim(preg_replace('/--.*$/', '', $line)), ';')) {
        $run();
        $statement = '';
    }
}
// what is left may be only a comment
if (trim(preg_replace('/--.*$/m', '', $statement)) !== '') {
    echo $name . ':' . $start . ": ERROR: the last statement does not end with ';'\n";
}
