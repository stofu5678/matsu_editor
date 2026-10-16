# The Ruby part of the mruby engine (src/mruby.rs). It runs before the user's script, as a file
# called "ume-script boot". The functions of `Editor` are native (crates/ume-mruby-sys/glue.c);
# `Editor.info` is set from Rust.

# The editor answered a call with an error (a JSON-RPC error object). The message names the
# function and the argument, and the backtrace of the exception shows the line of the call.
class EditorError < StandardError
  attr_reader :code

  def initialize(code, message)
    super(message)
    @code = code
  end
end

module Editor
  class << self
    attr_reader :info
  end
end

module UmeBoot
  BOOT_FILE = 'ume-script boot'

  # The text `puts` writes for its arguments: each on a line, an array flattened.
  def self.lines(args, out = '')
    args.each do |arg|
      if arg.is_a?(Array)
        lines(arg, out)
      else
        text = arg.to_s
        out << text
        out << "\n" unless text.end_with?("\n")
      end
    end
    out
  end

  # What `$stdout` and `$stderr` are: an object with the usual writing methods.
  # `UmeBoot.write_stdout` and `UmeBoot.write_stderr` are native (glue.c).
  class Stream
    def initialize(error)
      @error = error
    end

    def write(*args)
      text = args.map(&:to_s).join
      @error ? UmeBoot.write_stderr(text) : UmeBoot.write_stdout(text)
      text.bytesize
    end

    def print(*args)
      write(*args)
      nil
    end

    def <<(arg)
      write(arg)
      self
    end

    def puts(*args)
      write(args.empty? ? "\n" : UmeBoot.lines(args))
      nil
    end

    def printf(fmt, *args)
      write(format(fmt, *args))
      nil
    end

    def flush
      self
    end

    def sync
      true
    end

    def sync=(value)
      value
    end

    def tty?
      false
    end
    alias isatty tty?
  end

  # The text that goes to the standard error when `exception` ends the script, in the form of
  # the `ruby` command: "name:3: message (ErrorClass)".
  def self.describe(exception, name)
    message = exception.message.to_s
    class_name = exception.class.to_s

    # A syntax error has no backtrace; the parser's message starts with "line 3: ".
    if exception.is_a?(SyntaxError) && message.start_with?('line ')
      rest = message[5..-1]
      colon = rest.index(': ')
      return "#{name}:#{rest[0, colon]}: #{rest[(colon + 2)..-1]} (#{class_name})" if colon
    end

    # The first place of the backtrace that is in the user's script.
    place = nil
    (exception.backtrace || []).each do |line|
      next if line.start_with?(BOOT_FILE)

      place = line
      break
    end
    if place
      stop = place.index(':in ')
      place = place[0, stop] if stop
      "#{place}: #{message} (#{class_name})"
    else
      "#{name}: #{message} (#{class_name})"
    end
  end
end

# mruby has these in its io gem, which is not bundled (a macro has no use for files).
module Kernel
  def puts(*args)
    print(args.empty? ? "\n" : UmeBoot.lines(args))
    nil
  end

  def printf(fmt, *args)
    print(format(fmt, *args))
    nil
  end

  def pp(*args)
    args.each { |arg| p(arg) }
    args.size <= 1 ? args.first : args
  end

  def warn(*args)
    UmeBoot.write_stderr(args.empty? ? '' : UmeBoot.lines(args))
    nil
  end
end

$stdout = UmeBoot::Stream.new(false)
$stderr = UmeBoot::Stream.new(true)
STDOUT = $stdout
STDERR = $stderr
