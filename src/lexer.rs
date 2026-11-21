//! Tokenizer for Puddi source text.

use crate::diag::{Diag, Span};

#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Lit(String),
    Expr(Vec<Token>, Span),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(u128),
    Float(f64),
    Char(char),
    Str(Vec<StrPart>),
    // keywords
    As,
    Async,
    Await,
    Break,
    Const,
    Continue,
    Dyn,
    Else,
    Enum,
    Export,
    False,
    Fn,
    For,
    If,
    Impl,
    Import,
    In,
    Let,
    Loop,
    Match,
    Pub,
    Return,
    SelfLower,
    SelfUpper,
    Struct,
    Test,
    Trait,
    True,
    Type,
    Var,
    While,
    // punctuation
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Dot,
    DotDot,
    DotDotEq,
    Colon,
    Semi,
    Arrow,
    At,
    Question,
    Pipe,
    PipePipe,
    Amp,
    AmpAmp,
    Caret,
    Bang,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Shl,
    Assign,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    Underscore,
    Newline,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

fn keyword(s: &str) -> Option<Tok> {
    Some(match s {
        "as" => Tok::As,
        "async" => Tok::Async,
        "await" => Tok::Await,
        "break" => Tok::Break,
        "const" => Tok::Const,
        "continue" => Tok::Continue,
        "dyn" => Tok::Dyn,
        "else" => Tok::Else,
        "enum" => Tok::Enum,
        "export" => Tok::Export,
        "false" => Tok::False,
        "fn" => Tok::Fn,
        "for" => Tok::For,
        "if" => Tok::If,
        "impl" => Tok::Impl,
        "import" => Tok::Import,
        "in" => Tok::In,
        "let" => Tok::Let,
        "loop" => Tok::Loop,
        "match" => Tok::Match,
        "pub" => Tok::Pub,
        "return" => Tok::Return,
        "self" => Tok::SelfLower,
        "Self" => Tok::SelfUpper,
        "struct" => Tok::Struct,
        "test" => Tok::Test,
        "trait" => Tok::Trait,
        "true" => Tok::True,
        "type" => Tok::Type,
        "var" => Tok::Var,
        "while" => Tok::While,
        "_" => Tok::Underscore,
        _ => return None,
    })
}

pub fn tok_name(t: &Tok) -> String {
    use Tok::*;
    match t {
        Ident(s) => format!("identifier `{s}`"),
        Int(_) => "integer literal".into(),
        Float(_) => "float literal".into(),
        Char(_) => "character literal".into(),
        Str(_) => "string literal".into(),
        Newline => "newline".into(),
        Eof => "end of file".into(),
        other => format!("`{}`", tok_text(other)),
    }
}

pub fn tok_text(t: &Tok) -> &'static str {
    use Tok::*;
    match t {
        As => "as",
        Async => "async",
        Await => "await",
        Break => "break",
        Const => "const",
        Continue => "continue",
        Dyn => "dyn",
        Else => "else",
        Enum => "enum",
        Export => "export",
        False => "false",
        Fn => "fn",
        For => "for",
        If => "if",
        Impl => "impl",
        Import => "import",
        In => "in",
        Let => "let",
        Loop => "loop",
        Match => "match",
        Pub => "pub",
        Return => "return",
        SelfLower => "self",
        SelfUpper => "Self",
        Struct => "struct",
        Test => "test",
        Trait => "trait",
        True => "true",
        Type => "type",
        Var => "var",
        While => "while",
        LParen => "(",
        RParen => ")",
        LBracket => "[",
        RBracket => "]",
        LBrace => "{",
        RBrace => "}",
        Comma => ",",
        Dot => ".",
        DotDot => "..",
        DotDotEq => "..=",
        Colon => ":",
        Semi => ";",
        Arrow => "->",
        At => "@",
        Question => "?",
        Pipe => "|",
        PipePipe => "||",
        Amp => "&",
        AmpAmp => "&&",
        Caret => "^",
        Bang => "!",
        Plus => "+",
        Minus => "-",
        Star => "*",
        Slash => "/",
        Percent => "%",
        Shl => "<<",
        Assign => "=",
        EqEq => "==",
        Ne => "!=",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        PlusEq => "+=",
        MinusEq => "-=",
        StarEq => "*=",
        SlashEq => "/=",
        PercentEq => "%=",
        AmpEq => "&=",
        PipeEq => "|=",
        CaretEq => "^=",
        ShlEq => "<<=",
        Underscore => "_",
        _ => "?",
    }
}

