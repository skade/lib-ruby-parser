use crate::Lexer;
use crate::lexer::{StringLiteral, Token, TokenType};
use crate::lexer::lex_char::LexChar;
use crate::lexer::lex_states::*;
use crate::lexer::str_types::*;

impl Lexer {
    pub fn parse_string(&mut self, quote: &mut StringLiteral) -> TokenType {
        let func = quote.func;
        let term = quote.term;
        let paren = quote.paren;
        let mut c: LexChar;
        let mut space = false;

        println!("func = {}, pcur = {}, ptok = {}", func, self.p.lex.pcur, self.p.lex.ptok);

        if (func & STR_FUNC_TERM) != 0 {
            if (func & STR_FUNC_QWORDS) != 0 { self.nextc(); } /* delayed term */
            self.set_lex_state(EXPR_END);
            self.p.lex.strterm = None;
            if (func & STR_FUNC_REGEXP) != 0 {
                return Token::tREGEXP_END
            } else {
                return Token::tSTRING_END;
            }
        }
        c = self.nextc();
        if (func & STR_FUNC_QWORDS) != 0 && c.is_space() {
            loop {
                c = self.nextc();

                if !c.is_space() { break }
            }
            space = true;
        }
        if (func & STR_FUNC_LIST) != 0 {
            quote.func &= !STR_FUNC_LIST;
            space = true;
        }
        if c == term && quote.nest == 0 {
            if (func & STR_FUNC_QWORDS) != 0 {
                quote.func &= STR_FUNC_TERM;
                self.pushback(&c); /* dispatch the term at tSTRING_END */
                return Token::tSP;
            }
            return self.parser_string_term(func);
        }
        if space {
            self.pushback(&c);
            return Token::tSP;
        }
        self.newtok();
        if ((func & STR_FUNC_EXPAND) != 0) && c == '#' {
            if let Some(t) = self.parser_peek_variable_name() {
                return t;
            }
            self.tokadd(&LexChar::Some('#'));
            c = self.nextc();
        }
        self.pushback(&c);

        if self.tokadd_string(func, term, paren, &mut quote.nest).is_eof() {
            if self.p.eofp {
                self.literal_flush(self.p.lex.pcur);
                if (func & STR_FUNC_QWORDS) != 0 {
                    /* no content to add, bailing out here */
                    self.yyerror0("unterminated list meets end of file");
                    self.p.lex.strterm = None;
                    return Token::tSTRING_END;
                }
                if (func & STR_FUNC_REGEXP) != 0 {
                    self.yyerror0("unterminated regexp meets end of file");
                } else {
                    self.yyerror0("unterminated string meets end of file");
                }
                quote.func |= STR_FUNC_TERM;
            }
        }

        self.tokfix();
        self.set_yylval_str(&self.tok());
        self.flush_string_content();

        // if let Some(lval) = &self.p.lval {
        //     if lval != "a string" {
        //         panic!("dead");
        //     }
        // }
        Token::tSTRING_CONTENT
    }

    pub fn parser_string_term(&mut self, func: usize) -> TokenType {
        self.p.lex.strterm = None;
        if (func & STR_FUNC_REGEXP) != 0 {
            let regx_options = self.regx_options();
            self.set_yylval_num(&regx_options);
            self.set_lex_state(EXPR_END);
            return Token::tREGEXP_END;
        }
        if (func & STR_FUNC_LABEL) != 0 && self.is_label_suffix(0) {
            self.nextc();
            self.set_lex_state(EXPR_BEG|EXPR_LABEL);
            return Token::tLABEL_END;
        }
        self.set_lex_state(EXPR_END);
        return Token::tSTRING_END;
    }

    pub fn set_yylval_num(&mut self, _num: &str) { unimplemented!("set_yylval_num") }
    pub fn regx_options(&mut self) -> String { unimplemented!("regx_options") }

    pub fn parser_peek_variable_name(&mut self) -> Option<TokenType> {
        let mut c: LexChar;
        let mut ptr: usize = self.p.lex.pcur;

        if ptr + 1 >= self.p.lex.pend { return None }
        c = self.char_at(ptr);
        ptr += 1;

        match c {
            LexChar::Some('$') => {
                c = self.char_at(ptr);
                if c == '-' {
                    ptr += 1;
                    if ptr >= self.p.lex.pend { return None }
                    // c = self.char_at(ptr);
                } else if c.is_global_name_punct() || c.is_digit() {
                    return Some(Token::tSTRING_DVAR);
                }
            },

            LexChar::Some('@') => {
                c = self.char_at(ptr);
                if c == '@' {
                    ptr += 1;
                    if ptr >= self.p.lex.pend { return None }
                    // c = self.char_at(ptr);
                }
            },

            LexChar::Some('{') => {
                self.p.lex.pcur = ptr;
                self.p.command_start = true;
                return Some(Token::tSTRING_DBEG)
            },

            _ => return None
        }

        None
    }

