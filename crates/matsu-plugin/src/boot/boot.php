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
        if (is_array($info) && ($info['kind'] ?? null) === 'plugin') {
            // A plugin host has no script: the handlers come with each call.
            self::$plugin = is_array($info['plugin'] ?? null) ? $info['plugin'] : [];
            unset(Editor::$info['plugin']);
            return '';
        }
        return realpath($argv[1]) ?: $argv[1];
    }

    /** The identity of the plugin (`id`, `dir`) when this process is a plugin host, else null. */
    public static $plugin = null;

    /**
     * The next `plugin/invoke` of the editor, or null when the conversation ends. Sets the objects
     * of the handler (`Plugin`, `Indent`) for it.
     */
    public static function nextInvoke(): ?array
    {
        while (true) {
            $text = self::readLine();
            if ($text === null) {
                return null;
            }
            $text = trim($text);
            if ($text === '') {
                continue;
            }
            $message = json_decode($text, true);
            if (!is_array($message)) {
                continue;
            }
            $method = $message['method'] ?? null;
            if ($method === 'plugin/shutdown') {
                return null;
            }
            if ($method !== 'plugin/invoke') {
                continue;
            }
            $params = is_array($message['params'] ?? null) ? $message['params'] : [];
            Plugin::$jack = is_string($params['jack'] ?? null) ? $params['jack'] : '';
            Plugin::$args = $params['args'] ?? null;
            $typed = '';
            if (Plugin::$jack === 'SmartIndent' && is_array(Plugin::$args) && is_string(Plugin::$args['char'] ?? null)) {
                $typed = Plugin::$args['char'];
            }
            Indent::$typed = Plugin::$jack === 'SmartIndent' ? $typed : null;
            return [
                'id' => $message['id'] ?? null,
                'name' => is_string($params['name'] ?? null) ? $params['name'] : '<plugin>',
                'source' => is_string($params['source'] ?? null) ? $params['source'] : '',
            ];
        }
    }

    /**
     * The code to give to `eval` for the text of a handler. The text is a PHP file: it starts with
     * `<?php`, which `eval` does not take, so the tag goes (the rest stays, so the line numbers are
     * the file's own and `declare(strict_types=1)` is still the first statement). A text that does
     * not start with the tag starts outside PHP (`?>`). A byte order mark would be printed, so it
     * goes too.
     */
    public static function code(string $source): string
    {
        if (strncmp($source, "\xEF\xBB\xBF", 3) === 0) {
            $source = substr($source, 3);
        }
        if (preg_match('/\A<\?php(?=\s)/i', $source) === 1) {
            return substr($source, 5);
        }
        return '?>' . $source;
    }

    /** The text of a failure for the editor: the error with the places in the handler. */
    public static function describe(Throwable $error, string $name): string
    {
        $where = static function (?string $file, $line) use ($name): string {
            $inEval = $file !== null && strpos($file, "eval()'d code") !== false;
            return ($inEval ? $name : (string) $file) . ':' . $line;
        };
        $text = $error instanceof EditorError
            ? 'EditorError ' . $error->getCode() . ': ' . $error->getMessage()
            : get_class($error) . ': ' . $error->getMessage();
        $text .= "\n  at " . $where($error->getFile(), $error->getLine());
        foreach ($error->getTrace() as $frame) {
            $file = $frame['file'] ?? null;
            if ($file !== null && strpos($file, "eval()'d code") !== false) {
                $text .= "\n  at " . $where($file, $frame['line'] ?? 0);
            }
        }
        return $text;
    }

    /** Answers the invoke. */
    public static function respond($id, ?string $failure): void
    {
        // The marker: a line on standard error that says everything the handler wrote there before
        // its answer has been written (the editor reads the two streams on two threads).
        // `fwrite` on a closed STDERR throws a TypeError in PHP 8: the answer goes out all the same.
        try {
            fwrite(STDERR, "\x1fmatsu-stderr-done " . json_encode($id) . "\n");
            fflush(STDERR);
        } catch (\Throwable $_matsu_ignored) {
        }
        fflush(STDOUT);
        $message = $failure === null
            ? ['jsonrpc' => '2.0', 'id' => $id, 'result' => ['ok' => true]]
            : ['jsonrpc' => '2.0', 'id' => $id, 'error' => ['code' => -32000, 'message' => $failure]];
        // JSON_INVALID_UTF8_SUBSTITUTE: a message with bytes that are not UTF-8 must still go.
        self::write(self::RS . json_encode($message, JSON_INVALID_UTF8_SUBSTITUTE) . "\n");
    }
}

