// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::num::IntErrorKind;

mod scanner;

use self::scanner::Scanner;

pub type InterpSlot = (usize, usize);

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    IntLiteral(i64),
    StrLiteral(String),
    InterpStrLiteral(String, Vec<InterpSlot>),

    Break,
    Continue,
    Else,
    False,
    Fn,
    For,
    If,
    In,
    Null,
    Return,
    True,
    While,

    BraceClose,
    BraceOpen,
    BracketClose,
    BracketOpen,
    Colon,
    Comma,
    Div,
    Dot,
    Equals,
    GreaterThan,
    LessThan,
    Mod,
    Mul,
    ParenClose,
    ParenOpen,
    Semicolon,
    Sub,
    Sum,

    AmpAmp,
    BangEquals,
    ColonEquals,
    DashGreaterThan,
    DivEquals,
    DotDot,
    EqualsEquals,
    GreaterThanEquals,
    LessThanEquals,
    ModEquals,
    MulEquals,
    PipePipe,
    SubEquals,
    SumEquals,
}

#[derive(Debug)]
pub enum LexError {
    Unexpected(Location, char),
    IntTooHigh(Location, String),
    IntTooLow(Location, String),
    UnescapedDollar(Location),
    InvalidInterpolationStart(Location, char),
    InvalidEscapeChar(Location, char),
    InvalidHexChar(Location, char),
}

pub struct Lexer<'input> {
    pub scanner: Scanner<'input>,
}

impl<'input> Lexer<'input> {
    pub fn new(chars: &'input str) -> Self {
        Lexer{scanner: Scanner::new(chars)}
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.scanner.peek_char() {
            if c == '#' {
                while let Some(c_) = self.scanner.peek_char() {
                    if c_ == '\n' {
                        break;
                    }
                    self.scanner.next_char();
                }
            } else {
                if !c.is_ascii_whitespace() {
                    return;
                }
                self.scanner.next_char();
            }
        }
    }

    fn next_keyword_or_ident(&mut self) -> Token {
        let start = self.scanner.index;
        while let Some(c) = self.scanner.peek_char() {
            if !c.is_ascii_alphanumeric() && c != '_' {
                break;
            }
            self.scanner.next_char();
        }
        let end = self.scanner.index;

        let t = self.scanner.range(start, end);

        match t {
            "break" => Token::Break,
            "continue" => Token::Continue,
            "else" => Token::Else,
            "false" => Token::False,
            "fn" => Token::Fn,
            "for" => Token::For,
            "if" => Token::If,
            "in" => Token::In,
            "null" => Token::Null,
            "return" => Token::Return,
            "true" => Token::True,
            "while" => Token::While,

            _ => Token::Ident(t.to_string()),
        }
    }

    fn next_int(&mut self) -> Result<Token, LexError> {
        let loc = self.scanner.loc();

        let start = self.scanner.index;
        while let Some(c) = self.scanner.peek_char() {
            if !c.is_ascii_digit() && c != '_' {
                break;
            }
            self.scanner.next_char();
        }
        let end = self.scanner.index;

        let raw_int = self.scanner.range(start, end).to_string();
        let int: i64 =
            match raw_int.replace('_', "").parse() {
                Ok(v) => {
                    v
                },
                Err(e) => {
                    match e.kind() {
                        IntErrorKind::PosOverflow =>
                            return Err(LexError::IntTooHigh(loc, raw_int)),
                        IntErrorKind::NegOverflow =>
                            return Err(LexError::IntTooLow(loc, raw_int)),
                        e =>
                            panic!(
                                "unexpected parse error ({:?}) for '{}'",
                                e,
                                raw_int,
                            ),
                    };
                },
            };

        Ok(Token::IntLiteral(int))
    }

