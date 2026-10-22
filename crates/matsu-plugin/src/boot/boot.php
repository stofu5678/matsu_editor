<?php
// The boot library of the PHP macros: php boot.php user.php
//
// It reads and writes the editor's JSON-RPC lines synchronously (docs/poc/json-rpc.md, spec
// sections 2 and 3), and runs the user's file with the class `Editor` (api_gen.php) and the
// class `EditorError` ready, so that the file needs no require. The file is run by `require`
// in the global scope, so the line numbers of errors are the file's own. The file starts with
// `<?php`, as any PHP file does.

// The editor starts PHP with these set too (the errors go to the standard error); this is for a
// run by hand.
ini_set('display_errors', 'stderr');
ini_set('log_errors', '0');
ini_set('html_errors', '0');

/** The editor answered a call with an error (a JSON-RPC error object). getCode() is its code. */
final class EditorError extends Exception
{
    public function __construct($code, $message)
    {
        parent::__construct((string) $message, (int) $code);
    }
}

final class MatsuBoot
{
    private const RS = "\x1e";

    private static $nextId = 1;

    /** Writes all of the text to the standard output. */
    private static function write(string $text): void
    {
        $length = strlen($text);
        $offset = 0;
        while ($offset < $length) {
            $written = fwrite(STDOUT, substr($text, $offset));
            if ($written === false || $written === 0) {
                // The editor has gone away; there is nobody to read it.
                exit(1);
            }
            $offset += $written;
        }
        fflush(STDOUT);
    }

    /** One line from the standard input, or null at the end of the input. */
    public static function readLine(): ?string
    {
        $line = fgets(STDIN);
        return $line === false ? null : $line;
    }

    /** Sends a request and waits for its response. */
    public static function call(string $method, array $params)
    {
        $id = self::$nextId++;
        // json_encode writes only ASCII by default (non-ASCII characters become \uXXXX).
        $json = json_encode(
            ['jsonrpc' => '2.0', 'id' => $id, 'method' => $method, 'params' => array_values($params)],
            JSON_THROW_ON_ERROR
        );
        self::write(self::RS . $json . "\n");
        while (true) {
            $text = self::readLine();
            if ($text === null) {
                throw new RuntimeException("The editor closed the connection before it answered $method");
            }
            $text = trim($text);
            if ($text === '') {
                continue;
            }
            $message = json_decode($text, true, 512, JSON_THROW_ON_ERROR);
            if (!is_array($message) || ($message['id'] ?? null) !== $id) {
                continue; // a notification, or an answer to something that is no longer waited for
            }
            if (isset($message['error'])) {
                $error = $message['error'];
                throw new EditorError($error['code'] ?? 0, $error['message'] ?? '');
            }
            return $message['result'] ?? null;
        }
    }

    /** Reads `initialize`, installs the API, and returns the path of the user's file. */
    public static function setUp(array $argv): string
    {
        // The first line is the initialize notification.
        $info = null;
        $first = self::readLine();
        if ($first !== null && trim($first) !== '') {
            $message = json_decode(trim($first), true);
            if (is_array($message) && ($message['method'] ?? null) === 'initialize') {
                $info = $message['params'] ?? null;
            }
        }
        require __DIR__ . '/api_gen.php';
        Editor::install([self::class, 'call']);
        Editor::$info = $info;
        return realpath($argv[1]) ?: $argv[1];
    }
}

$_matsu_user_file = MatsuBoot::setUp($argv);
// The user's file sees its own name and arguments, as in `php user.php`.
$argv = array_slice($argv, 1);
$argc = count($argv);
$_SERVER['argv'] = $argv;
$_SERVER['argc'] = $argc;
$_SERVER['SCRIPT_FILENAME'] = $_SERVER['SCRIPT_NAME'] = $_SERVER['PHP_SELF'] = $_matsu_user_file;
require $_matsu_user_file;
