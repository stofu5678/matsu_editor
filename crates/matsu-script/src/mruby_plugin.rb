# The serving loop of a plugin in mruby (src/mruby.rs). It runs after the boot, as the main script of
# the host: it asks the host for each `plugin/invoke` through the reserved calls (src/plugin.rs) and
# runs the text of the handler in an object of its own for each handler file, kept for the next call
# of the same file (ADR-0026), as the boot of the external Ruby does. The handler sees `Editor`,
# `EditorError`, `Plugin`, `Outline`, `Complement` and, for the jack SmartIndent, `Indent`.
#
# The calls of the plugin objects are native functions of `Editor` under their full names
# (`Plugin.GetDef`), so they are called with `__send__`.
module MatsuPlugin
  # An object with one method for each of `names`, which asks the editor for `prefix.name`.
  def self.calls(object, prefix, names)
    names.each do |name|
      full = "#{prefix}.#{name}"
      object.define_singleton_method(name.to_sym) { |*args| Editor.__send__(full, *args) }
    end
    object
  end

  # The `Plugin` object of a plugin (Sakura's CPluginIfObj): the editor answers the calls.
  class PluginClass
    attr_accessor :id, :dir, :jack, :args
  end

  # The `Complement` object of the completion jacks (Sakura's CComplementIfObj). The word and the
  # option come with the call; the candidates go to the editor.
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

    def AddList(*args)
      Editor.__send__('Complement.AddList', *args.first(1).map(&:to_s))
    end
  end

  # The `Indent` object of the jack SmartIndent: GetChar answers the typed char.
  class IndentClass
    def initialize(typed)
      @typed = typed
    end

    def GetChar
      @typed
    end
  end

  # Runs the handler in a method of its own. A string `instance_eval` sees the local variables of its
  # caller, so in `serve` a handler that assigns `invoke` or `plugin` would overwrite the host's.
  # This method has no local variables: `serve` leaves the values in instance variables of this
  # module. `return` ends only the handler.
  def self.evaluate
    @space.instance_eval(@source, @name, 1)
  end

  def self.serve
    identity = Editor.__send__('plugin/identity')
    members = identity['members']
    plugin = calls(PluginClass.new, 'Plugin', members['Plugin'])
    plugin.id = identity['id']
    plugin.dir = identity['dir']
    plugin.jack = ''
    Object.const_set(:Plugin, plugin)
    Object.const_set(:Outline, calls(Object.new, 'Outline', members['Outline']))
    Object.const_set(:Complement, ComplementClass.new(plugin))
    spaces = {}
    loop do
      invoke = Editor.__send__('plugin/next')
      break if invoke.nil?

      # The token that proves this loop is the one that answers the call. It is a local of this
      # block only (no constant, no instance variable, not in the method that evaluates the
      # handler), and an integer: an immediate value, not an object that a handler could find by
      # walking the heap (ObjectSpace).
      token = invoke.delete('token')
      name = invoke['name'].to_s
      name = '<plugin>' if name.empty?
      plugin.jack = invoke['jack'].to_s
      plugin.args = invoke['args']
      Object.__send__(:remove_const, :Indent) if Object.const_defined?(:Indent)
      if plugin.jack == 'SmartIndent'
        typed = plugin.args.is_a?(Hash) ? plugin.args['char'] : nil
        Object.const_set(:Indent, IndentClass.new(typed.is_a?(String) ? typed : ''))
      end
      failure = nil
      begin
        @space = spaces[name] ||= Object.new
        # A BOM starts a UTF-8 file but is not Ruby.
        @source = invoke['source'].to_s
        @source = @source[1..-1] if @source.start_with?("\xEF\xBB\xBF")
        @name = name
        evaluate
      rescue Exception => e # whatever the handler raised fails the call
        failure = MatsuBoot.describe(e, name)
      end
      begin
        Editor.__send__('plugin/respond', invoke['id'], failure, token)
      rescue EditorError
        # Refused: the host does not take the answer (it has answered already). That is no reason
        # to stop serving.
        nil
      end
      token = nil
    end
  end
end

MatsuPlugin.serve
