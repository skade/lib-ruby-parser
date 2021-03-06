use crate::maybe_byte::*;
use crate::source::SourceLine;
use crate::source::{decode_input, CustomDecoder, InputError};
use std::convert::TryFrom;

#[derive(Debug, Default)]
pub struct Input {
    pub name: String,
    pub bytes: Vec<u8>,
    pub lines: Vec<SourceLine>,
}

impl Input {
    pub(crate) fn set_bytes(&mut self, bytes: Vec<u8>) {
        let mut line = SourceLine {
            start: 0,
            end: 0,
            ends_with_eof: true,
        };
        let mut lines: Vec<SourceLine> = vec![];

        for (idx, c) in bytes.iter().enumerate() {
            line.end = idx + 1;
            if *c == b'\n' {
                line.ends_with_eof = false;
                lines.push(line);
                line = SourceLine {
                    start: idx + 1,
                    end: 0,
                    ends_with_eof: true,
                }
            }
        }
        line.end = bytes.len();
        line.ends_with_eof = true;
        lines.push(line);

        self.bytes = bytes;
        self.lines = lines;
    }

    pub(crate) fn byte_at(&self, idx: usize) -> Option<u8> {
        if let Some(c) = self.bytes.get(idx) {
            Some(*c)
        } else {
            None
        }
    }

    pub(crate) fn substr_at(&self, start: usize, end: usize) -> Option<&[u8]> {
        if start <= end && end <= self.bytes.len() {
            Some(&self.bytes[start..end])
        } else {
            None
        }
    }

