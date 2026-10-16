# GENERATED from crates/ume-plugin/src/api.rs (FUNCS). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files

module UmeEditorApi
  UNSET = Object.new.freeze

  # The editor's macro API (the names and the arguments of Sakura's macros).
  class EditorObject
    attr_accessor :info

    def initialize(call)
      @call = call
      @info = nil
    end

    # GetLineCount(reserved: int) -> int
    #
    # The number of logical lines (lines separated by line breaks). The text after the last line
    # break is not a line unless it is not empty. The argument is reserved and must be 0.
    def GetLineCount(reserved)
      @call.call('GetLineCount', _pack(reserved))
    end

    # GetLineStr(line: int) -> string
    #
    # A logical line with its line break ("\n"; the last line has none unless the text ends with
    # one). 0 is the line of the cursor, 1 is the first line. Past the last line it is an empty
    # string.
    def GetLineStr(line)
      @call.call('GetLineStr', _pack(line))
    end

    # InsText(text: string) -> null
    #
    # Inserts the text at the cursor, replacing the selection. The cursor ends after the text.
    def InsText(text)
      @call.call('InsText', _pack(text))
    end

    # AddTail(text: string) -> null
    #
    # Moves to the end of the file, drops the selection, and inserts the text there.
    def AddTail(text)
      @call.call('AddTail', _pack(text))
    end

    # GoFileTop() -> null
    #
    # Moves the cursor to the start of the file and drops the selection.
    def GoFileTop
      @call.call('GoFileTop', _pack())
    end

    # GoFileEnd() -> null
    #
    # Moves the cursor to the end of the file and drops the selection.
    def GoFileEnd
      @call.call('GoFileEnd', _pack())
    end

    # Up() -> null
    #
    # Moves the cursor up one line and drops the selection.
    def Up
      @call.call('Up', _pack())
    end

    # Down() -> null
    #
    # Moves the cursor down one line and drops the selection.
    def Down
      @call.call('Down', _pack())
    end

    # SelectAll() -> null
    #
    # Selects the whole text. The cursor goes to the end.
    def SelectAll
      @call.call('SelectAll', _pack())
    end

    # GetSelectedString(reserved: int) -> string
    #
    # The selected text with its line breaks ("\n"), or an empty string when nothing is
    # selected. The argument is reserved and ignored.
    def GetSelectedString(reserved)
      @call.call('GetSelectedString', _pack(reserved))
    end

    # IsTextSelected() -> int
    #
    # 0 when nothing is selected, 1 when text is selected. (Sakura returns 2 for a box
    # selection, which is not supported yet.)
    def IsTextSelected
      @call.call('IsTextSelected', _pack())
    end

    # MoveCursor(line: int, column: int, option: int) -> null
    #
    # Moves the cursor to a line and a column, both counted from 1. The column counts UTF-16
    # code units. Option: 0 does not select, 1 selects from the old position. (2, the box
    # selection, is not supported.) Past the end of the text it goes to the end of the text.
    def MoveCursor(line, column, option)
      @call.call('MoveCursor', _pack(line, column, option))
    end

    # Jump(line: int, flag: int?) -> null
    #
    # Moves to the start of a line, counted from 1 in logical lines. Lines before 1 and after
    # the last one are limited to the first and the last line. Flag bit 0 (1, the default) is
    # the line break unit, which is the only unit supported yet; bit 1 (PL/SQL) is not
    # supported.
    def Jump(line, flag = UNSET)
      @call.call('Jump', _pack(line, flag))
    end

    # GetFilename() -> string
    #
    # The full path of the file, or an empty string when the text has no file.
    def GetFilename
      @call.call('GetFilename', _pack())
    end

    # Any other name that starts like a Sakura macro name (an upper case letter or a digit) is
    # sent to the editor as it is. The editor answers a name it does not know with an error
    # (EditorError).
    def method_missing(name, *args)
      return super unless name.to_s.match?(/\A[0-9A-Z]/)

      @call.call(name.to_s, args)
    end

    def respond_to_missing?(name, include_private = false)
      name.to_s.match?(/\A[0-9A-Z]/) || super
    end

    private

    # A trailing UNSET is a missing argument, not a nil.
    def _pack(*args)
      args.pop while !args.empty? && args.last.equal?(UNSET)
      args
    end
  end

  def self.install(call)
    EditorObject.new(call)
  end
end
