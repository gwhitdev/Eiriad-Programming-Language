use crate::error::{EiriadError, EiriadResult};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Let,
    Mut,
    Fn,
    Match,
    True,
    False,
    None,
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    Plus,
    Minus,
    Arrow,
    Star,
    Slash,
    Percent,
    Caret,
    Bang,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    PipeGt,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semi,
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
}

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    _source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            _source: source,
        }
    }

    pub fn lex(mut self) -> EiriadResult<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(c) = self.peek() {
            match c {
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '\n' => {
                    self.bump();
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        lexeme: "\\n".to_string(),
                    });
                }
                '\\' if self.peek_next() == Some('\n') => {
                    // A trailing backslash joins the next line into the current statement.
                    self.bump();
                    self.bump();
                }
                '/' if self.peek_next() == Some('/') => {
                    self.skip_line_comment();
                }
                '(' => {
                    self.bump();
                    tokens.push(simple(TokenKind::LParen, "("));
                }
                ')' => {
                    self.bump();
                    tokens.push(simple(TokenKind::RParen, ")"));
                }
                '{' => {
                    self.bump();
                    tokens.push(simple(TokenKind::LBrace, "{"));
                }
                '}' => {
                    self.bump();
                    tokens.push(simple(TokenKind::RBrace, "}"));
                }
                ',' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Comma, ","));
                }
                ':' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Colon, ":"));
                }
                ';' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Semi, ";"));
                }
                '+' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Plus, "+"));
                }
                '-' => {
                    self.bump();
                    if self.match_char('>') {
                        tokens.push(simple(TokenKind::Arrow, "->"));
                    } else {
                        tokens.push(simple(TokenKind::Minus, "-"));
                    }
                }
                '*' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Star, "*"));
                }
                '%' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Percent, "%"));
                }
                '^' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Caret, "^"));
                }
                '!' => {
                    self.bump();
                    if self.match_char('=') {
                        tokens.push(simple(TokenKind::Ne, "!="));
                    } else {
                        tokens.push(simple(TokenKind::Bang, "!"));
                    }
                }
                '=' => {
                    self.bump();
                    if self.match_char('=') {
                        tokens.push(simple(TokenKind::EqEq, "=="));
                    } else {
                        tokens.push(simple(TokenKind::Eq, "="));
                    }
                }
                '<' => {
                    self.bump();
                    if self.match_char('=') {
                        tokens.push(simple(TokenKind::Le, "<="));
                    } else {
                        tokens.push(simple(TokenKind::Lt, "<"));
                    }
                }
                '>' => {
                    self.bump();
                    if self.match_char('=') {
                        tokens.push(simple(TokenKind::Ge, ">="));
                    } else {
                        tokens.push(simple(TokenKind::Gt, ">"));
                    }
                }
                '&' if self.peek_next() == Some('&') => {
                    self.bump();
                    self.bump();
                    tokens.push(simple(TokenKind::AndAnd, "&&"));
                }
                '|' if self.peek_next() == Some('|') => {
                    self.bump();
                    self.bump();
                    tokens.push(simple(TokenKind::OrOr, "||"));
                }
                '|' if self.peek_next() == Some('>') => {
                    self.bump();
                    self.bump();
                    tokens.push(simple(TokenKind::PipeGt, "|>"));
                }
                '/' => {
                    self.bump();
                    tokens.push(simple(TokenKind::Slash, "/"));
                }
                '"' => {
                    tokens.push(self.lex_string()?);
                }
                d if d.is_ascii_digit() => {
                    tokens.push(self.lex_number()?);
                }
                ch if is_ident_start(ch) => {
                    tokens.push(self.lex_ident_or_keyword());
                }
                _ => {
                    return Err(EiriadError::new(format!("Unexpected character: {}", c)));
                }
            }
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
        });
        Ok(tokens)
    }

    fn lex_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        self.bump();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                self.bump();
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        let kind = match text.as_str() {
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "fn" => TokenKind::Fn,
            "match" => TokenKind::Match,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "None" => TokenKind::None,
            _ => TokenKind::Ident(text.clone()),
        };
        Token { kind, lexeme: text }
    }

    fn lex_number(&mut self) -> EiriadResult<Token> {
        let start = self.pos;
        self.bump();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }

        let mut is_float = false;
        if self.peek() == Some('.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }

        let text: String = self.chars[start..self.pos].iter().collect();
        if is_float {
            let value = text
                .parse::<f64>()
                .map_err(|_| EiriadError::new(format!("Invalid float literal: {}", text)))?;
            Ok(Token {
                kind: TokenKind::Float(value),
                lexeme: text,
            })
        } else {
            let value = text
                .parse::<i64>()
                .map_err(|_| EiriadError::new(format!("Invalid integer literal: {}", text)))?;
            Ok(Token {
                kind: TokenKind::Int(value),
                lexeme: text,
            })
        }
    }

    fn lex_string(&mut self) -> EiriadResult<Token> {
        self.bump();
        let mut out = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.bump();
                return Ok(Token {
                    kind: TokenKind::Str(out.clone()),
                    lexeme: out,
                });
            }

            if c == '\\' {
                self.bump();
                let escaped = self
                    .peek()
                    .ok_or_else(|| EiriadError::new("Unterminated escape sequence"))?;
                self.bump();
                match escaped {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    other => {
                        return Err(EiriadError::new(format!(
                            "Unsupported escape sequence: \\{}",
                            other
                        )));
                    }
                }
            } else {
                out.push(c);
                self.bump();
            }
        }
        Err(EiriadError::new("Unterminated string literal"))
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek() {
            self.bump();
            if c == '\n' {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }
}

fn simple(kind: TokenKind, lexeme: &str) -> Token {
    Token {
        kind,
        lexeme: lexeme.to_string(),
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}
