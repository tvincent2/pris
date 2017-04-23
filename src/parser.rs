// Pris -- A language for designing slides
// Copyright 2017 Ruud van Asseldonk

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3. A copy
// of the License is available in the root of the repository.

//! This module contains building blocks for the parser. The actual parser is
//! generated by Lalrpop, and can be found in the `syntax` module.

use lalrpop_util;
use lexer::Token;
use std::char;

type ParseError<'a> = lalrpop_util::ParseError<usize, Token<'a>, String>;

/// Strips the '---' of a raw string literal and corrects its indentation.
pub fn unescape_raw_string_literal<'a>(s: &'a str) -> String {
    // TODO: Correct indentation.
    String::from(&s[3..s.len() - 3])
}

/// Turns a string literal into the string it represents.
///
/// For example, `"foo\"bar"` becomes `foo"bar`.
pub fn unescape_string_literal<'a>(s: &'a str)
                                   -> Result<String, ParseError<'a>> {
    let mut string = String::with_capacity(s.len() - 2);

    // Parsing escape sequences in a string literal is a small state machine
    // with the following states.
    enum EscState {
        // The base state.
        Normal,
        // After a backslash.
        Escape,
        // After '\u'. The state is (current_value, bytes_left).
        Unicode(u32, u32),
    }

    let mut st = EscState::Normal;

    // Iterate all characters except for the enclosing quotes.
    for ch in s[1..s.len() - 1].chars() {
        match st {
            EscState::Normal => {
                match ch {
                    '\\' => st = EscState::Escape,
                    _ => string.push(ch),
                }
            }
            EscState::Escape => {
                match ch {
                    '\\' => { string.push('\\'); st = EscState::Normal; }
                    '"' => { string.push('"'); st = EscState::Normal; }
                    'n' => { string.push('\n'); st = EscState::Normal; }
                    'u' => { st = EscState::Unicode(0, 6); }
                    _ => {
                        let err = lalrpop_util::ParseError::User {
                            error: format!("Invalid escape code '\\{}'.", ch),
                        };
                        return Err(err)
                    }
                }
            }
            EscState::Unicode(codepoint, num_left) => {
                // An unicode escape sequence of the form \u1f574 consists of at
                // most 6 hexadecimal characters, and ends at the first non-hex
                // character. Examples:
                // "\u"        -> U+0
                // "\u1f574z"  -> U+1F574, U+7A
                // "\u1f5741"  -> U+1F5741
                // "\u01f5741" -> U+1F574, U+31
                if ch.is_digit(16) && num_left > 0 {
                    // Parsing the digit will succeed, because we checked above.
                    let d = ch.to_digit(16).unwrap();
                    st = EscState::Unicode(codepoint * 16 + d, num_left - 1);
                } else {
                    // End of unicode escape, append the value and the current
                    // character which was not part of the escape.
                    string.push(char_from_codepoint(codepoint)?);
                    string.push(ch);
                    st = EscState::Normal;
                }
            }
        }
    }

    match st {
        // A string might end in an escape code.
        EscState::Unicode(codepoint, _num_left) => {
            string.push(char_from_codepoint(codepoint)?);
        }
        _ => { }
    }

    Ok(string)
}

fn char_from_codepoint<'a>(codepoint: u32) -> Result<char, ParseError<'a>> {
    match char::from_u32(codepoint) {
        Some(c) => Ok(c),
        None => {
            let err = lalrpop_util::ParseError::User {
                error: format!("Invalid code point U+{:X}.", codepoint),
            };
            Err(err)
        }
    }
}

#[test]
fn unescape_string_literal_strips_quotes() {
    let x = unescape_string_literal("\"\"");
    assert_eq!(Ok("".into()), x);
}

#[test]
fn unescape_string_literal_handles_escaped_quotes() {
    let x = unescape_string_literal("\"x\\\"y\"");
    assert_eq!(Ok("x\"y".into()), x);
}

#[test]
fn unescape_string_literal_handles_escaped_newlines() {
    let x = unescape_string_literal("\"\\n\"");
    assert_eq!(Ok("\n".into()), x);
}

#[test]
fn unescape_string_literal_handles_escaped_codepoints() {
    let x = unescape_string_literal("\"\\u1f574 Unicode 6 was a bad idea.\"");
    assert_eq!(Ok("\u{1f574} Unicode 6 was a bad idea.".into()), x);
}

#[test]
fn unescape_string_literal_handles_escaped_codepoints_at_end() {
    let x = unescape_string_literal("\"\\u1f574\"");
    assert_eq!(Ok("\u{1f574}".into()), x);
}

#[test]
fn unescape_string_literal_handles_short_escaped_codepoints() {
    let x = unescape_string_literal("\"\\u0anewline\"");
    assert_eq!(Ok("\nnewline".into()), x);
}

#[test]
fn unescape_string_literal_handles_long_escaped_codepoints() {
    let x = unescape_string_literal("\"\\u00000afg\"");
    assert_eq!(Ok("\nfg".into()), x);
    let y = unescape_string_literal("\"\\u0000afg\"");
    assert_eq!(Ok("\u{00af}g".into()), y);
}