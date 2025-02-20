/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2023 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use crate::ast::*;
use crate::parser::combinators::*;
use crate::parser::error::*;
use crate::parser::primitives::*;
use crate::parser::reader::Reader;
use crate::parser::{template, ParseResult};

/// Steps:
/// 1- parse String until end of stream or end of line or #
///    the string does not contain trailing space
/// 2- templatize
pub fn unquoted_template(reader: &mut Reader) -> ParseResult<'static, Template> {
    let start = reader.state.clone();
    let mut chars = vec![];
    let mut spaces = vec![];
    let mut end = start.clone();
    loop {
        let pos = reader.state.pos.clone();
        match any_char(vec!['#'], reader) {
            Err(e) => {
                if e.recoverable {
                    break;
                } else {
                    return Err(e);
                }
            }
            Ok((c, s)) => {
                if s == "\n" {
                    break;
                }
                if s == " " {
                    spaces.push((c, s, pos));
                } else {
                    if !spaces.is_empty() {
                        chars.append(&mut spaces);
                        spaces = vec![];
                    }
                    chars.push((c, s, pos));
                    end = reader.state.clone();
                }
            }
        }
    }
    reader.state = end.clone();
    let encoded_string = template::EncodedString {
        source_info: SourceInfo {
            start: start.pos.clone(),
            end: end.pos.clone(),
        },
        chars,
    };
    let elements = template::templatize(encoded_string)?;
    Ok(Template {
        delimiter: None,
        elements,
        source_info: SourceInfo {
            start: start.pos,
            end: end.pos,
        },
    })
}

pub fn unquoted_string_key(reader: &mut Reader) -> ParseResult<'static, EncodedString> {
    let start = reader.state.pos.clone();
    let mut value = String::new();
    let mut encoded = String::new();
    loop {
        let save = reader.state.clone();
        match escape_char(reader) {
            Ok(c) => {
                value.push(c);
                encoded.push_str(reader.peek_back(save.cursor).as_str())
            }
            Err(e) => {
                if e.recoverable {
                    reader.state = save.clone();
                    match reader.read() {
                        None => break,
                        Some(c) => {
                            if c.is_alphanumeric()
                                || c == '_'
                                || c == '-'
                                || c == '.'
                                || c == '['
                                || c == ']'
                                || c == '@'
                                || c == '$'
                            {
                                value.push(c);
                                encoded.push_str(reader.peek_back(save.cursor).as_str())
                            } else {
                                reader.state = save;
                                break;
                            }
                        }
                    }
                } else {
                    return Err(e);
                }
            }
        }
    }

    // check nonempty/ starts with [
    if value.is_empty() || encoded.starts_with('[') {
        return Err(Error {
            pos: start,
            recoverable: true,
            inner: ParseError::Expecting {
                value: "key string".to_string(),
            },
        });
    }

    let quotes = false;
    let end = reader.state.pos.clone();
    let source_info = SourceInfo { start, end };
    Ok(EncodedString {
        value,
        encoded,
        quotes,
        source_info,
    })
}

// TODO: should return an EncodedString
// (decoding escape sequence)
pub fn quoted_oneline_string(reader: &mut Reader) -> ParseResult<'static, String> {
    literal("\"", reader)?;
    let s = reader.read_while(|c| *c != '"' && *c != '\n');
    literal("\"", reader)?;
    Ok(s)
}

pub fn quoted_template(reader: &mut Reader) -> ParseResult<'static, Template> {
    let start = reader.state.clone().pos;
    let mut end = start.clone();
    try_literal("\"", reader)?;
    let mut chars = vec![];
    loop {
        let pos = reader.state.pos.clone();
        let save = reader.state.clone();
        match any_char(vec!['"'], reader) {
            Err(e) => {
                if e.recoverable {
                    reader.state = save;
                    break;
                } else {
                    return Err(e);
                }
            }
            Ok((c, s)) => {
                chars.push((c, s, pos));
                end = reader.state.clone().pos;
            }
        }
    }
    literal("\"", reader)?;
    let encoded_string = template::EncodedString {
        source_info: SourceInfo {
            start: start.clone(),
            end,
        },
        chars,
    };
    let elements = template::templatize(encoded_string)?;
    Ok(Template {
        delimiter: Some('"'),
        elements,
        source_info: SourceInfo {
            start,
            end: reader.state.pos.clone(),
        },
    })
}

