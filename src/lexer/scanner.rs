// Copyright 2023-2025 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::str::CharIndices;

pub struct Scanner<'a> {
    raw_chars: &'a str,
    chars: CharIndices<'a>,
    pub index: usize,
    cur_char: Option<char>,

    // TODO Consider using a "cursor" abstraction for tracking the current line
    // and column.
    line: usize,
    col: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(chars: &'a str) -> Self {
        let mut char_indices = chars.char_indices();

        let cur_char = char_indices.next().map(|(_, c)| c);
        let mut loc = (1, 1);
        if let Some('\n') = cur_char {
            loc = (2, 0);
        }

        Scanner{
            raw_chars: chars,
            chars: char_indices,
            index: 0,
            cur_char,

            line: loc.0,
            col: loc.1,
        }
    }

    pub fn peek_char(&mut self) -> Option<char> {
        self.cur_char
    }

    pub fn next_char(&mut self) {
        // We use `chars.next()` to iterate through the characters of `chars`
        // because the characters of a UTF-8 string can't be indexed in
        // constant time.

        if let Some((i, c)) = self.chars.next() {
            self.index = i;
            self.cur_char = Some(c);

            if c == '\n' {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
        } else {
            self.index = self.raw_chars.len();
            self.cur_char = None;
        }
    }

    pub fn loc(&mut self) -> (usize, usize) {
        (self.line, self.col)
    }

    pub fn range(&self, start: usize, end: usize) -> &'a str {
        &self.raw_chars[start..end]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scans() {
        let tests = &[
            ("$", (1, 1)),
            ("1234$", (1, 5)),
            ("\n$", (2, 1)),
            ("\n1234$", (2, 5)),
            ("\n\n$", (3, 1)),
            ("\n\n1234$", (3, 5)),
        ];

        for (src, exp_dollar_loc) in tests {
            assert_scan_dollar(src, *exp_dollar_loc);
        }
    }

    fn assert_scan_dollar(src: &str, exp_dollar_loc: (usize, usize)) {
        let mut scanner = Scanner::new(src);

        while let Some(c) = scanner.peek_char() {
            if c == '$' {
                assert_eq!(
                    exp_dollar_loc,
                    scanner.loc(),
                    "unexpected location for '{src}'",
                );
            }
            scanner.next_char();
        }
    }
}
