$: << File.expand_path('./parser/test', __dir__)
$: << File.expand_path('./ast/lib', __dir__)

$LOADED_FEATURES << 'simplecov.rb' << 'parser/lexer.rb' << 'minitest/autorun'
module Parser
  class Lexer
    attr_accessor :comments, :diagnostics, :source_buffer, :state, :static_env
    def initialize(*); end
    def reset(*); end
    def advance; end
    def cond; end
  end
end

module Minitest
  def self.after_run(*); end
  class Test
  end
end

require 'test_lexer'
require 'ostruct'

class DummyLex
  def method_missing(*); self; end
  def state=(state); $current_rust_test.lex_state = state.to_s.upcase; end
end

class TestLexer
  def initialize(*)
    @lex = DummyLex.new
  end

  def setup_lexer(*); end

  def assert_scanned(input, *tokens)
    $current_rust_test.input = input
    $current_rust_test.tokens = tokens
    $rust_tests << $current_rust_test
    $current_rust_test = RustTest.new
  end

  def assert_escape(*); end
  def assert_raises(*); end
  def refute_escape(*); end

  # these guys are too complex
  def assert_lex_fname(*); end
  def assert_equal(*); end
  def test_comment(*); end
  def test_comment_begin(*); end
  def test_comment_begin_space(*); end
  def test_comment_end_space_and_text(*); end
  def test_comment_eos(*); end
  def test_bug_hidden_eof_case_5(*); end

  # these guys are wrong in 3.0 moe
  def test_bug_ragel_stack; end
  def test_string_pct_W_interp; end
  def test_string_double_interp_label; end
  def test_or2; end
  def test_label_in_params__18; end
  def test_numbers; end
  def test_integer_oct_O_not_bad_none; end
  def test_bug_expr_endarg_braces; end
  def test_label__18; end
  def test_string_pct_w_backslash_interp_nl; end
  def test_heredoc_empty; end
  def test_ternary; end
  def test_bug_eh_symbol_no_newline; end
  def test_string_double_interp; end
  def test_dot2; end
  def test_bug_interleaved_heredoc; end
  def test_heredoc_double_interp; end
  def test_bug_interp_expr_value; end
  def test_integer_oct_o_not_bad_none; end
  def test_dot3; end
end

class String
  def rustify
    self
      .gsub("\\\#", "#")
      .gsub("\\a", "\\x07")
      .gsub("\\b", "\\x08")
      .gsub("\\e", "\\x1b")
      .gsub("\\f", "\\x0c")
      .gsub("\\v", "\\x0b")
      .gsub(/\\u[0-9a-fA-F]{4}/) { |match| "\\u{" + match[-4..-1] + "}"  }
  end
end

class Token < Struct.new(:name, :value, :range)
  def name=(name)
    super("#{name.inspect}, ")
  end

  def value=(value)
    if value.is_a?(Numeric)
      value = value.to_s
    end

    if value.is_a?(String)
      value = value.dup.force_encoding('utf-8')
      if value.valid_encoding?
        value = value.inspect.rustify
      end
      super("Some(#{value}), ")
    elsif value.nil?
      super("None, ")
    else
      raise "Unknown token value type #{value.inspect}"
    end
  end

  def valid?
    if value.is_a?(String)
      value.valid_encoding?
    else
      true
    end
  end
end

class Tokens
  def initialize(list)
    @list = list
  end

  def to_s
    max_name_length  = @list.map(&:name).map(&:length).max
    max_value_length = @list.map(&:value).map(&:length).max

    @list.map do |t|
      name  = t.name.to_s.ljust(max_name_length, ' ')
      value = t.value.ljust(max_value_length, ' ')
      "#{name}#{value}#{t.range.inspect}"
    end.join(",\n                    ")
  end

  def valid?
    @list.all?(&:valid?)
  end
end

class RustTest < Struct.new(:lex_state, :input, :tokens, :mid)
  IGNORED_MIDS = [
    'test_bug_expr_arg_newline_case_0',
    'test_whitespace_arg_case_2',
    'test_whitespace_end_case_2',
    'test_whitespace_endarg_case_2',
    'test_whitespace_endfn_case_2',
    'test_whitespace_endfn_case_3',
    'test_whitespace_mid_case_2',

    # bugs
    'test_float_pos_case_0', # '+1.0' is a literal, there's no unary plus
  ]

  def mid=(mid)
    super(mid.gsub("__", "_").gsub("__", "_"))
  end

  def input=(input)
    input = input.dup.force_encoding('utf-8')
    if input.valid_encoding?
      input = input.inspect.rustify
    end
    super(input)
  end

  def tokens=(tokens)
    tokens = tokens
      .each_slice(3)
      .to_a
      .map { |(name, value, range)|
        t = Token.new
        t.name = name
        t.value = value
        t.range = range
        t
      }

    super(Tokens.new(tokens))
  end

  def lex_state=(lex_state)
    if lex_state
      super(lex_state.to_s.upcase)
    end
  end

  def valid?
    input.valid_encoding? && tokens.valid? && lex_state != 'LINE_BEGIN' && !IGNORED_MIDS.include?(mid)
  end

  def to_s
    return "// skipping #{mid}" unless valid?

    [
      "#[test]",
      "fn #{mid}() {",
      "    let mut lexer = setup_lexer!();",
     ("    set_lex_state!(lexer, #{lex_state});" if lex_state),
      "    assert_scanned!(&mut lexer,",
      "                    #{input},",
      "                    #{tokens.to_s});",
      "}"
  ].compact.join("\n")
  end
end
$current_rust_test = RustTest.new

ruby_test = TestLexer.new
methods = ruby_test.methods.grep(/\Atest_/).sort
# methods = [:test_colon2]
$recorded_rust_tests = []

methods.each do |mid|
  $rust_tests = []

  ruby_test.send(mid)

  rust_mid = mid.to_s.gsub(/[A-Z]+/) { |match| match.downcase + "_upper" }

  $rust_tests.each_with_index do |rust_test, idx|
    rust_test.mid = "#{rust_mid}_case_#{idx}"
    $recorded_rust_tests << rust_test
  end
end

File.write(
  File.expand_path('../tests/lexer_test.rs', __dir__),
<<-RUST
// THIS FILE IS AUTO-GENERATED BY vendor/codegen.rb

mod assert_scanned;

#{$recorded_rust_tests.map(&:to_s).join("\n\n")}
RUST
)
