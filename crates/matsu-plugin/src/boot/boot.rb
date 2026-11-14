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

module MatsuBoot
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
first = MatsuBoot.read_line&.strip
if first && !first.empty?
  message = JSON.parse(first)
  info = message['params'] if message['method'] == 'initialize'
end

require_relative 'api_gen'
editor = MatsuEditorApi.install(MatsuBoot.method(:call))
editor.info = info
Object.const_set(:Editor, editor)

# A plugin host (ADR-0026): `initialize` says kind "plugin", and there is no script. The editor sends
# `plugin/invoke` (the jack, the file of the handler, its text, the arguments) and waits for the
# answer; the process stays for the next call when the plugin is resident. The handler sees `Editor`,
# `EditorError`, `Plugin`, `Outline`, `Complement` and, for the jack SmartIndent, `Indent`, as
# top-level constants (the QuickJS host has the same objects).
module MatsuPlugin
  PLUGIN_CALLS = %w[GetPluginDir GetDef GetOption SetOption GetString GetCommandNo AddCommand].freeze
  OUTLINE_CALLS = %w[AddFuncInfo AddFuncInfo2 AddFuncInfo3 AddFuncInfo4 SetTitle SetListType SetLabel].freeze

  # A trailing nil is a missing argument, not a null.
  def self.trim(args)
    args = args.dup
    args.pop while !args.empty? && args.last.nil?
    args
  end

  # A class whose methods pass their arguments to the editor as the call `prefix + name`.
  def self.calls(prefix, names, &body)
    Class.new do
      names.each do |name|
        define_method(name) { |*args| MatsuBoot.call("#{prefix}#{name}", MatsuPlugin.trim(args)) }
      end
      class_eval(&body) if body
    end
  end

  PluginClass = calls('Plugin.', PLUGIN_CALLS) do
    attr_accessor :id, :dir, :jack, :args
  end
  OutlineClass = calls('Outline.', OUTLINE_CALLS)

  # The word to complete, and the option, come with the call; the candidates go to the editor.
  class ComplementClass
    def initialize(plugin)
      @plugin = plugin
    end

    def GetCurrentWord
      word = @plugin.args.is_a?(Hash) ? @plugin.args['word'] : nil
      word.is_a?(String) ? word : ''
    end

    def GetOption
      option = @plugin.args.is_a?(Hash) ? @plugin.args['option'] : nil
      option.is_a?(Integer) ? option : 0
    end

    def AddList(word = nil)
      MatsuBoot.call('Complement.AddList', word.nil? ? [] : [word.to_s])
    end
  end

  # The object of the jack SmartIndent: GetChar answers the typed char.
  class IndentClass
    def initialize(typed)
      @typed = typed
    end

    def GetChar
      @typed
    end
  end

  def self.respond(id, error)
    message = if error.nil?
                { 'jsonrpc' => '2.0', 'id' => id, 'result' => { 'ok' => true } }
              else
                { 'jsonrpc' => '2.0', 'id' => id, 'error' => { 'code' => -32_000, 'message' => error } }
              end
    $stdout.write("#{MatsuBoot::RS}#{JSON.generate(message, ascii_only: true)}\n")
  end

  # The text of a failure for the editor: the error with the frames of the handler (the frames of
  # this boot are left out).
  def self.describe(exc)
    if exc.is_a?(EditorError)
      # Raised in this boot, so its own first frame is not the handler's.
      frames = (exc.backtrace || []).reject { |line| line.include?(__FILE__) }
      return "EditorError #{exc.code}: #{exc.message}\n#{frames.map { |f| "\tfrom #{f}" }.join("\n")}".scrub.rstrip
    end

    text = exc.full_message(highlight: false, order: :top).scrub
    text.lines.reject { |line| line.include?(__FILE__) }.join.rstrip
  end

  def self.serve(info)
    identity = info.delete('plugin') || {}
    plugin = PluginClass.new
    plugin.id = identity['id']
    plugin.dir = identity['dir']
    plugin.jack = ''
    Object.const_set(:Plugin, plugin)
    Object.const_set(:Outline, OutlineClass.new)
    Object.const_set(:Complement, ComplementClass.new(plugin))
    # The folder of the plugin, so that a handler can `require` its own files.
    $LOAD_PATH.unshift(plugin.dir) if plugin.dir
    # One object for each handler file: `self` of the handler, so that its instance variables and
    # methods stay for the next call of the same file (the plugin is resident), and no other file
    # sees them.
    spaces = {}
    loop do
      text = MatsuBoot.read_line
      break if text.nil?

      text = text.strip
      next if text.empty?

      begin
        message = JSON.parse(text)
      rescue JSON::ParserError
        next
      end
      break if message['method'] == 'plugin/shutdown'
      next unless message['method'] == 'plugin/invoke'

      params = message['params'] || {}
      name = params['name'] || '<plugin>'
      plugin.jack = params['jack'] || ''
      plugin.args = params['args']
      if Object.const_defined?(:Indent)
        Object.send(:remove_const, :Indent)
      end
      if plugin.jack == 'SmartIndent'
        typed = plugin.args.is_a?(Hash) ? plugin.args['char'] : nil
        Object.const_set(:Indent, IndentClass.new(typed.is_a?(String) ? typed : ''))
      end
      failure = nil
      begin
        (spaces[name] ||= Object.new).instance_eval(params['source'].to_s, name, 1)
      rescue SystemExit
        # `exit` in a handler ends the handler, not the host.
        nil
      rescue Exception => e # rubocop:disable Lint/RescueException -- whatever the handler raised fails the call
        failure = describe(e)
      end
      $stdout.flush
      $stderr.flush
      respond(message['id'], failure)
    end
  end
end

if info && info['kind'] == 'plugin'
  MatsuPlugin.serve(info)
  exit 0
end

user_file = File.expand_path(ARGV.shift)
$0 = user_file
load user_file
