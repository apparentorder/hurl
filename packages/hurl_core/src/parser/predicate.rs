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
use crate::parser::predicate_value::predicate_value;
use crate::parser::primitives::*;
use crate::parser::reader::Reader;
use crate::parser::ParseResult;

pub fn predicate(reader: &mut Reader) -> ParseResult<'static, Predicate> {
    let (not, space0) = predicate_not(reader);
    let func = predicate_func(reader)?;
    Ok(Predicate {
        not,
        space0,
        predicate_func: func,
    })
}

// can not fail
fn predicate_not(reader: &mut Reader) -> (bool, Whitespace) {
    let save = reader.state.clone();
    let no_whitespace = Whitespace {
        value: String::new(),
        source_info: SourceInfo {
            start: save.pos.clone(),
            end: save.pos.clone(),
        },
    };
    if try_literal("not", reader).is_ok() {
        match one_or_more_spaces(reader) {
            Ok(space) => (true, space),
            Err(_) => {
                reader.state = save;
                (false, no_whitespace)
            }
        }
    } else {
        (false, no_whitespace)
    }
}

fn predicate_func(reader: &mut Reader) -> ParseResult<'static, PredicateFunc> {
    let start = reader.state.clone().pos;
    let value = predicate_func_value(reader)?;
    let end = reader.state.clone().pos;
    Ok(PredicateFunc {
        source_info: SourceInfo { start, end },
        value,
    })
}

fn predicate_func_value(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    let start = reader.state.clone();
    match choice(
        &[
            equal_predicate,
            not_equal_predicate,
            greater_or_equal_predicate,
            greater_predicate,
            less_or_equal_predicate,
            less_predicate,
            start_with_predicate,
            end_with_predicate,
            contain_predicate,
            include_predicate,
            match_predicate,
            integer_predicate,
            float_predicate,
            boolean_predicate,
            string_predicate,
            collection_predicate,
            date_predicate,
            exist_predicate,
            is_empty_predicate,
        ],
        reader,
    ) {
        Err(Error {
            recoverable: true, ..
        }) => Err(Error {
            pos: start.pos,
            recoverable: false,
            inner: ParseError::Predicate,
        }),
        x => x,
    }
}

impl PredicateValue {
    pub fn is_number(&self) -> bool {
        matches!(self, PredicateValue::Integer(_) | PredicateValue::Float(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, PredicateValue::String(_))
    }

    pub fn is_bytearray(&self) -> bool {
        matches!(self, PredicateValue::Hex(_)) | matches!(self, PredicateValue::Base64(_))
    }

    pub fn is_expression(&self) -> bool {
        matches!(self, PredicateValue::Expression(_))
    }
}

fn equal_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    let operator = try_literals("equals", "==", reader)? == "==";
    if !operator {
        eprintln!("'equals' predicate is now deprecated. Use '==' instead");
    }
    let space0 = if operator {
        zero_or_more_spaces(reader)?
    } else {
        one_or_more_spaces(reader)?
    };
    let value = predicate_value(reader)?;
    Ok(PredicateFuncValue::Equal {
        space0,
        value,
        operator,
    })
}

fn not_equal_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    let operator = try_literals("notEquals", "!=", reader)? == "!=";
    if !operator {
        eprintln!("'notEquals' predicate is now deprecated. Use '!=' instead");
    }
    let space0 = if operator {
        zero_or_more_spaces(reader)?
    } else {
        one_or_more_spaces(reader)?
    };
    let value = predicate_value(reader)?;
    Ok(PredicateFuncValue::NotEqual {
        space0,
        value,
        operator,
    })
}

fn greater_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    let operator = try_literals("greaterThan", ">", reader)? == ">";
    if !operator {
        eprintln!("'greaterThan' predicate is now deprecated. Use '>' instead");
    }
    let space0 = if operator {
        zero_or_more_spaces(reader)?
    } else {
        one_or_more_spaces(reader)?
    };
    let start = reader.state.clone();
    let value = predicate_value(reader)?;
    if value.is_number() || value.is_string() || value.is_expression() {
        Ok(PredicateFuncValue::GreaterThan {
            space0,
            value,
            operator,
        })
    } else {
        Err(Error {
            pos: start.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        })
    }
}

fn greater_or_equal_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    let operator = try_literals("greaterThanOrEquals", ">=", reader)? == ">=";
    if !operator {
        eprintln!("'greaterThanOrEquals' predicate is now deprecated. Use '>=' instead");
    }
    let space0 = if operator {
        zero_or_more_spaces(reader)?
    } else {
        one_or_more_spaces(reader)?
    };
    let start = reader.state.clone();
    let value = predicate_value(reader)?;
    if value.is_number() || value.is_string() || value.is_expression() {
        Ok(PredicateFuncValue::GreaterThanOrEqual {
            space0,
            value,
            operator,
        })
    } else {
        Err(Error {
            pos: start.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        })
    }
}

