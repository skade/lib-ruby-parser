use crate::lex_states::*;
use crate::lexer::*;
use crate::maybe_byte::*;
use crate::source::buffer::*;
use crate::DiagnosticMessage;

pub(crate) trait ParseQmark {
    fn parse_qmark_ternary(&mut self, c: &MaybeByte) -> Result<i32, ()>;
    fn parse_qmark(&mut self, space_seen: bool) -> Result<i32, ()>;
}

impl ParseQmark for Lexer {
    fn parse_qmark_ternary(&mut self, c: &MaybeByte) -> Result<i32, ()> {
        self.buffer.pushback(c);
        self.lex_state.set(EXPR_VALUE);
        Ok(Self::tEH)
    }

    fn parse_qmark(&mut self, space_seen: bool) -> Result<i32, ()> {
        // let enc;
        let mut c;

        if self.lex_state.is_end() {
            self.lex_state.set(EXPR_VALUE);
            return Ok(Self::tEH);
        }
        c = self.nextc();
        if c.is_eof() {
            self.compile_error(
                DiagnosticMessage::IncompleteCharacterSyntax,
                self.current_range(),
            );
            return Ok(Self::END_OF_INPUT);
        }
        if c.is_space() {
            if !self.lex_state.is_arg() {
                if let Some(c2) = c.escaped_control_code() {
                    self.warn_space_char(c2, "?");
                }
            }
            return self.parse_qmark_ternary(&c);
        }
        self.newtok();

        if !self.is_ascii() {
            if self.tokadd_mbchar(&c).is_err() {
                return Ok(Self::END_OF_INPUT);
            }
        } else if (c.is_alnum() || c == b'_')
            && self.buffer.pcur < self.buffer.pend
            && self.buffer.is_identchar(self.buffer.pcur, self.buffer.pend)
        {
            if space_seen {
                let start = self.buffer.pcur - 1;
                let mut ptr = start;
                loop {
                    if let Some(n) = self.multibyte_char_len(ptr) {
                        ptr += n;
                    } else {
                        return Err(());
                    }

                    if !(ptr < self.buffer.pend && self.buffer.is_identchar(ptr, self.buffer.pend))
                    {
                        break;
                    }
                }
                self.warn(
                    DiagnosticMessage::AmbiguousTernaryOperator(
                        String::from_utf8_lossy(self.buffer.substr_at(start, ptr).unwrap())
                            .into_owned(),
                    ),
                    self.range(start - 1, start),
                )
            }
            return self.parse_qmark_ternary(&c);
        } else if c == b'\\' {
            if self.buffer.peek(b'u') {
                self.nextc();
                self.tokadd_utf8(None, 0, 0);
            } else if !self.buffer.is_eol() && !self.char_at(self.buffer.pcur).is_ascii() {
                c = self.char_at(self.buffer.pcur);
                self.nextc();
                if self.tokadd_mbchar(&c).is_err() {
                    return Ok(Self::END_OF_INPUT);
                }
            } else {
                let byte = self.read_escape(0);
                self.tokadd(&byte);
            }
        } else {
            self.tokadd(&c);
        }
        self.tokfix();
        let yylval = self.tokenbuf.take();
        self.set_yylval_str(&yylval);
        self.lex_state.set(EXPR_END);
        Ok(Self::tCHAR)
    }
}