pub fn backtick_template(reader: &mut Reader) -> ParseResult<'static, Template> {
    let delimiter = Some('`');
    let start = reader.state.clone().pos;
    let mut end = start.clone();
    try_literal("`", reader)?;
    let mut chars = vec![];
    loop {
        let pos = reader.state.pos.clone();
        let save = reader.state.clone();
        match any_char(vec!['`', '\n'], reader) {
            Err(e) => {
                if e.recoverable {
                    reader.state = save;
                    break;
                } else {
                    return Err(e);
                }
            }
            Ok((c, s)) => {
                chars.push((c, s, pos));
                end = reader.state.clone().pos;
            }
        }
    }
    literal("`", reader)?;
    let encoded_string = template::EncodedString {
        source_info: SourceInfo {
            start: start.clone(),
            end,
        },
        chars,
    };
    let elements = template::templatize(encoded_string)?;
    Ok(Template {
        delimiter,
        elements,
        source_info: SourceInfo {
            start,
            end: reader.state.pos.clone(),
        },
    })
}

fn any_char(except: Vec<char>, reader: &mut Reader) -> ParseResult<'static, (char, String)> {
    let start = reader.state.clone();
    match escape_char(reader) {
        Ok(c) => Ok((c, reader.peek_back(start.cursor))),
        Err(e) => {
            if e.recoverable {
                reader.state = start.clone();
                match reader.read() {
                    None => Err(Error {
                        pos: start.pos,
                        recoverable: true,
                        inner: ParseError::Expecting {
                            value: "char".to_string(),
                        },
                    }),
                    Some(c) => {
                        if except.contains(&c)
                            || vec!['\\', '\x08', '\n', '\x0c', '\r', '\t'].contains(&c)
                        {
                            Err(Error {
                                pos: start.pos,
                                recoverable: true,
                                inner: ParseError::Expecting {
                                    value: "char".to_string(),
                                },
                            })
                        } else {
                            Ok((c, reader.peek_back(start.cursor)))
                        }
                    }
                }
            } else {
                Err(e)
            }
        }
    }
}

fn escape_char(reader: &mut Reader) -> ParseResult<'static, char> {
    try_literal("\\", reader)?;
    let start = reader.state.clone();
    match reader.read() {
        Some('#') => Ok('#'),
        Some('"') => Ok('"'),
        Some('`') => Ok('`'),
        Some('\\') => Ok('\\'),
        Some('/') => Ok('/'),
        Some('b') => Ok('\x08'),
        Some('n') => Ok('\n'),
        Some('f') => Ok('\x0c'),
        Some('r') => Ok('\r'),
        Some('t') => Ok('\t'),
        Some('u') => unicode(reader),
        _ => Err(Error {
            pos: start.pos,
            recoverable: false,
            inner: ParseError::EscapeChar,
        }),
    }
}

fn unicode(reader: &mut Reader) -> ParseResult<'static, char> {
    literal("{", reader)?;
    let v = hex_value(reader)?;
    let c = match std::char::from_u32(v) {
        None => {
            return Err(Error {
                pos: reader.clone().state.pos,
                recoverable: false,
                inner: ParseError::Unicode,
            });
        }
        Some(c) => c,
    };
    literal("}", reader)?;
    Ok(c)
}

