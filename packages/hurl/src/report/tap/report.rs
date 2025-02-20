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

use crate::report::Error;

use super::Testcase;
use std::fs::File;
use std::io::Write;

/// Creates/Append a Tap report from a list of `testcases`
pub fn write_report(filename: &str, new_testcases: &[Testcase]) -> Result<(), Error> {
    eprintln!("write tap report {filename}");
    let mut testcases = vec![];

    let existing_testcases = parse_tap_file(filename)?;
    for testcase in existing_testcases.iter() {
        testcases.push(testcase);
    }
    for testcase in new_testcases {
        testcases.push(testcase);
    }
    write_tap_file(filename, &testcases)
}

/// Creates a Tap from a list of `testcases`.
fn write_tap_file(filename: &str, testcases: &[&Testcase]) -> Result<(), Error> {
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(e) => {
            return Err(Error {
                message: format!("Failed to produce TAP report: {e:?}"),
            });
        }
    };
    let start = 1;
    let end = testcases.len();
    let mut s = format!("{start}..{end}\n");
    for (i, testcase) in testcases.iter().enumerate() {
        let success = if testcase.success { "" } else { "not " };
        let number = i + 1;
        let description = &testcase.description;
        s.push_str(format!("{success}ok {number} - {description}\n").as_str());
    }
    match file.write_all(s.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => Err(Error {
            message: format!("Failed to write TAP report: {e:?}"),
        }),
    }
}

/// Parse Tap report file
fn parse_tap_file(filename: &str) -> Result<Vec<Testcase>, Error> {
    let path = std::path::Path::new(&filename);
    if path.exists() {
        let s = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(why) => {
                return Err(Error {
                    message: format!("Issue reading {} to string to {:?}", path.display(), why),
                });
            }
        };
        parse_tap_report(&s)
    } else {
        Ok(vec![])
    }
}

/// Parse Tap report
fn parse_tap_report(s: &str) -> Result<Vec<Testcase>, Error> {
    let mut testcases = vec![];
    let mut lines: Vec<&str> = s.lines().collect::<Vec<&str>>();
    if !lines.is_empty() {
        let header = lines.remove(0);
        let header_tokens = header.split("..").collect::<Vec<&str>>();
        match header_tokens.first() {
            None => {
                return Err(Error {
                    message: format!("Invalid TAP Header <{header}>"),
                });
            }
            Some(value) => match value.parse::<usize>() {
                Ok(value) => value,
                Err(_) => {
                    return Err(Error {
                        message: format!("Invalid TAP Header <{header}>"),
                    })
                }
            },
        };
        match header_tokens.get(1) {
            None => {
                return Err(Error {
                    message: format!("Invalid TAP Header <{header}>"),
                });
            }
            Some(value) => match value.parse::<usize>() {
                Ok(value) => value,
                Err(_) => {
                    return Err(Error {
                        message: format!("Invalid TAP Header <{header}>"),
                    })
                }
            },
        };
        for line in lines {
            let line = line.trim();
            if !line.is_empty() {
                let testcase = Testcase::parse(line)?;
                testcases.push(testcase);
            }
        }
    }
    Ok(testcases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tap_report() {
        let s = r#"1..3
ok 1 - tests_ok/test.1.hurl
 ok 2  -tests_ok/test.2.hurl
nok 3 - tests_ok/test.3.hurl

"#;
        assert_eq!(
            parse_tap_report(s).unwrap(),
            vec![
                Testcase {
                    description: "tests_ok/test.1.hurl".to_string(),
                    success: true
                },
                Testcase {
                    description: "tests_ok/test.2.hurl".to_string(),
                    success: true
                },
                Testcase {
                    description: "tests_ok/test.3.hurl".to_string(),
                    success: false
                }
            ]
        )
    }
}
