# The boot library of the Ruby macros: ruby -E UTF-8 boot.rb user.rb
#
# It reads and writes the editor's JSON-RPC lines synchronously (docs/poc/json-rpc.md, spec
# sections 2 and 3), and loads the user's file with `Editor` and `EditorError` as top-level
# constants, so that the file needs no require. The file is run by `load`, so the line numbers
# of errors are the file's own.
require 'json'

# Bytes in and out, so that Windows does not turn the line break into CR LF and the user's
# output stays UTF-8 whatever the console code page is.
$stdout.binmode
$stderr.binmode
$stdin.binmode
$stdout.sync = true
$stderr.sync = true

# The editor answered a call with an error (a JSON-RPC error object).
class EditorError < StandardError
  attr_reader :code

  def initialize(code, message)
    super(message)
    @code = code
  end
end

module UmeBoot
  RS = "\x1e"

  @next_id = 1

  def self.read_line
    line = $stdin.gets("\n")
    line&.force_encoding(Encoding::UTF_8)
  end

  # Sends a request and waits for its response.
  def self.call(method, params)
    id = @next_id
    @next_id += 1
    # ascii_only: non-ASCII characters become \uXXXX (surrogate pairs above U+FFFF).
    json = JSON.generate({ 'jsonrpc' => '2.0', 'id' => id, 'method' => method, 'params' => params },
                         ascii_only: true)
    $stdout.write("#{RS}#{json}\n")
    loop do
      text = read_line
      raise EOFError, "The editor closed the connection before it answered #{method}" if text.nil?

      text = text.strip
      next if text.empty?

      message = JSON.parse(text)
      next unless message['id'] == id # a notification, or an answer to something no longer waited for

      if message['error']
        error = message['error']
        raise EditorError.new(error['code'], error['message'])
      end
      return message['result']
    end
  end
end

# The first line is the initialize notification.
info = nil
first = UmeBoot.read_line&.strip
if first && !first.empty?
  message = JSON.parse(first)
  info = message['params'] if message['method'] == 'initialize'
end

require_relative 'api_gen'
editor = UmeEditorApi.install(UmeBoot.method(:call))
editor.info = info
Object.const_set(:Editor, editor)

user_file = File.expand_path(ARGV.shift)
$0 = user_file
load user_file