struct Lexer<'a> {
    src: &'a [u8],
    text: &'a str,
    pos: usize,
    file: u32,
    out: Vec<Token>,
    /// Delimiter stack: b'(' b'[' b'{'. Newlines are significant only when the
    /// top is `{` or the stack is empty.
    stack: Vec<u8>,
    /// When lexing an interpolation, stop at the `}` that closes it.
    interp: bool,
}

pub fn lex(text: &str, file: u32) -> Result<Vec<Token>, Diag> {
    let mut lx = Lexer { src: text.as_bytes(), text, pos: 0, file, out: Vec::new(), stack: Vec::new(), interp: false };
    lx.run()?;
    let end = text.len() as u32;
    lx.push_newline(end);
    lx.out.push(Token { tok: Tok::Eof, span: Span::new(file, end, end) });
    Ok(lx.out)
}

/// Lexes `text[lo..hi]` as if it were part of lexing the whole text from a top-level item
/// boundary at `lo`; spans are offsets into `text`. Returns the tokens without a final
/// newline or end-of-file token, and whether lexing ended outside every delimiter.
pub fn lex_range(text: &str, file: u32, lo: usize, hi: usize) -> Result<(Vec<Token>, bool), Diag> {
    let mut lx = Lexer { src: text[..hi].as_bytes(), text: &text[..hi], pos: lo, file, out: Vec::new(), stack: Vec::new(), interp: false };
    lx.run()?;
    Ok((lx.out, lx.stack.is_empty()))
}

impl<'a> Lexer<'a> {
    fn span(&self, lo: usize) -> Span {
        Span::new(self.file, lo as u32, self.pos as u32)
    }

    fn err(&self, lo: usize, msg: impl Into<String>) -> Diag {
        Diag::new(Span::new(self.file, lo as u32, (self.pos.max(lo + 1)) as u32), msg)
    }

    fn peek(&self) -> u8 {
        *self.src.get(self.pos).unwrap_or(&0)
    }

    fn peek_at(&self, n: usize) -> u8 {
        *self.src.get(self.pos + n).unwrap_or(&0)
    }

    fn newline_significant(&self) -> bool {
        matches!(self.stack.last(), None | Some(b'{'))
    }

    fn push_newline(&mut self, at: u32) {
        if self.interp {
            return;
        }
        match self.out.last() {
            None => {}
            Some(t) if t.tok == Tok::Newline => {}
            Some(t) if t.tok == Tok::LBrace => {}
            _ => self.out.push(Token { tok: Tok::Newline, span: Span::new(self.file, at, at) }),
        }
    }

    fn push(&mut self, tok: Tok, lo: usize) {
        let span = self.span(lo);
        self.out.push(Token { tok, span });
    }