    pub fn line_col_for_pos(&self, mut pos: usize) -> Option<(usize, usize)> {
        if pos == self.len() {
            // EOF loc
            let last_line = self.lines.last()?;
            return Some((self.lines.len() - 1, last_line.len()));
        }

        for (lineno, line) in self.lines.iter().enumerate() {
            if line.len() > pos {
                return Some((lineno, pos));
            } else {
                pos -= line.len()
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    // pub fn take_bytes
}

impl Clone for Input {
    fn clone(&self) -> Self {
        println!("Cloning input");
        Self {
            name: self.name.clone(),
            bytes: self.bytes.clone(),
            lines: self.lines.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Buffer {
    pub input: Input,

    pub(crate) line_count: usize,
    pub(crate) prevline: Option<usize>, // index
    pub(crate) lastline: usize,         // index
    pub(crate) nextline: usize,         // index
    pub(crate) pbeg: usize,
    pub(crate) pcur: usize,
    pub(crate) pend: usize,
    pub(crate) ptok: usize,

    pub(crate) eofp: bool,
    pub(crate) cr_seen: bool,

    pub(crate) heredoc_end: usize,
    pub(crate) heredoc_indent: i32,
    pub(crate) heredoc_line_indent: i32,

    pub(crate) tokidx: usize,
    pub(crate) toksize: usize,
    pub(crate) tokline: usize,

    pub(crate) has_shebang: bool,

    pub(crate) ruby_sourceline: usize,     /* current line no. */
    pub(crate) ruby_sourcefile: Vec<char>, /* current source file */
    pub(crate) ruby_sourcefile_string: Vec<char>,

    pub(crate) debug: bool,
    pub(crate) decoder: CustomDecoder,
}

impl Buffer {
    const CTRL_Z_CHAR: char = 0x1a as char;
    const CTRL_D_CHAR: char = 0x04 as char;

    pub fn new(name: &str, bytes: Vec<u8>, decoder: CustomDecoder) -> Self {
        let mut input = Input {
            name: name.to_owned(),
            ..Default::default()
        };

        input.set_bytes(bytes);

        let mut this = Self {
            input,
            decoder,
            ..Self::default()
        };

        this.prepare();

        this
    }

    fn prepare(&mut self) {
        let c = self.nextc();
        match c.to_option() {
            Some(b'#') => {
                if self.peek(b'!') {
                    self.has_shebang = true;
                }
            }
            Some(0xef) => {
                // handle UTF-8 BOM marker
                if self.pend - self.pcur >= 2
                    && self.byte_at(self.pcur) == 0xbb
                    && self.byte_at(self.pcur + 1) == 0xbf
                {
                    self.pcur += 2;
                    self.pbeg = self.pcur;
                    return;
                }
            }
            None => return,
            _ => {}
        }

        self.pushback(&c)
    }

    pub(crate) fn nextc(&mut self) -> MaybeByte {
        if self.pcur == self.pend || self.eofp || self.nextline != 0 {
            let n = self.nextline();
            if self.debug {
                println!("nextline = {:?}", n);
            }
            if n.is_err() {
                return MaybeByte::EndOfInput;
            }
        }
        let mut c = match self.input.bytes.get(self.pcur) {
            Some(c) => *c,
            None => return MaybeByte::EndOfInput,
        };
        self.pcur += 1;
        if c == b'\r' {
            c = self.parser_cr(c);
        }
        if self.debug {
            println!("nextc = {:?}", c);
        }
        MaybeByte::new(c)
    }

    pub(crate) fn goto_eol(&mut self) {
        self.pcur = self.pend;
    }

    pub(crate) fn is_eol(&self) -> bool {
        self.pcur >= self.pend
    }

    pub(crate) fn is_eol_n(&self, n: usize) -> bool {
        self.pcur + n >= self.pend
    }

    pub(crate) fn peek(&self, c: u8) -> bool {
        self.peek_n(c, 0)
    }
    pub(crate) fn peek_n(&self, c: u8, n: usize) -> bool {
        !self.is_eol_n(n) && c == self.input.bytes[self.pcur + n]
    }
    pub(crate) fn peekc_n(&self, n: usize) -> MaybeByte {
        if self.is_eol_n(n) {
            MaybeByte::EndOfInput
        } else {
            self.byte_at(self.pcur + n)
        }
    }

    pub(crate) fn nextline(&mut self) -> Result<(), ()> {
        let mut v = self.nextline;
        self.nextline = 0;

        if v == 0 {
            if self.eofp {
                return Err(());
            }

            if self.pend > self.pbeg && self.input.bytes[self.pend - 1] != b'\n' {
                self.eofp = true;
                self.goto_eol();
                return Err(());
            }

            match self.getline() {
                Ok(line) => v = line,
                Err(_) => {
                    self.eofp = true;
                    self.goto_eol();
                    return Err(());
                }
            }

            self.cr_seen = false;
        }
        // TODO: after here-document without terminator

        let line = &self.input.lines[v];

        if self.heredoc_end > 0 {
            self.ruby_sourceline = self.heredoc_end;
            self.heredoc_end = 0;
        }
        self.ruby_sourceline += 1;
        self.pbeg = line.start;
        self.pcur = line.start;
        self.pend = line.end;
        self.token_flush();
        self.prevline = Some(self.lastline);
        self.lastline = v;

        Ok(())
    }

    pub(crate) fn getline(&mut self) -> Result<usize, ()> {
        if self.line_count < self.input.lines.len() {
            self.line_count += 1;
            if self.debug {
                println!("line_count = {}", self.line_count)
            }
            Ok(self.line_count - 1)
        } else {
            Err(())
        }
    }

    pub(crate) fn token_flush(&mut self) {
        self.set_ptok(self.pcur);
    }

    pub(crate) fn set_ptok(&mut self, ptok: usize) {
        if self.debug {
            println!("set_ptok({})", ptok);
        }
        self.ptok = ptok;
    }

    pub(crate) fn parser_cr(&mut self, mut c: u8) -> u8 {
        if self.peek(b'\n') {
            self.pcur += 1;
            c = b'\n';
        }
        c
    }

    pub(crate) fn byte_at(&self, idx: usize) -> MaybeByte {
        match self.input.byte_at(idx) {
            Some(byte) => MaybeByte::Some(byte),
            None => MaybeByte::EndOfInput,
        }
    }

    pub(crate) fn substr_at(&self, start: usize, end: usize) -> Option<&[u8]> {
        self.input.substr_at(start, end)
    }

    pub(crate) fn was_bol(&self) -> bool {
        self.pcur == self.pbeg + 1
    }

    pub(crate) fn is_word_match(&self, word: &str) -> bool {
        let len = word.len();

        if self.substr_at(self.pcur, self.pcur + len) != Some(word.as_bytes()) {
            return false;
        }
        if self.pcur + len == self.pend {
            return true;
        }
        let c = self.byte_at(self.pcur + len);
        if c.is_space() {
            return true;
        }
        if c == b'\0' || c == Self::CTRL_Z_CHAR || c == Self::CTRL_D_CHAR {
            return true;
        }
        false
    }

    pub(crate) fn is_looking_at_eol(&self) -> bool {
        let mut ptr = self.pcur;
        while ptr < self.pend {
            let c = self.input.bytes.get(ptr);
            ptr += 1;
            if let Some(c) = c {
                let eol = *c == b'\n' || *c == b'#';
                if eol || !c.is_ascii_whitespace() {
                    return eol;
                }
            };
        }
        true
    }

    pub(crate) fn is_whole_match(&self, eos: &[u8], indent: usize) -> bool {
        let mut ptr = self.pbeg;
        let len = eos.len();

        if indent > 0 {
            while let Some(c) = self.input.bytes.get(ptr) {
                if !c.is_ascii_whitespace() {
                    break;
                }
                ptr += 1;
            }
        }

        if self.pend < ptr + len {
            return false;
        }

        if let Ok(n) = isize::try_from(self.pend - (ptr + len)) {
            if n < 0 {
                return false;
            }
            let last_char = self.byte_at(ptr + len);
            let char_after_last_char = self.byte_at(ptr + len + 1);

            if n > 0 && last_char != b'\n' {
                if last_char != b'\r' {
                    return false;
                }
                if n <= 1 || char_after_last_char != b'\n' {
                    return false;
                }
            }

            let next_len_chars = self.substr_at(ptr, ptr + len);
            Some(eos) == next_len_chars
        } else {
            false
        }
    }

    pub(crate) fn eof_no_decrement(&mut self) {
        if let Some(prevline) = self.prevline {
            if !self.eofp {
                self.lastline = prevline;
            }
        }
        self.pbeg = self.input.lines[self.lastline].start;
        self.pend = self.pbeg + self.input.lines[self.lastline].len();
        self.pcur = self.pend;
        self.pushback(&MaybeByte::new(1));
        self.set_ptok(self.pcur);
    }

    pub(crate) fn is_identchar(&self, begin: usize, _end: usize) -> bool {
        self.input.bytes[begin].is_ascii_alphanumeric()
            || self.input.bytes[begin] == b'_'
            || !self.input.bytes[begin].is_ascii()
    }

    pub(crate) fn set_encoding(&mut self, encoding: &str) -> Result<(), InputError> {
        let new_input = decode_input(&self.input.bytes, encoding, &self.decoder)?;
        self.input.set_bytes(new_input);
        Ok(())
    }
}

pub(crate) trait Pushback<T> {
    fn pushback(&mut self, c: &T);
}

impl Pushback<Option<u8>> for Buffer {
    fn pushback(&mut self, c: &Option<u8>) {
        if c.is_none() {
            return;
        };
        self.pcur -= 1;
        if self.pcur > self.pbeg
            && self.byte_at(self.pcur) == b'\n'
            && self.byte_at(self.pcur - 1) == b'\r'
        {
            self.pcur -= 1;
        }
        if self.debug {
            println!("pushback({:?}) pcur = {}", c, self.pcur);
        }
    }
}

impl Pushback<MaybeByte> for Buffer {
    fn pushback(&mut self, c: &MaybeByte) {
        self.pushback(&c.to_option())
    }
}

impl Pushback<char> for Buffer {
    fn pushback(&mut self, _c: &char) {
        self.pushback(&Some(1))
    }
}