    pub fn tokadd_string(&mut self, func: usize, term: char, paren: Option<char>, nest: &mut usize) -> LexChar {
        let mut c;
        let _erred = false;

        loop {
            c = self.nextc();
            if c.is_eof() { break; }

            if self.p.heredoc_indent > 0 {
                self.parser_update_heredoc_indent(&c);
            }

            if let Some(paren) = paren {
                if c == paren {
                    *nest += 1;
                }
            } else if c == term {
                if *nest == 0 {
                    self.pushback(&c);
                    break;
                }
                *nest -= 1;
            } else if ((func & STR_FUNC_EXPAND) != 0) && c == '#' && self.p.lex.pcur < self.p.lex.pend {
                let c2 = self.char_at(self.p.lex.pcur);
                if c2 == '$' || c2 == '@' || c2 == '{' {
                    self.pushback(&c);
                    break;
                }
            } else if c == '\\' {
                self.literal_flush(self.p.lex.pcur - 1);
                c = self.nextc();
                match c {
                    LexChar::Some('\n') => {
                        if (func & STR_FUNC_QWORDS) != 0 { break }
                        if (func & STR_FUNC_EXPAND) != 0 {
                            if (func & STR_FUNC_INDENT) == 0 || self.p.heredoc_indent < 0 {
                                continue;
                            }
                            if c == term {
                                c = LexChar::Some('\\');
                                return c;
                            }
                        }
                        self.tokadd(&LexChar::Some('\\'));
                        break;
                    },

                    LexChar::Some('\\') => {
                        if (func & STR_FUNC_ESCAPE) != 0 { self.tokadd(&c) }
                        break;
                    },

                    LexChar::Some('u') => {
                        if (func & STR_FUNC_EXPAND) == 0 {
                            self.tokadd(&LexChar::Some('\\'));
                            break;
                        }
                        self.tokadd_utf8(term, func & STR_FUNC_SYMBOL, func & STR_FUNC_REGEXP);
                        continue;
                    },

                    LexChar::EOF => {
                        return LexChar::EOF;
                    },

                    LexChar::Some(_) => {
                        if !c.is_ascii() {
                            if (func & STR_FUNC_EXPAND) == 0 {
                                self.tokadd(&LexChar::Some('\\'));
                                // goto non_ascii (inlined)
                                unimplemented!("non_ascii1");
                            }
                            if (func & STR_FUNC_REGEXP) != 0 {
                                if c == term && !self.is_simple_re_match(&c) {
                                    self.tokadd(&c);
                                    continue;
                                }
                                self.pushback(&c);
                                c = self.tokadd_escape();
                                if c.is_eof() {
                                    return c;
                                }
                                // TODO: compare encodings
                                continue;
                            } else if (func & STR_FUNC_EXPAND) != 0 {
                                self.pushback(&c);
                                if (func & STR_FUNC_ESCAPE) != 0 { self.tokadd(&LexChar::Some('\\')) }
                                c = self.read_escape(0);
                            } else if (func & STR_FUNC_QWORDS) != 0 && c.is_space() {
                                // ignore backslashed spaces in %w
                            } else if c != term && c != paren {
                                self.tokadd(&LexChar::Some('\\'));
                                self.pushback(&c);
                                continue;
                            }
                        }
                    }
                }
            } else if !self.parser_is_ascii() {
                unimplemented!("non_ascii1");
            } else if (func & STR_FUNC_QWORDS) != 0 && c.is_space() {
                self.pushback(&c);
                break;
            }
            if let LexChar::Some(c) = c {
                if (c as u8 & 0x80) != 0 {
                    unimplemented!("part of non_ascii related to encodings");
                }
            }
            self.tokadd(&c);
        }

        c
    }
    pub fn set_yylval_str(&mut self, value: &str) {
        println!("set_yylval_str {}", value);
        self.p.lval = Some(value.into());
    }

    pub fn flush_string_content(&mut self) {
        // noop
    }

    pub fn parser_update_heredoc_indent(&mut self, _c: &LexChar) {
        unimplemented!("parser_update_heredoc_indent")
    }

    pub fn tokadd_utf8(&mut self, _c: char, _func1: usize, _func2: usize) {
        unimplemented!("tokadd_utf8")
    }

    pub fn is_simple_re_match(&mut self, _c: &LexChar) -> bool {
        unimplemented!("is_simple_re_match")
    }

    pub fn tokadd_escape(&mut self) -> LexChar {
        unimplemented!("tokadd_escape")
    }

    pub fn read_escape(&mut self, _x: usize) -> LexChar {
        unimplemented!("read_escape")
    }

    pub fn parser_is_ascii(&self) -> bool {
        self.char_at(self.p.lex.pcur - 1).is_ascii()
    }

}