fn less_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    let operator = try_literals("lessThan", "<", reader)? == "<";
    if !operator {
        eprintln!("'lessThan' predicate is now deprecated. Use '<' instead");
    }
    let space0 = if operator {
        zero_or_more_spaces(reader)?
    } else {
        one_or_more_spaces(reader)?
    };
    let start = reader.state.clone();
    let value = predicate_value(reader)?;
    if value.is_number() || value.is_string() || value.is_expression() {
        Ok(PredicateFuncValue::LessThan {
            space0,
            value,
            operator,
        })
    } else {
        Err(Error {
            pos: start.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        })
    }
}

fn less_or_equal_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    let operator = try_literals("lessThanOrEquals", "<=", reader)? == "<=";
    if !operator {
        eprintln!("'lessThanOrEquals' predicate is now deprecated. Use '<=' instead");
    }
    let space0 = if operator {
        zero_or_more_spaces(reader)?
    } else {
        one_or_more_spaces(reader)?
    };
    let start = reader.state.clone();
    let value = predicate_value(reader)?;
    if value.is_number() || value.is_string() || value.is_expression() {
        Ok(PredicateFuncValue::LessThanOrEqual {
            space0,
            value,
            operator,
        })
    } else {
        Err(Error {
            pos: start.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        })
    }
}

fn start_with_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("startsWith", reader)?;
    let space0 = one_or_more_spaces(reader)?;
    let save = reader.state.clone();
    let value = predicate_value(reader)?;
    if !value.is_string() && !value.is_bytearray() {
        return Err(Error {
            pos: save.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        });
    }
    Ok(PredicateFuncValue::StartWith { space0, value })
}

fn end_with_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("endsWith", reader)?;
    let space0 = one_or_more_spaces(reader)?;
    let save = reader.state.clone();
    let value = predicate_value(reader)?;
    if !value.is_string() && !value.is_bytearray() {
        return Err(Error {
            pos: save.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        });
    }
    Ok(PredicateFuncValue::EndWith { space0, value })
}

fn contain_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("contains", reader)?;
    let space0 = one_or_more_spaces(reader)?;
    let save = reader.state.clone();
    let value = predicate_value(reader)?;
    if !value.is_string() && !value.is_bytearray() {
        return Err(Error {
            pos: save.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        });
    }
    Ok(PredicateFuncValue::Contain { space0, value })
}

fn include_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("includes", reader)?;
    let space0 = one_or_more_spaces(reader)?;
    let value = predicate_value(reader)?;
    Ok(PredicateFuncValue::Include { space0, value })
}

fn match_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("matches", reader)?;
    let space0 = one_or_more_spaces(reader)?;
    let save = reader.state.clone();
    let value = predicate_value(reader)?;
    if !matches!(value, PredicateValue::String(_)) && !matches!(value, PredicateValue::Regex(_)) {
        return Err(Error {
            pos: save.pos,
            recoverable: false,
            inner: ParseError::PredicateValue,
        });
    }
    Ok(PredicateFuncValue::Match { space0, value })
}

fn integer_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("isInteger", reader)?;
    Ok(PredicateFuncValue::IsInteger)
}

fn float_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("isFloat", reader)?;
    Ok(PredicateFuncValue::IsFloat)
}

fn boolean_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("isBoolean", reader)?;
    Ok(PredicateFuncValue::IsBoolean)
}

fn string_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("isString", reader)?;
    Ok(PredicateFuncValue::IsString)
}

fn collection_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("isCollection", reader)?;
    Ok(PredicateFuncValue::IsCollection)
}

fn date_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("isDate", reader)?;
    Ok(PredicateFuncValue::IsDate)
}

fn exist_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("exists", reader)?;
    Ok(PredicateFuncValue::Exist)
}