    #[allow(clippy::too_many_lines)]
    fn next_str_literal(&mut self, interpolate: bool)
        -> Result<Token, LexError>
    {
        self.scanner.next_char();

        let mut chars = vec![];
        let mut state = StrScanState::None;
        let mut first_hex_char = None;

        let mut cur_interpolation_start = 0;
        let mut interpolation_slots = vec![];
        let mut interpolation_brace_count = 0;

        while let Some(c) = self.scanner.peek_char() {
            let cur_loc = self.scanner.loc();

            self.scanner.next_char();

            match state {
                StrScanState::None => {
                    if c == '\\' {
                        state = StrScanState::Escape;
                    } else if c == '$' {
                        if interpolate {
                            cur_interpolation_start = chars.len();
                            state = StrScanState::Interpolate;
                            chars.push('$');
                        } else {
                            return Err(LexError::UnescapedDollar(cur_loc));
                        }
                    } else if c == '"' {
                        break;
                    } else {
                        chars.push(c);
                    }
                },
                StrScanState::Escape => {
                    if c == '\\' || c == '"' || c == '$' {
                        chars.push(c);
                    } else if c == 'n' {
                        chars.push('\n');
                    } else if c == 'r' {
                        chars.push('\r');
                    } else if c == 'x' {
                        state = StrScanState::Hex;
                        continue;
                    } else {
                        return Err(LexError::InvalidEscapeChar(cur_loc, c));
                    }
                    state = StrScanState::None;
                },
                StrScanState::Hex => {
                    let h =
                        match u8::from_str_radix(&c.to_string(), 16) {
                            Ok(v) => v,
                            Err(_) => return Err(LexError::InvalidHexChar(
                                cur_loc,
                                c,
                            )),
                        };

                    match first_hex_char {
                        None => {
                            first_hex_char = Some(h);
                        },
                        Some(n) => {
                            chars.push((n * 16 + h) as char);

                            first_hex_char = None;
                            state = StrScanState::None;
                        },
                    }
                },
                // NOTE We identify interpolation slots during lexing instead
                // of parsing because the identification of escapes (namely
                // `\\` and `\$`) needs to be performed at the same time. If
                // not, escaping `$` would require a double backslash (`\\$`)
                // because the handling of the escapes would be performed in
                // two separate steps. Another solution would be to use a
                // different escape character for the different steps, but we
                // use the current approach for consistency.
                StrScanState::Interpolate => {
                    if cur_interpolation_start+1 == chars.len() && c != '{' {
                        return Err(LexError::InvalidInterpolationStart(
                            cur_loc,
                            c,
                        ));
                    }

                    if c == '{' {
                        interpolation_brace_count += 1;
                    } else if c == '}' {
                        interpolation_brace_count -= 1;
                    }

                    if interpolation_brace_count == 0 {
                        // We shorten the slot to ignore the delimiters.
                        let slot = (cur_interpolation_start, chars.len()+1);
                        interpolation_slots.push(slot);
                        state = StrScanState::None;
                    }

                    chars.push(c);
                },
            }
        }

        let s = chars.into_iter().collect();

        if interpolate {
            Ok(Token::InterpStrLiteral(s, interpolation_slots))
        } else {
            Ok(Token::StrLiteral(s))
        }
    }

    fn next_symbol_token(&mut self, c: char) -> Option<Token> {
        if let Some(initial_t) = match_single_symbol_token(c) {
            self.scanner.next_char();

            let next_char =
                match self.scanner.peek_char() {
                    Some(c) => c,
                    None => return Some(initial_t),
                };

            let t =
                match match_double_symbol_token(c, next_char) {
                    Some(t) => t,
                    None => return Some(initial_t),
                };

            self.scanner.next_char();

            Some(t)
        } else {
            self.next_double_symbol_token(c)
        }
    }

    fn next_double_symbol_token(&mut self, first_char: char) -> Option<Token> {
        self.scanner.next_char();
        let second_char = self.scanner.peek_char()?;
        self.scanner.next_char();

        match_double_symbol_token(first_char, second_char)
    }
}

enum StrScanState {
    None,
    Escape,
    Hex,
    Interpolate,
}

pub type Span = (Location, Token, Location);

pub type Location = (usize, usize);

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<Span, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace_and_comments();

        let start_loc = self.scanner.loc();

        let c = self.scanner.peek_char()?;

        let t =
            if c.is_ascii_alphabetic() || c == '_' {
                self.next_keyword_or_ident()
            } else if c.is_ascii_digit() {
                match self.next_int() {
                    Ok(n) => n,
                    Err(e) => return Some(Err(e)),
                }
            } else if c == '"' || c == '$' {
                let mut interpolate = false;
                if c == '$' {
                    self.scanner.next_char();
                    interpolate = true;
                }

                match self.next_str_literal(interpolate) {
                    Ok(s) => s,
                    Err(e) => return Some(Err(e)),
                }
            } else if let Some(t) = self.next_symbol_token(c) {
                t
            } else {
                return Some(Err(LexError::Unexpected(start_loc, c)));
            };

        let (line, mut col) = self.scanner.loc();
        if self.scanner.peek_char().is_some() && col > 0 {
            // We reduce the column count by one, under the assumption that in
            // the successful cases above the scanner has progressed one
            // character beyond the end of the current token.
            col -= 1;
        }

        Some(Ok((start_loc, t, (line, col))))
    }
}