    fn run(&mut self) -> Result<(), Diag> {
        loop {
            // skip whitespace and comments
            loop {
                let c = self.peek();
                if c == b' ' || c == b'\t' || c == b'\r' {
                    self.pos += 1;
                } else if c == b'\n' {
                    if self.newline_significant() {
                        self.push_newline(self.pos as u32);
                    }
                    self.pos += 1;
                } else if c == b'/' && self.peek_at(1) == b'/' {
                    while self.pos < self.src.len() && self.peek() != b'\n' {
                        self.pos += 1;
                    }
                } else if c == b'/' && self.peek_at(1) == b'*' {
                    let lo = self.pos;
                    self.pos += 2;
                    let mut depth = 1;
                    while depth > 0 {
                        if self.pos >= self.src.len() {
                            return Err(self.err(lo, "unterminated block comment"));
                        }
                        if self.peek() == b'/' && self.peek_at(1) == b'*' {
                            depth += 1;
                            self.pos += 2;
                        } else if self.peek() == b'*' && self.peek_at(1) == b'/' {
                            depth -= 1;
                            self.pos += 2;
                        } else {
                            if self.peek() == b'\n' && self.newline_significant() {
                                // a block comment spanning lines acts like a newline
                                self.push_newline(self.pos as u32);
                            }
                            self.pos += 1;
                        }
                    }
                } else {
                    break;
                }
            }
            if self.pos >= self.src.len() {
                if self.interp {
                    return Err(self.err(self.pos.saturating_sub(1), "unterminated string interpolation"));
                }
                return Ok(());
            }
            let lo = self.pos;
            let c = self.peek();
            if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 {
                self.ident(lo)?;
                continue;
            }
            if c.is_ascii_digit() {
                self.number(lo)?;
                continue;
            }
            match c {
                b'"' => {
                    self.pos += 1;
                    let parts = self.string(lo)?;
                    self.push(Tok::Str(parts), lo);
                }
                b'\'' => {
                    self.pos += 1;
                    let ch = if self.peek() == b'\\' {
                        self.escape(lo)?
                    } else {
                        let rest = &self.text[self.pos..];
                        let ch = rest.chars().next().ok_or_else(|| self.err(lo, "unterminated character literal"))?;
                        if ch == '\'' || ch == '\n' {
                            return Err(self.err(lo, "empty character literal"));
                        }
                        self.pos += ch.len_utf8();
                        ch
                    };
                    if self.peek() != b'\'' {
                        return Err(self.err(lo, "unterminated character literal"));
                    }
                    self.pos += 1;
                    self.push(Tok::Char(ch), lo);
                }
                b'(' => {
                    self.pos += 1;
                    self.stack.push(b'(');
                    self.push(Tok::LParen, lo);
                }
                b'[' => {
                    self.pos += 1;
                    self.stack.push(b'[');
                    self.push(Tok::LBracket, lo);
                }
                b'{' => {
                    self.pos += 1;
                    self.stack.push(b'{');
                    self.push(Tok::LBrace, lo);
                }
                b')' | b']' | b'}' => {
                    let open = match c {
                        b')' => b'(',
                        b']' => b'[',
                        _ => b'{',
                    };
                    if c == b'}' && self.interp && self.stack.is_empty() {
                        // end of interpolation; caller consumes the brace
                        return Ok(());
                    }
                    match self.stack.pop() {
                        Some(o) if o == open => {}
                        Some(o) => {
                            self.pos += 1;
                            let expected = match o {
                                b'(' => ")",
                                b'[' => "]",
                                _ => "}",
                            };
                            return Err(self.err(lo, format!("mismatched delimiter: expected `{expected}`, found `{}`", c as char)));
                        }
                        None => {
                            self.pos += 1;
                            return Err(self.err(lo, format!("unmatched closing delimiter `{}`", c as char)));
                        }
                    }
                    self.pos += 1;
                    if c == b'}' && self.newline_significant() {
                        // allow `}` to end a statement even without newline handling
                    }
                    let t = match c {
                        b')' => Tok::RParen,
                        b']' => Tok::RBracket,
                        _ => Tok::RBrace,
                    };
                    if t == Tok::RBrace {
                        // a newline right before `}` is redundant
                        if self.out.last().map(|t| t.tok == Tok::Newline).unwrap_or(false) {
                            self.out.pop();
                        }
                    }
                    self.push(t, lo);
                }
                _ => {
                    let (tok, len) = self.punct()?;
                    self.pos += len;
                    self.push(tok, lo);
                }
            }
        }
    }

