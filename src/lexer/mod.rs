// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

mod scanner;

use self::scanner::Scanner;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    IntLiteral(i64),
    StrLiteral(String),

    False,
    Fn,
    Null,
    True,

    BraceClose,
    BraceOpen,
    BracketClose,
    BracketOpen,
    Colon,
    Comma,
    Div,
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
    DivEquals,
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
}

pub struct Lexer<'input> {
    scanner: Scanner<'input>,
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
            "false" => Token::False,
            "fn" => Token::Fn,
            "null" => Token::Null,
            "true" => Token::True,

            _ => Token::Ident(t.to_string()),
        }
    }

    fn next_int(&mut self) -> Token {
        let start = self.scanner.index;
        while let Some(c) = self.scanner.peek_char() {
            if !c.is_ascii_digit() {
                break;
            }
            self.scanner.next_char();
        }
        let end = self.scanner.index;

        let raw_int = self.scanner.range(start, end);
        let int: i64 = raw_int.parse().unwrap();

        Token::IntLiteral(int)
    }

    fn next_str_literal(&mut self) -> Token {
        let start = self.scanner.index;
        self.scanner.next_char();
        while let Some(c) = self.scanner.peek_char() {
            self.scanner.next_char();
            if c == '"' {
                break;
            }
        }
        let end = self.scanner.index;

        let id = self.scanner.range(start + 1, end - 1);

        Token::StrLiteral(id.to_string())
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
                self.next_int()
            } else if c == '"' {
                self.next_str_literal()
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
        ('/', '=') => Some(Token::DivEquals),
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
