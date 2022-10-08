// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Re-export everything from `src/test_utils.rs`.
pub use sqlparser::test_utils::*;

// For the test-only macros we take a different approach of keeping them here
// rather than in the library crate.
//
// This is because we don't need any of them to be shared between the
// integration tests (i.e. `tests/*`) and the unit tests (i.e. `src/*`),
// but also because Rust doesn't scope macros to a particular module
// (and while we export internal helpers as sqlparser::test_utils::<...>,
// expecting our users to abstain from relying on them, exporting internal
// macros at the top level, like `sqlparser::nest` was deemed too confusing).

#[macro_export]
macro_rules! nest {
    ($base:expr $(, $join:expr)*) => {
        TableFactor::NestedJoin { table_with_joins: Box::new(TableWithJoins {
            relation: $base,
            joins: vec![$(join($join)),*]
        }), alias: None}
    };
}


#[cfg(test)]
pub mod testfile {
    use std::fs::{File, OpenOptions};
    use std::io::{BufRead, BufReader, Lines};
    use serde_lexpr;
    use sqlparser::ast::Statement;
    use sqlparser::parser::ParserError;

    pub struct TestCase {
        pub sql: String,
        pub canonical: String,
        pub expected: Result<Statement, ParserError>
    }

    impl TestCase {
        fn new(sql: &str, canonical: &str, result: &str, line_start: u32, line_end: u32) -> Self {
            Self {
                sql: String::from(sql),
                canonical: String::from(canonical.trim()),
                expected: serde_lexpr::from_str(result).unwrap_or_else(
                    |e| panic!(
                        "Deserialize error in block from line {} to {}.\nERROR: {}{}",
                        line_start, line_end, e, result
                    )
                ),
            }
        }
    }

    pub struct TestFileReader {
        line_number: u32,
        lines: Lines<BufReader<File>>,
    }

    impl TestFileReader {
        pub fn new(testfile_path: &str) -> Self {
            let testfile = OpenOptions::new().read(true).open(testfile_path)
                .unwrap_or_else(|_| panic!("Can't open file {} error!", testfile_path));
            Self {
                line_number: 0,
                lines: BufReader::new(testfile).lines(),
            }
        }
    }

    impl Iterator for TestFileReader {
        type Item = TestCase;

        fn next(&mut  self) -> Option<Self::Item> {
            let mut sql = String::new();
            let mut ast = String::new();
            let mut canonical = String::new();

            let mut mode: u8 = 0;
            let line_start = self.line_number;
            loop {
                self.line_number += 1;
                match self.lines.next() {
                    Some(Ok(line_str)) if line_str.starts_with("#") | line_str.is_empty() => {
                        // ignore comments and blank line
                        continue;
                    },
                    Some(Ok(line_str)) if line_str.starts_with("--") => {
                        // separator between sql, ast and  canonical
                        if mode < 2 {
                            mode += 1;
                        }
                    },
                    Some(Ok(line_str)) if line_str.starts_with("==") => {
                        // separator between two testcases
                        break;
                    },
                    Some(Ok(line_str)) => {
                        if mode == 0 {
                            sql.push('\n');
                            sql.push_str(&line_str);
                        }
                        else if mode == 1 {
                            ast.push('\n');
                            ast.push_str(&line_str);
                        }
                        else {
                            let line_str = line_str.trim();
                            if line_str.starts_with(")") && canonical.chars().last().unwrap() == ' ' {
                                canonical.pop();
                            }
                            canonical.push_str(&line_str);
                            if !line_str.ends_with("(") {
                                canonical.push(' ');
                            }
                        }
                    },
                    Some(Err(_)) => {
                        panic!("Error while reading testfile");
                    },
                    None => {
                        if mode < 2 {
                            return None;
                        }
                        else {
                            break;
                        }
                    },
                }
            }

            Some(TestCase::new(&sql, &canonical, &ast, line_start, self.line_number))
        }
    }
}