/** The `Plugin` object of a plugin (Sakura's CPluginIfObj): the editor answers the calls. */
final class Plugin
{
    public static $id = '';
    public static $dir = '';
    public static $jack = '';
    public static $args = null;

    private static function ask(string $name, array $args)
    {
        // A trailing null is a missing argument, not a null.
        while ($args !== [] && $args[count($args) - 1] === null) {
            array_pop($args);
        }
        return MatsuBoot::call('Plugin.' . $name, $args);
    }

    public static function GetPluginDir(...$args) { return self::ask('GetPluginDir', $args); }
    public static function GetDef(...$args) { return self::ask('GetDef', $args); }
    public static function GetOption(...$args) { return self::ask('GetOption', $args); }
    public static function SetOption(...$args) { return self::ask('SetOption', $args); }
    public static function GetString(...$args) { return self::ask('GetString', $args); }
    public static function GetCommandNo(...$args) { return self::ask('GetCommandNo', $args); }
    public static function AddCommand(...$args) { return self::ask('AddCommand', $args); }
}

/** The `Outline` object of the jack Outline (Sakura's COutlineIfObj). */
final class Outline
{
    private static function ask(string $name, array $args)
    {
        while ($args !== [] && $args[count($args) - 1] === null) {
            array_pop($args);
        }
        return MatsuBoot::call('Outline.' . $name, $args);
    }

    public static function AddFuncInfo(...$args) { return self::ask('AddFuncInfo', $args); }
    public static function AddFuncInfo2(...$args) { return self::ask('AddFuncInfo2', $args); }
    public static function AddFuncInfo3(...$args) { return self::ask('AddFuncInfo3', $args); }
    public static function AddFuncInfo4(...$args) { return self::ask('AddFuncInfo4', $args); }
    public static function SetTitle(...$args) { return self::ask('SetTitle', $args); }
    public static function SetListType(...$args) { return self::ask('SetListType', $args); }
    public static function SetLabel(...$args) { return self::ask('SetLabel', $args); }
}

/**
 * The `Complement` object of the completion jacks (Sakura's CComplementIfObj). The word and the
 * option come with the call; the candidates go to the editor.
 */
final class Complement
{
    public static function GetCurrentWord(): string
    {
        $word = is_array(Plugin::$args) ? (Plugin::$args['word'] ?? null) : null;
        return is_string($word) ? $word : '';
    }

    public static function GetOption(): int
    {
        $option = is_array(Plugin::$args) ? (Plugin::$args['option'] ?? null) : null;
        return is_int($option) ? $option : 0;
    }

    public static function AddList($word = null)
    {
        return MatsuBoot::call('Complement.AddList', $word === null ? [] : [(string) $word]);
    }
}

/** The `Indent` object of the jack SmartIndent: GetChar answers the typed char ("" in other jacks). */
final class Indent
{
    public static $typed = null;

    public static function GetChar(): string
    {
        return self::$typed ?? '';
    }
}

$_matsu_user_file = MatsuBoot::setUp($argv);
if (MatsuBoot::$plugin !== null) {
    Plugin::$id = (string) (MatsuBoot::$plugin['id'] ?? '');
    Plugin::$dir = (string) (MatsuBoot::$plugin['dir'] ?? '');
    // The folder of the plugin, so that a handler can `require` its own files.
    if (Plugin::$dir !== '') {
        set_include_path(Plugin::$dir . PATH_SEPARATOR . get_include_path());
    }
    // The handler runs by `eval` at this level, so its variables are global and stay for the next
    // call (the plugin is resident). A function or class that it declares exists after the first
    // call: a second call of the same file has to guard the declaration or `require_once` a file.
    while (($_matsu_invoke = MatsuBoot::nextInvoke()) !== null) {
        $_matsu_failure = null;
        try {
            eval(MatsuBoot::code($_matsu_invoke['source']));
        } catch (Throwable $_matsu_error) {
            $_matsu_failure = MatsuBoot::describe($_matsu_error, $_matsu_invoke['name']);
        }
        MatsuBoot::respond($_matsu_invoke['id'], $_matsu_failure);
    }
    exit(0);
}
// The user's file sees its own name and arguments, as in `php user.php`.
$argv = array_slice($argv, 1);
$argc = count($argv);
$_SERVER['argv'] = $argv;
$_SERVER['argc'] = $argc;
$_SERVER['SCRIPT_FILENAME'] = $_SERVER['SCRIPT_NAME'] = $_SERVER['PHP_SELF'] = $_matsu_user_file;
require $_matsu_user_file;