fn match_single_symbol_token(c: char) -> Option<Token> {
    match c {
        '}' => Some(Token::BraceClose),
        '{' => Some(Token::BraceOpen),
        ']' => Some(Token::BracketClose),
        '[' => Some(Token::BracketOpen),
        ':' => Some(Token::Colon),
        ',' => Some(Token::Comma),
        '/' => Some(Token::Div),
        '.' => Some(Token::Dot),
        '=' => Some(Token::Equals),
        '>' => Some(Token::GreaterThan),
        '<' => Some(Token::LessThan),
        '%' => Some(Token::Mod),
        '*' => Some(Token::Mul),
        ')' => Some(Token::ParenClose),
        '(' => Some(Token::ParenOpen),
        ';' => Some(Token::Semicolon),
        '-' => Some(Token::Sub),
        '+' => Some(Token::Sum),

        _ => None,
    }
}

fn match_double_symbol_token(a: char, b: char) -> Option<Token> {
    match (a, b) {
        ('&', '&') => Some(Token::AmpAmp),
        ('!', '=') => Some(Token::BangEquals),
        (':', '=') => Some(Token::ColonEquals),
        ('-', '>') => Some(Token::DashGreaterThan),
        ('/', '=') => Some(Token::DivEquals),
        ('.', '.') => Some(Token::DotDot),
        ('=', '=') => Some(Token::EqualsEquals),
        ('>', '=') => Some(Token::GreaterThanEquals),
        ('<', '=') => Some(Token::LessThanEquals),
        ('%', '=') => Some(Token::ModEquals),
        ('*', '=') => Some(Token::MulEquals),
        ('|', '|') => Some(Token::PipePipe),
        ('-', '=') => Some(Token::SubEquals),
        ('+', '=') => Some(Token::SumEquals),

        _ => None,
    }
}

#[cfg(test)]
mod test {
    // The testing approach taken in this module is largely inspired by the
    // approach used in
    // <https://github.com/gluon-lang/gluon/blob/d7ce3e8/parser/src/token.rs>.

    use super::*;

    #[test]
    fn test_lexs() {
        let tests = &[
            (
                r#"print ( "hello" )"#,
                r#"(---) - (-----) -"#,
                vec![
                    Token::Ident("print".to_string()),
                    Token::ParenOpen,
                    Token::StrLiteral("hello".to_string()),
                    Token::ParenClose,
                ],
            ),
            (
                r#"test := 1234 ;"#,
                r#"(--) () (--) -"#,
                vec![
                    Token::Ident("test".to_string()),
                    Token::ColonEquals,
                    Token::IntLiteral(1234),
                    Token::Semicolon,
                ],
            ),
        ];

        for (src, encoded_exp_locs, exp_toks) in tests.iter() {
            assert_lex(src, encoded_exp_locs, exp_toks.clone());
        }
    }

    fn assert_lex(src: &str, encoded_exp_locs: &str, exp_toks: Vec<Token>) {
        let mut lexer = Lexer::new(src);

        let exp_spans = new_expected_spans(encoded_exp_locs, exp_toks);

        for (n, exp_span) in exp_spans.into_iter().enumerate() {
            let act_span = lexer.next()
                .expect("token stream ended before expected")
                .expect("unexpected error in token stream");

            assert_eq!(
                exp_span,
                act_span,
                "span {} of '{}' wasn't as expected",
                n,
                src,
            );
        }

        let r = lexer.next();
        assert!(
            matches!(r, None),
            "expected end of token stream, got '{:?}'",
            r,
        );
    }

    fn new_expected_spans(encoded_exp_locs: &str, exp_toks: Vec<Token>)
        -> Vec<Span>
    {
        let exp_locs = parse_encoded_locs(encoded_exp_locs);

        if exp_locs.len() != exp_toks.len() {
            panic!("unbalanced number of expected spans and expected tokens");
        }

        exp_locs
            .into_iter()
            .zip(exp_toks)
            .map(|((start, end), tok)| (start, tok, end))
            .collect()
    }