fn is_empty_predicate(reader: &mut Reader) -> ParseResult<'static, PredicateFuncValue> {
    try_literal("isEmpty", reader)?;
    Ok(PredicateFuncValue::IsEmpty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Pos;

    #[test]
    fn test_predicate_not() {
        let mut reader = Reader::new("notXX");
        assert_eq!(
            predicate_not(&mut reader),
            (
                false,
                Whitespace {
                    value: String::new(),
                    source_info: SourceInfo::new(1, 1, 1, 1),
                }
            )
        );
        assert_eq!(reader.state.pos, Pos { line: 1, column: 1 });

        let mut reader = Reader::new("not XX");
        assert_eq!(
            predicate_not(&mut reader),
            (
                true,
                Whitespace {
                    value: String::from(" "),
                    source_info: SourceInfo::new(1, 4, 1, 5),
                }
            )
        );
        assert_eq!(reader.state.pos, Pos { line: 1, column: 5 });
    }

    #[test]
    fn test_predicate() {
        let mut reader = Reader::new("not equals true");
        assert_eq!(
            predicate(&mut reader).unwrap(),
            Predicate {
                not: true,
                space0: Whitespace {
                    value: String::from(" "),
                    source_info: SourceInfo::new(1, 4, 1, 5),
                },
                predicate_func: PredicateFunc {
                    source_info: SourceInfo::new(1, 5, 1, 16),
                    value: PredicateFuncValue::Equal {
                        space0: Whitespace {
                            value: String::from(" "),
                            source_info: SourceInfo::new(1, 11, 1, 12),
                        },
                        value: PredicateValue::Bool(true),
                        operator: false,
                    },
                },
            }
        );
    }

    #[test]
    fn test_predicate_func() {
        let mut reader = Reader::new("tata equals 1");
        let error = predicate_func(&mut reader).err().unwrap();
        assert_eq!(error.pos, Pos { line: 1, column: 1 });
        assert!(!error.recoverable);
        assert_eq!(error.inner, ParseError::Predicate);
    }

    #[test]
    fn test_equal_predicate() {
        let mut reader = Reader::new("equals  true");
        assert_eq!(
            equal_predicate(&mut reader).unwrap(),
            PredicateFuncValue::Equal {
                value: PredicateValue::Bool(true),
                space0: Whitespace {
                    value: String::from("  "),
                    source_info: SourceInfo::new(1, 7, 1, 9),
                },

                operator: false,
            }
        );

        let mut reader = Reader::new("equals 1.1");
        assert_eq!(
            equal_predicate(&mut reader).unwrap(),
            PredicateFuncValue::Equal {
                value: PredicateValue::Float(Float {
                    value: 1.1,
                    encoded: "1.1".to_string(),
                }),
                space0: Whitespace {
                    value: String::from(" "),
                    source_info: SourceInfo::new(1, 7, 1, 8),
                },
                operator: false,
            }
        );

        let mut reader = Reader::new("equals 2");
        assert_eq!(
            equal_predicate(&mut reader).unwrap(),
            PredicateFuncValue::Equal {
                value: PredicateValue::Integer(2),
                space0: Whitespace {
                    value: String::from(" "),
                    source_info: SourceInfo::new(1, 7, 1, 8),
                },
                operator: false,
            },
        );

        let mut reader = Reader::new("== 2");
        assert_eq!(
            equal_predicate(&mut reader).unwrap(),
            PredicateFuncValue::Equal {
                value: PredicateValue::Integer(2),
                space0: Whitespace {
                    value: String::from(" "),
                    source_info: SourceInfo::new(1, 3, 1, 4),
                },
                operator: true,
            },
        );

        let mut reader = Reader::new("equals \"Bob\"");
        assert_eq!(
            equal_predicate(&mut reader).unwrap(),
            PredicateFuncValue::Equal {
                value: PredicateValue::String(Template {
                    delimiter: Some('"'),
                    elements: vec![TemplateElement::String {
                        value: "Bob".to_string(),
                        encoded: "Bob".to_string(),
                    }],
                    source_info: SourceInfo::new(1, 8, 1, 13),
                }),
                space0: Whitespace {
                    value: String::from(" "),
                    source_info: SourceInfo::new(1, 7, 1, 8),
                },
                operator: false,
            }
        );
    }

    #[test]
    fn test_equal_expression_predicate() {
        let mut reader = Reader::new("equals {{count}}");
        assert_eq!(
            equal_predicate(&mut reader).unwrap(),
            PredicateFuncValue::Equal {
                value: PredicateValue::Expression(Expr {
                    space0: Whitespace {
                        value: String::new(),
                        source_info: SourceInfo::new(1, 10, 1, 10),
                    },
                    variable: Variable {
                        name: "count".to_string(),
                        source_info: SourceInfo::new(1, 10, 1, 15),
                    },
                    space1: Whitespace {
                        value: String::new(),
                        source_info: SourceInfo::new(1, 15, 1, 15),
                    },
                }),
                space0: Whitespace {
                    value: String::from(" "),
                    source_info: SourceInfo::new(1, 7, 1, 8),
                },
                operator: false,
            }
        );
    }

    #[test]
    fn test_start_with_predicate() {
        let mut reader = Reader::new("startsWith 2");
        let error = start_with_predicate(&mut reader).err().unwrap();
        assert_eq!(
            error.pos,
            Pos {
                line: 1,
                column: 12,
            }
        );
        assert!(!error.recoverable);
        assert_eq!(error.inner, ParseError::PredicateValue);
    }

    #[test]
    fn test_date_predicate() {
        let mut reader = Reader::new("isDate");
        let result = date_predicate(&mut reader);
        assert_eq!(result.unwrap(), PredicateFuncValue::IsDate);
    }
}