    fn punct(&self) -> Result<(Tok, usize), Diag> {
        let c = self.peek();
        let n = self.peek_at(1);
        let n2 = self.peek_at(2);
        use Tok::*;
        Ok(match (c, n, n2) {
            (b'.', b'.', b'=') => (DotDotEq, 3),
            (b'.', b'.', _) => (DotDot, 2),
            (b'.', _, _) => (Dot, 1),
            (b',', _, _) => (Comma, 1),
            (b':', _, _) => (Colon, 1),
            (b';', _, _) => (Semi, 1),
            (b'-', b'>', _) => (Arrow, 2),
            (b'-', b'=', _) => (MinusEq, 2),
            (b'-', _, _) => (Minus, 1),
            (b'@', _, _) => (At, 1),
            (b'?', _, _) => (Question, 1),
            (b'|', b'|', _) => (PipePipe, 2),
            (b'|', b'=', _) => (PipeEq, 2),
            (b'|', _, _) => (Pipe, 1),
            (b'&', b'&', _) => (AmpAmp, 2),
            (b'&', b'=', _) => (AmpEq, 2),
            (b'&', _, _) => (Amp, 1),
            (b'^', b'=', _) => (CaretEq, 2),
            (b'^', _, _) => (Caret, 1),
            (b'!', b'=', _) => (Ne, 2),
            (b'!', _, _) => (Bang, 1),
            (b'+', b'=', _) => (PlusEq, 2),
            (b'+', _, _) => (Plus, 1),
            (b'*', b'=', _) => (StarEq, 2),
            (b'*', _, _) => (Star, 1),
            (b'/', b'=', _) => (SlashEq, 2),
            (b'/', _, _) => (Slash, 1),
            (b'%', b'=', _) => (PercentEq, 2),
            (b'%', _, _) => (Percent, 1),
            (b'<', b'<', b'=') => (ShlEq, 3),
            (b'<', b'<', _) => (Shl, 2),
            (b'<', b'=', _) => (Le, 2),
            (b'<', _, _) => (Lt, 1),
            (b'>', b'=', _) => (Ge, 2),
            (b'>', _, _) => (Gt, 1),
            (b'=', b'=', _) => (EqEq, 2),
            (b'=', _, _) => (Assign, 1),
            _ => {
                let ch = self.text[self.pos..].chars().next().unwrap_or('?');
                return Err(self.err(self.pos, format!("unexpected character `{ch}`")));
            }
        })
    }

    fn ident(&mut self, lo: usize) -> Result<(), Diag> {
        let rest = &self.text[self.pos..];
        let mut end = 0;
        for (i, ch) in rest.char_indices() {
            let ok = if i == 0 { ch == '_' || is_xid_start(ch) } else { ch == '_' || is_xid_continue(ch) };
            if !ok {
                break;
            }
            end = i + ch.len_utf8();
        }
        if end == 0 {
            let ch = rest.chars().next().unwrap_or('?');
            return Err(self.err(lo, format!("unexpected character `{ch}`")));
        }
        let word = &rest[..end];
        self.pos += end;
        let tok = keyword(word).unwrap_or_else(|| Tok::Ident(word.to_string()));
        self.push(tok, lo);
        Ok(())
    }

    fn number(&mut self, lo: usize) -> Result<(), Diag> {
        let after_dot = self.out.last().map(|t| t.tok == Tok::Dot).unwrap_or(false);
        let (radix, start) = if self.peek() == b'0' && matches!(self.peek_at(1), b'x' | b'b' | b'o') {
            let r = match self.peek_at(1) {
                b'x' => 16,
                b'b' => 2,
                _ => 8,
            };
            (r, self.pos + 2)
        } else {
            (10, self.pos)
        };
        self.pos = start;
        let mut digits = String::new();
        while self.pos < self.src.len() {
            let c = self.peek();
            if c == b'_' {
                self.pos += 1;
            } else if (c as char).is_digit(radix) {
                digits.push(c as char);
                self.pos += 1;
            } else {
                break;
            }
        }
        if digits.is_empty() {
            return Err(self.err(lo, "missing digits in numeric literal"));
        }
        let mut is_float = false;
        if radix == 10 && !after_dot {
            if self.peek() == b'.' && self.peek_at(1).is_ascii_digit() {
                is_float = true;
                digits.push('.');
                self.pos += 1;
                while self.peek().is_ascii_digit() || self.peek() == b'_' {
                    if self.peek() != b'_' {
                        digits.push(self.peek() as char);
                    }
                    self.pos += 1;
                }
            }
            if matches!(self.peek(), b'e' | b'E')
                && (self.peek_at(1).is_ascii_digit()
                    || (matches!(self.peek_at(1), b'+' | b'-') && self.peek_at(2).is_ascii_digit()))
            {
                is_float = true;
                digits.push('e');
                self.pos += 1;
                if matches!(self.peek(), b'+' | b'-') {
                    digits.push(self.peek() as char);
                    self.pos += 1;
                }
                while self.peek().is_ascii_digit() {
                    digits.push(self.peek() as char);
                    self.pos += 1;
                }
            }
        }
        if self.peek().is_ascii_alphabetic() || self.peek() == b'_' {
            return Err(self.err(lo, "invalid suffix on numeric literal"));
        }
        if is_float {
            let v: f64 = digits.parse().map_err(|_| self.err(lo, "invalid float literal"))?;
            self.push(Tok::Float(v), lo);
        } else {
            let v = u128::from_str_radix(&digits, radix).map_err(|_| self.err(lo, "integer literal is too large"))?;
            self.push(Tok::Int(v), lo);
        }
        Ok(())
    }