    fn parse_encoded_locs(encoded_locs: &str) -> Vec<(Location, Location)> {
        let mut locs: Vec<(Location, Location)> = vec![];

        let mut line = 1;
        let mut col = 0;
        let mut span_start = None;

        for c in encoded_locs.chars() {
            col += 1;
            match c {
                '\n' => {
                    line += 1;
                    col = 0;
                },
                '(' => {
                    if span_start.is_some() {
                        panic!("encountered '(' inside span");
                    }
                    span_start = Some((line, col));
                },
                ')' => {
                    if let Some(start) = span_start {
                        let end = (line, col);
                        locs.push((start, end));
                    } else {
                        panic!("encountered ')' outside span");
                    }
                    span_start = None;
                },
                '-' => {
                    if span_start.is_none() {
                        let loc = (line, col);
                        locs.push((loc, loc));
                    }
                },
                ' ' => {
                    if span_start.is_some() {
                        panic!("encountered ' ' inside span");
                    }
                },
                c => {
                    panic!("encountered '{}' inside spans encoding", c);
                },
            }
        }

        locs
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_parse_encoded_locs() {
        let tests = &[
            (
                "-",
                vec![
                    ((1, 1), (1, 1)),
                ],
            ),
            (
                " -",
                vec![
                    ((1, 2), (1, 2)),
                ],
            ),
            (
                "--",
                vec![
                    ((1, 1), (1, 1)),
                    ((1, 2), (1, 2)),
                ],
            ),
            (
                " --",
                vec![
                    ((1, 2), (1, 2)),
                    ((1, 3), (1, 3)),
                ],
            ),
            (
                "---",
                vec![
                    ((1, 1), (1, 1)),
                    ((1, 2), (1, 2)),
                    ((1, 3), (1, 3)),
                ],
            ),
            (
                "- -",
                vec![
                    ((1, 1), (1, 1)),
                    ((1, 3), (1, 3)),
                ],
            ),
            (
                "- --",
                vec![
                    ((1, 1), (1, 1)),
                    ((1, 3), (1, 3)),
                    ((1, 4), (1, 4)),
                ],
            ),
            (
                "()",
                vec![
                    ((1, 1), (1, 2)),
                ],
            ),
            (
                "(-)",
                vec![
                    ((1, 1), (1, 3)),
                ],
            ),
            (
                "(--)",
                vec![
                    ((1, 1), (1, 4)),
                ],
            ),
            (
                " (--)",
                vec![
                    ((1, 2), (1, 5)),
                ],
            ),
            (
                "(--)-",
                vec![
                    ((1, 1), (1, 4)),
                    ((1, 5), (1, 5)),
                ],
            ),
            (
                "(--)--(--)",
                vec![
                    ((1, 1), (1, 4)),
                    ((1, 5), (1, 5)),
                    ((1, 6), (1, 6)),
                    ((1, 7), (1, 10)),
                ],
            ),
            (
                "--(--)--",
                vec![
                    ((1, 1), (1, 1)),
                    ((1, 2), (1, 2)),
                    ((1, 3), (1, 6)),
                    ((1, 7), (1, 7)),
                    ((1, 8), (1, 8)),
                ],
            ),
            (
                "-\n-",
                vec![
                    ((1, 1), (1, 1)),
                    ((2, 1), (2, 1)),
                ],
            ),
            (
                "-\n-\n-",
                vec![
                    ((1, 1), (1, 1)),
                    ((2, 1), (2, 1)),
                    ((3, 1), (3, 1)),
                ],
            ),
            (
                "-\n--\n-",
                vec![
                    ((1, 1), (1, 1)),
                    ((2, 1), (2, 1)),
                    ((2, 2), (2, 2)),
                    ((3, 1), (3, 1)),
                ],
            ),
            (
                "-\n---\n-",
                vec![
                    ((1, 1), (1, 1)),
                    ((2, 1), (2, 1)),
                    ((2, 2), (2, 2)),
                    ((2, 3), (2, 3)),
                    ((3, 1), (3, 1)),
                ],
            ),
            (
                "-\n()\n-",
                vec![
                    ((1, 1), (1, 1)),
                    ((2, 1), (2, 2)),
                    ((3, 1), (3, 1)),
                ],
            ),
            (
                "-\n(-)\n-",
                vec![
                    ((1, 1), (1, 1)),
                    ((2, 1), (2, 3)),
                    ((3, 1), (3, 1)),
                ],
            ),
        ];

        for (src, tgt) in tests {
            let locs = parse_encoded_locs(src);

            assert_eq!(
                &locs,
                tgt,
                "incorrect locations parsed from '{}'",
                src,
            );
        }
    }
}