fn hex_value(reader: &mut Reader) -> ParseResult<'static, u32> {
    let mut digits = one_or_more(hex_digit, reader)?;
    let mut v = 0;
    let mut weight = 1;
    digits.reverse();
    for d in digits.iter() {
        v += weight * d;
        weight *= 16;
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn test_unquoted_template_empty() {
        let mut reader = Reader::new("");
        assert_eq!(
            unquoted_template(&mut reader).unwrap(),
            Template {
                delimiter: None,
                elements: vec![],
                source_info: SourceInfo::new(1, 1, 1, 1),
            }
        );
        assert_eq!(reader.state.cursor, 0);
    }

    #[test]
    fn test_unquoted_template_with_hash() {
        let mut reader = Reader::new("a#");
        assert_eq!(
            unquoted_template(&mut reader).unwrap(),
            Template {
                delimiter: None,
                elements: vec![TemplateElement::String {
                    value: "a".to_string(),
                    encoded: "a".to_string(),
                }],
                source_info: SourceInfo::new(1, 1, 1, 2),
            }
        );
        assert_eq!(reader.state.cursor, 1);
    }

    #[test]
    fn test_unquoted_template_with_encoded_hash() {
        let mut reader = Reader::new("a\\u{23}");
        assert_eq!(
            unquoted_template(&mut reader).unwrap(),
            Template {
                delimiter: None,
                elements: vec![TemplateElement::String {
                    value: "a#".to_string(),
                    encoded: "a\\u{23}".to_string(),
                }],
                source_info: SourceInfo::new(1, 1, 1, 8),
            }
        );
        assert_eq!(reader.state.cursor, 7);
    }

    #[test]
    fn test_unquoted_template_with_quote() {
        let mut reader = Reader::new("\"hi\"");
        assert_eq!(
            unquoted_template(&mut reader).unwrap(),
            Template {
                delimiter: None,
                elements: vec![TemplateElement::String {
                    value: "\"hi\"".to_string(),
                    encoded: "\"hi\"".to_string(),
                }],
                source_info: SourceInfo::new(1, 1, 1, 5),
            }
        );
        assert_eq!(reader.state.cursor, 4);
    }

    #[test]
    fn test_unquoted_template_hello_world() {
        let mut reader = Reader::new("hello\\u{20}{{name}}!");
        assert_eq!(
            unquoted_template(&mut reader).unwrap(),
            Template {
                delimiter: None,
                elements: vec![
                    TemplateElement::String {
                        value: "hello ".to_string(),
                        encoded: "hello\\u{20}".to_string(),
                    },
                    TemplateElement::Expression(Expr {
                        space0: Whitespace {
                            value: String::new(),
                            source_info: SourceInfo::new(1, 14, 1, 14),
                        },
                        variable: Variable {
                            name: "name".to_string(),
                            source_info: SourceInfo::new(1, 14, 1, 18),
                        },
                        space1: Whitespace {
                            value: String::new(),
                            source_info: SourceInfo::new(1, 18, 1, 18),
                        },
                    }),
                    TemplateElement::String {
                        value: "!".to_string(),
                        encoded: "!".to_string(),
                    },
                ],
                source_info: SourceInfo::new(1, 1, 1, 21),
            }
        );
        assert_eq!(reader.state.cursor, 20);
    }

    #[test]
    fn test_unquoted_template_trailing_space() {
        let mut reader = Reader::new("hello world # comment");
        assert_eq!(
            unquoted_template(&mut reader).unwrap(),
            Template {
                delimiter: None,
                elements: vec![TemplateElement::String {
                    value: "hello world".to_string(),
                    encoded: "hello world".to_string(),
                },],
                source_info: SourceInfo::new(1, 1, 1, 12),
            }
        );
        assert_eq!(reader.state.cursor, 11);
        assert_eq!(
            reader.state.pos,
            Pos {
                line: 1,
                column: 12
            }
        );
    }

    #[test]
    fn test_unquoted_key() {
        let mut reader = Reader::new("key");
        assert_eq!(
            unquoted_string_key(&mut reader).unwrap(),
            EncodedString {
                value: "key".to_string(),
                encoded: "key".to_string(),
                quotes: false,
                source_info: SourceInfo::new(1, 1, 1, 4),
            }
        );
        assert_eq!(reader.state.cursor, 3);

        let mut reader = Reader::new("key\\u{20}\\u{3a} :");
        assert_eq!(
            unquoted_string_key(&mut reader).unwrap(),
            EncodedString {
                value: "key :".to_string(),
                encoded: "key\\u{20}\\u{3a}".to_string(),
                quotes: false,
                source_info: SourceInfo::new(1, 1, 1, 16),
            }
        );
        assert_eq!(reader.state.cursor, 15);

        let mut reader = Reader::new("$top:");
        assert_eq!(
            unquoted_string_key(&mut reader).unwrap(),
            EncodedString {
                value: "$top".to_string(),
                encoded: "$top".to_string(),
                quotes: false,
                source_info: SourceInfo::new(1, 1, 1, 5),
            }
        );
        assert_eq!(reader.state.cursor, 4);
    }

    #[test]
    fn test_unquoted_key_with_square_bracket() {
        let mut reader = Reader::new("values\\u{5b}0\\u{5d} :");
        assert_eq!(
            unquoted_string_key(&mut reader).unwrap(),
            EncodedString {
                value: "values[0]".to_string(),
                encoded: "values\\u{5b}0\\u{5d}".to_string(),
                quotes: false,
                source_info: SourceInfo::new(1, 1, 1, 20),
            }
        );
        assert_eq!(reader.state.cursor, 19);

        let mut reader = Reader::new("values[0] :");
        assert_eq!(
            unquoted_string_key(&mut reader).unwrap(),
            EncodedString {
                value: "values[0]".to_string(),
                encoded: "values[0]".to_string(),
                quotes: false,
                source_info: SourceInfo::new(1, 1, 1, 10),
            }
        );
        assert_eq!(reader.state.cursor, 9);
    }

    #[test]
    fn test_unquoted_keys_ignore_start_square_bracket() {
        let mut reader = Reader::new("[0]:");
        let error = unquoted_string_key(&mut reader).err().unwrap();
        assert!(error.recoverable);
        assert_eq!(reader.state.cursor, 3);
    }

    #[test]
    fn test_unquoted_keys_accept_start_escape_square_bracket() {
        let mut reader = Reader::new("\\u{5b}0\\u{5d}");
        assert_eq!(
            unquoted_string_key(&mut reader).unwrap(),
            EncodedString {
                value: "[0]".to_string(),
                encoded: "\\u{5b}0\\u{5d}".to_string(),
                quotes: false,
                source_info: SourceInfo::new(1, 1, 1, 14),
            }
        );
        assert_eq!(reader.state.cursor, 13);
    }

    #[test]
    fn test_unquoted_key_error() {
        let mut reader = Reader::new("");
        let error = unquoted_string_key(&mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert_eq!(
            error.inner,
            ParseError::Expecting {
                value: "key string".to_string()
            }
        );

        let mut reader = Reader::new("\\l");
        let error = unquoted_string_key(&mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 2 });
        assert_eq!(error.inner, ParseError::EscapeChar);

        let mut reader = Reader::new(r#"{"id":1}"#);
        let error = unquoted_string_key(&mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert_eq!(
            error.inner,
            ParseError::Expecting {
                value: "key string".to_string()
            }
        );
    }

    #[test]
    fn test_quoted_template() {
        let mut reader = Reader::new("\"\"");
        assert_eq!(
            quoted_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('"'),
                elements: vec![],
                source_info: SourceInfo::new(1, 1, 1, 3),
            }
        );
        assert_eq!(reader.state.cursor, 2);

        let mut reader = Reader::new("\"a#\"");
        assert_eq!(
            quoted_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('"'),
                elements: vec![TemplateElement::String {
                    value: "a#".to_string(),
                    encoded: "a#".to_string(),
                }],
                source_info: SourceInfo::new(1, 1, 1, 5),
            }
        );
        assert_eq!(reader.state.cursor, 4);

        let mut reader = Reader::new("\"{0}\"");
        assert_eq!(
            quoted_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('"'),
                elements: vec![TemplateElement::String {
                    value: "{0}".to_string(),
                    encoded: "{0}".to_string(),
                }],
                source_info: SourceInfo::new(1, 1, 1, 6),
            }
        );
        assert_eq!(reader.state.cursor, 5);
    }

    #[test]
    fn test_quoted_template_with_quote() {
        // "\"hi\""
        let mut reader = Reader::new("\"\\\"hi\\\"\"");
        assert_eq!(
            quoted_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('"'),
                elements: vec![TemplateElement::String {
                    value: "\"hi\"".to_string(),
                    encoded: "\\\"hi\\\"".to_string()
                }],
                source_info: SourceInfo::new(1, 1, 1, 9),
            }
        );
        assert_eq!(reader.state.cursor, 8);
    }

    #[test]
    fn test_quoted_template_error_missing_closing_quote() {
        let mut reader = Reader::new("\"not found");
        let error = quoted_template(&mut reader).err().unwrap();
        assert_eq!(
            error.pos,
            Pos {
                line: 1,
                column: 11
            }
        );
        assert!(!error.recoverable);
    }

    #[test]
    fn test_quoted_string() {
        let mut reader = Reader::new("\"\"");
        assert_eq!(quoted_oneline_string(&mut reader).unwrap(), "");
        assert_eq!(reader.state.cursor, 2);

        let mut reader = Reader::new("\"Hello\"");
        assert_eq!(quoted_oneline_string(&mut reader).unwrap(), "Hello");
        assert_eq!(reader.state.cursor, 7);
    }

    #[test]
    fn test_backtick_template() {
        let mut reader = Reader::new("``");
        assert_eq!(
            backtick_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('`'),
                elements: vec![],
                source_info: SourceInfo::new(1, 1, 1, 3),
            }
        );
        assert_eq!(reader.state.cursor, 2);

        let mut reader = Reader::new("`foo#`");
        assert_eq!(
            backtick_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('`'),
                elements: vec![TemplateElement::String {
                    value: "foo#".to_string(),
                    encoded: "foo#".to_string(),
                }],
                source_info: SourceInfo::new(1, 1, 1, 7),
            }
        );
        assert_eq!(reader.state.cursor, 6);

        let mut reader = Reader::new("`{0}`");
        assert_eq!(
            backtick_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('`'),
                elements: vec![TemplateElement::String {
                    value: "{0}".to_string(),
                    encoded: "{0}".to_string(),
                }],
                source_info: SourceInfo::new(1, 1, 1, 6),
            }
        );
        assert_eq!(reader.state.cursor, 5);
    }

    #[test]
    fn test_backtick_template_with_backtick() {
        // `\`hi\``
        let mut reader = Reader::new("`\\`hi\\``");
        assert_eq!(
            backtick_template(&mut reader).unwrap(),
            Template {
                delimiter: Some('`'),
                elements: vec![TemplateElement::String {
                    value: "`hi`".to_string(),
                    encoded: "\\`hi\\`".to_string()
                }],
                source_info: SourceInfo::new(1, 1, 1, 9),
            }
        );
        assert_eq!(reader.state.cursor, 8);
    }

    #[test]
    fn test_backtick_template_error_missing_closing_backtick() {
        let mut reader = Reader::new("`not found");
        let error = backtick_template(&mut reader).err().unwrap();
        assert_eq!(
            error.pos,
            Pos {
                line: 1,
                column: 11
            }
        );
        assert!(!error.recoverable);
    }

    #[test]
    fn test_any_char() {
        let mut reader = Reader::new("a");
        assert_eq!(
            any_char(vec![], &mut reader).unwrap(),
            ('a', "a".to_string())
        );
        assert_eq!(reader.state.cursor, 1);

        let mut reader = Reader::new(" ");
        assert_eq!(
            any_char(vec![], &mut reader).unwrap(),
            (' ', " ".to_string())
        );
        assert_eq!(reader.state.cursor, 1);

        let mut reader = Reader::new("\\t");
        assert_eq!(
            any_char(vec![], &mut reader).unwrap(),
            ('\t', "\\t".to_string())
        );
        assert_eq!(reader.state.cursor, 2);

        let mut reader = Reader::new("#");
        assert_eq!(
            any_char(vec![], &mut reader).unwrap(),
            ('#', "#".to_string())
        );
        assert_eq!(reader.state.cursor, 1);
    }

    #[test]
    fn test_any_char_quote() {
        let mut reader = Reader::new("\\\"");
        assert_eq!(
            any_char(vec![], &mut reader).unwrap(),
            ('"', "\\\"".to_string())
        );
        assert_eq!(reader.state.cursor, 2);
    }

    #[test]
    fn test_any_char_error() {
        let mut reader = Reader::new("");
        let error = any_char(vec![], &mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert!(error.recoverable);

        let mut reader = Reader::new("#");
        let error = any_char(vec!['#'], &mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert!(error.recoverable);

        let mut reader = Reader::new("\t");
        let error = any_char(vec![], &mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert!(error.recoverable);
    }

    #[test]
    fn test_escape_char() {
        let mut reader = Reader::new("\\n");
        assert_eq!(escape_char(&mut reader).unwrap(), '\n');
        assert_eq!(reader.state.cursor, 2);

        let mut reader = Reader::new("\\u{0a}");
        assert_eq!(escape_char(&mut reader).unwrap(), '\n');
        assert_eq!(reader.state.cursor, 6);

        let mut reader = Reader::new("x");
        let error = escape_char(&mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert_eq!(
            error.inner,
            ParseError::Expecting {
                value: "\\".to_string()
            }
        );
        assert!(error.recoverable);
        assert_eq!(reader.state.cursor, 0);
    }

    #[test]
    fn test_unicode() {
        let mut reader = Reader::new("{000a}");
        assert_eq!(unicode(&mut reader).unwrap(), '\n');
        assert_eq!(reader.state.cursor, 6);

        let mut reader = Reader::new("{E9}");
        assert_eq!(unicode(&mut reader).unwrap(), 'é');
        assert_eq!(reader.state.cursor, 4);
    }

    #[test]
    fn test_hex_value() {
        let mut reader = Reader::new("20x");
        assert_eq!(hex_value(&mut reader).unwrap(), 32);

        let mut reader = Reader::new("x");
        let error = hex_value(&mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert_eq!(error.inner, ParseError::HexDigit);
        assert!(!error.recoverable);
    }

    #[test]
    fn test_quoted_template_benchmark() {
        // benchmark tests not in stable toolchain yet
        // Simply log duration for the time-being
        let mut reader = Reader::new(
            format!(
                "\"Hello World!\"{}",
                (0..10_000_000).map(|_| "X").collect::<String>()
            )
            .as_str(),
        );

        let now = SystemTime::now();
        assert!(quoted_template(&mut reader).is_ok());
        assert_eq!(reader.state.cursor, 14);
        eprintln!("duration= {}", now.elapsed().unwrap().as_nanos());
    }
}