    fn escape(&mut self, lo: usize) -> Result<char, Diag> {
        // at backslash
        self.pos += 1;
        let c = self.peek();
        self.pos += 1;
        Ok(match c {
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'0' => '\0',
            b'\\' => '\\',
            b'"' => '"',
            b'\'' => '\'',
            b'{' => '{',
            b'}' => '}',
            b'u' => {
                if self.peek() != b'{' {
                    return Err(self.err(lo, "expected `{` after `\\u`"));
                }
                self.pos += 1;
                let start = self.pos;
                while self.peek().is_ascii_hexdigit() {
                    self.pos += 1;
                }
                let hex = &self.text[start..self.pos];
                if self.peek() != b'}' || hex.is_empty() {
                    return Err(self.err(lo, "malformed unicode escape"));
                }
                self.pos += 1;
                let v = u32::from_str_radix(hex, 16).map_err(|_| self.err(lo, "malformed unicode escape"))?;
                char::from_u32(v).ok_or_else(|| self.err(lo, "invalid unicode scalar value in escape"))?
            }
            _ => return Err(self.err(self.pos - 2, "unknown escape sequence")),
        })
    }

    fn string(&mut self, lo: usize) -> Result<Vec<StrPart>, Diag> {
        let mut parts = Vec::new();
        let mut cur = String::new();
        loop {
            if self.pos >= self.src.len() {
                return Err(self.err(lo, "unterminated string literal"));
            }
            let c = self.peek();
            match c {
                b'"' => {
                    self.pos += 1;
                    break;
                }
                b'\\' => {
                    let ch = self.escape(lo)?;
                    cur.push(ch);
                }
                b'{' => {
                    let ilo = self.pos;
                    self.pos += 1;
                    if !cur.is_empty() {
                        parts.push(StrPart::Lit(std::mem::take(&mut cur)));
                    }
                    let mut sub = Lexer {
                        src: self.src,
                        text: self.text,
                        pos: self.pos,
                        file: self.file,
                        out: Vec::new(),
                        stack: Vec::new(),
                        interp: true,
                    };
                    sub.run()?;
                    if sub.out.is_empty() {
                        return Err(self.err(ilo, "empty interpolation in string literal"));
                    }
                    let end = sub.pos as u32;
                    sub.out.push(Token { tok: Tok::Eof, span: Span::new(self.file, end, end) });
                    self.pos = sub.pos + 1; // skip `}`
                    parts.push(StrPart::Expr(sub.out, Span::new(self.file, ilo as u32, self.pos as u32)));
                }
                b'}' => {
                    return Err(self.err(self.pos, "unescaped `}` in string literal; write `\\}`"));
                }
                _ => {
                    let ch = self.text[self.pos..].chars().next().unwrap();
                    cur.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        if !cur.is_empty() || parts.is_empty() {
            parts.push(StrPart::Lit(cur));
        }
        Ok(parts)
    }
}

fn is_xid_start(c: char) -> bool {
    c.is_alphabetic()
}

fn is_xid_continue(c: char) -> bool {
    c.is_alphanumeric()
}
