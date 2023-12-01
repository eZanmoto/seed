// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::str::CharIndices;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    StrLiteral(String),

    Comma,
    ParenClose,
    ParenOpen,
    Semicolon,
}

#[derive(Debug)]
pub enum LexError {
    Unexpected(Location, char),
}

pub struct Lexer<'input> {
    raw_chars: &'input str,
    chars: CharIndices<'input>,
    index: usize,
    cur: Option<(usize, char)>,

    // TODO Consider using a "cursor" abstraction for tracking the current line
    // and column.
    line: usize,
    col: usize,
}

impl<'input> Lexer<'input> {
    pub fn new(chars: &'input str) -> Self {
        let mut char_indices = chars.char_indices();

        let cur = char_indices.next();
        let mut line = 1;
        if let Some((_, '\n')) = cur {
            line += 1;
        }

        Lexer{
            raw_chars: chars,
            chars: char_indices,
            index: 0,
            cur,

            line,
            col: 1,
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == '#' {
                while let Some(c_) = self.peek_char() {
                    if c_ == '\n' {
                        break;
                    }
                    self.next_char();
                }
            } else {
                if !c.is_ascii_whitespace() {
                    return;
                }
                self.next_char();
            }
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        if let Some((_, c)) = self.cur {
            Some(c)
        } else {
            None
        }
    }

    fn next_char(&mut self) {
        // TODO Consider creating a separate cursor type that just keeps track
        // of the current index, character and file position.
        if let Some((i, c)) = self.chars.next() {
            self.index = i;
            self.cur = Some((i, c));

            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        } else {
            self.index = self.raw_chars.len();
            self.cur = None;
        }
    }

    fn loc(&mut self) -> Location {
        (self.line, self.col)
    }

    // `end_loc` returns the location one character before where the cursor
    // currently is, under the assumption that this function is called after
    // the newest token is parsed, such that the cursor has progressed one
    // character beyond the end of the token. This function just returns the
    // current location if the end of the token stream has been reached.
    fn end_loc(&mut self) -> Location {
        let (line, mut col) = self.loc();
        if self.peek_char().is_some() {
            col -= 1;
        }

        (line, col)
    }

    fn next_ident(&mut self, start: usize, start_loc: Location) -> Span {
        while let Some(c) = self.peek_char() {
            if !c.is_ascii_alphanumeric() && c != '_' {
                break;
            }
            self.next_char();
        }
        let end = self.index;
        let end_loc = self.end_loc();

        let id = &self.raw_chars[start..end];

        let t = Token::Ident(id.to_string());

        (start_loc, t, end_loc)
    }

    fn next_str_literal<F>(&mut self, start_loc: Location, new_str_token: F)
        -> Span
    where
        F: FnOnce(String) -> Token
    {
        let start = self.index;

        self.next_char();

        while let Some(c) = self.peek_char() {
            self.next_char();
            if c == '"' {
                break;
            }
        }
        let end = self.index;
        let end_loc = self.end_loc();

        let id = &self.raw_chars[(start + 1)..(end - 1)];

        let t = new_str_token(id.to_string());

        (start_loc, t, end_loc)
    }

    fn next_symbol_token(&mut self, c: char) -> Option<Span> {
        let start_loc = self.loc();

        if let Some(t) = match_single_symbol_token(c) {
            self.next_char();
            let end_loc = self.end_loc();

            Some((start_loc, t, end_loc))
        } else {
            None
        }
    }
}

pub type Span = (Location, Token, Location);

pub type Location = (usize, usize);

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<Span, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace_and_comments();

        let start = self.index;
        let start_loc = self.loc();

        let c = self.peek_char()?;

        let result =
            if c.is_ascii_alphabetic() || c == '_' {
                Ok(self.next_ident(start, start_loc))
            } else if c == '"' {
                Ok(self.next_str_literal(start_loc, Token::StrLiteral))
            } else if let Some(t) = self.next_symbol_token(c) {
                Ok(t)
            } else {
                Err(LexError::Unexpected(self.loc(), c))
            };

        Some(result)
    }
}

fn match_single_symbol_token(c: char) -> Option<Token> {
    match c {
        ',' => Some(Token::Comma),
        ')' => Some(Token::ParenClose),
        '(' => Some(Token::ParenOpen),
        ';' => Some(Token::Semicolon),

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
                // FIXME The start location for the interpolated string literal
                // should ideally include the `$` prefix.
                r#"print ( "hello" )"#,
                r#"(---) - (-----) -"#,
                vec![
                    Token::Ident("print".to_string()),
                    Token::ParenOpen,
                    Token::StrLiteral("hello".to_string()),
                    Token::ParenClose,
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
