require 'ripper';
keyword = {
    '__ENCODING__' => :k__ENCODING__,
    '__LINE__' => :k__LINE__,
    '__FILE__' => :k__FILE__,
    'BEGIN' => :klBEGIN,
    'END' => :klEND,
    'alias' => :kALIAS,
    'and' => :kAND,
    'begin' => :kBEGIN,
    'break' => :kBREAK,
    'case' => :kCASE,
    'class' => :kCLASS,
    'def' => :kDEF,
    'defined?' => :kDEFINED,
    'do' => :kDO,
    'else' => :kELSE,
    'elsif' => :kELSIF,
    'end' => :kEND,
    'ensure' => :kENSURE,
    'false' => :kFALSE,
    'for' => :kFOR,
    'if' => :kIF,
    'in' => :kIN,
    'module' => :kMODULE,
    'next' => :kNEXT,
    'nil' => :kNIL,
    'not' => :kNOT,
    'or' => :kOR,
    'redo' => :kREDO,
    'rescue' => :kRESCUE,
    'retry' => :kRETRY,
    'return' => :kRETURN,
    'self' => :kSELF,
    'super' => :kSUPER,
    'then' => :kTHEN,
    'true' => :kTRUE,
    'undef' => :kUNDEF,
    'unless' => :kUNLESS,
    'until' => :kUNTIL,
    'when' => :kWHEN,
    'while' => :kWHILE,
    'yield' => :kYIELD,
}

mapping = {
    on_int: :tINTEGER,
    on_ident: :tIDENTIFIER,
    on_tstring_beg: :tSTRING_BEG,
    on_tstring_content: :tSTRING_CONTENT,
    on_tstring_end: :tSTRING_END,
    on_const: :tCONSTANT,
    on_regexp_beg: :tREGEXP_BEG,
    on_regexp_end: :tREGEXP_END,
    on_comma: :tCOMMA,
    on_heredoc_beg: :tSTRING_BEG,
    on_heredoc_end: :tSTRING_END,
    on_embexpr_beg: :tSTRING_DBEG,
    on_embexpr_end: :tSTRING_DEND,
    on_symbeg: :tSYMBEG,
    on_period: :tDOT,
    on_lbracket: :tLBRACK,
    on_rbracket: :tRBRACK,
    on_semicolon: :tSEMI,
    on_lbrace: :tLBRACE,
    on_rbrace: :tRBRACE,
    on_lparen: :tLPAREN,
    on_rparen: :tRPAREN,
    on_CHAR: :tCHAR,
    on_ivar: :tIVAR,
    on_label: :tLABEL,
    on_backref: :tBACKREF,
    on_qwords_beg: :tQWORDS_BEG,
    on_gvar: :tGVAR,
    on_tlambda: :tLAMBDA,
    on_tlambeg: :tLAMBEG,
    on_float: :tFLOAT,
    on_backtick: :tXSTRING_BEG,
    on_cvar: :tCVAR,
    on_imaginary: :tIMAGINARY,
    on_rational: :tRATIONAL,
    on_qsymbols_beg: :tQSYMBOLS_BEG,
    on_embdoc_beg: :tCOMMENT,
    on_embdoc_end: :tCOMMENT,
    on_embdoc: :tCOMMENT,
    on_words_beg: :tWORDS_BEG,
    on___end__: :t__END__,
    on_embvar: :tSTRING_DVAR,
    on_symbols_beg: :tSYMBOLS_BEG,
    on_nl: :tNL,
}

ops = {
    '='   => :tEQL,     '&'   => :tAMPER2,  '|'   => :tPIPE,
    '!'   => :tBANG,    '^'   => :tCARET,   '+'   => :tPLUS,
    '-'   => :tMINUS,   '*'   => :tSTAR2,   '/'   => :tDIVIDE,
    '%'   => :tPERCENT, '~'   => :tTILDE,   ','   => :tCOMMA,
    ';'   => :tSEMI,    '.'   => :tDOT,     '..'  => :tDOT2,
    '...' => :tDOT3,    '['   => :tLBRACK2, ']'   => :tRBRACK,
    '('   => :tLPAREN2, ')'   => :tRPAREN,  '?'   => :tEH,
    ':'   => :tCOLON,   '&&'  => :tANDOP,   '||'  => :tOROP,
    '-@'  => :tUMINUS,  '+@'  => :tUPLUS,   '~@'  => :tTILDE,
    '**'  => :tPOW,     '->'  => :tLAMBDA,  '=~'  => :tMATCH,
    '!~'  => :tNMATCH,  '=='  => :tEQ,      '!='  => :tNEQ,
    '>'   => :tGT,      '>>'  => :tRSHFT,   '>='  => :tGEQ,
    '<'   => :tLT,      '<<'  => :tLSHFT,   '<='  => :tLEQ,
    '=>'  => :tASSOC,   '::'  => :tCOLON2,  '===' => :tEQQ,
    '<=>' => :tCMP,     '[]'  => :tAREF,    '[]=' => :tASET,
    '{'   => :tLCURLY,  '}'   => :tRCURLY,  '`'   => :tBACK_REF2,
    '!@'  => :tBANG,    '&.'  => :tANDDOT,

    '+='  => :tOP_ASGN, '-='  => :tOP_ASGN, '||=' => :tOP_ASGN,
    '*='  => :tOP_ASGN, '|='  => :tOP_ASGN, '&='  => :tOP_ASGN,
    '>>=' => :tOP_ASGN, '<<=' => :tOP_ASGN, '&&=' => :tOP_ASGN,
    '%='  => :tOP_ASGN, '/='  => :tOP_ASGN, '^='  => :tOP_ASGN,
    '**=' => :tOP_ASGN,
}

strs = []

$stderr.puts ARGV.first
Ripper.lex(File.read(ARGV.first)).each do |(start, tok_name, tok_value, _)|
    tok_name =
        case tok_name
        when :on_kw then keyword.fetch(tok_value) { raise 'unsupported keyword ' + tok_value  }
        when :on_sp, :on_ignored_sp, :on_ignored_nl, :on_comment, :on_words_sep then next
        when :on_op then ops.fetch(tok_value) { raise 'unsupported op ' + tok_value }
        else
            mapping.fetch(tok_name) { raise 'unsupported ' + tok_name.to_s + ' ' + tok_value }
        end

    case tok_name
    when :tSTRING_BEG, :tREGEXP_BEG
        strs.push(tok_value)
    when :tSTRING_END, :tREGEXP_END
        strs.pop
    when :tSTRING_CONTENT
        case strs.last
        when "'" # no escaping
        when "\"", "/"
            tok_value = ("\"" + tok_value + "\"").undump
        else
            raise "unknown str type #{strs.last.inspect}"
        end
    end

    puts tok_name.to_s + ' ' + tok_value.bytes.inspect + ' ' + start.join(':')
end
