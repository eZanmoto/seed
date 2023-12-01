// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::env;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

#[macro_use]
extern crate indoc;
extern crate lalrpop;

// TODO This is the first, functional version of this file, which hasn't been
// refactored with best practices (e.g. dependency injection, and verifyng
// abstractions). This should be done before any additional work is applied to
// the file.
fn main() {
    lalrpop::process_root().unwrap();

    let raw_tgt_dir = env::var("OUT_DIR").unwrap();
    let tgt_dir = Path::new(&raw_tgt_dir);

    // TODO Consider returning an error from `gen_tests` instead of `panic`ing.
    gen_tests("tests/stdout", tgt_dir);
}

fn gen_tests(src_dir: &str, tgt_dir: &Path) {
    let tgt_file = tgt_dir.join("tests.rs");
    let mut test_file = File::create(&tgt_file)
        .expect("couldn't create test file");

    let test_dir = tgt_dir.join("tests");
    // TODO Consider removing old test directories on each run.

    // TODO Consider creating the test directory when running generated tests,
    // as opposed to the time of generating the tests.
    if let Err(e) = fs::create_dir(&test_dir) {
        if e.kind() != ErrorKind::AlreadyExists {
            panic!("couldn't create test directory: {}", e);
        }
    }

    write_test_file_header(&mut test_file);

    let entries = fs::read_dir(src_dir)
        .expect("couldn't read test directory");

    for maybe_entry in entries {
        let entry = maybe_entry
            .expect("couldn't read test directory entry");

        let file_type = entry.file_type()
            .expect("couldn't get file type");

        let entry_path = entry.path();
        if !file_type.is_file() {
            panic!("'{}' isn't a file", entry_path.display());
        }

        if let Some(ext) = entry_path.extension() {
            if ext != "test" && ext != "xtest" {
                continue;
            }

            let entry_stem_raw = entry_path.file_stem()
                .expect("couldn't extract file stem from path");

            let entry_stem = entry_stem_raw.to_str()
                .expect("file stem contains invalid UTF-8");

            writedoc!(
                test_file,
                "
                    mod {mod_name} {{
                        #[allow(clippy::wildcard_imports)]
                        use super::*;
                ",
                mod_name = entry_stem,
            )
                .expect("couldn't write test file module start");

            let is_extended_test = ext == "xtest";
            for test in extract_tests(entry_path.clone(), is_extended_test) {
                write_test(&mut test_file, &test_dir, entry_stem, &test);
            }

            write!(test_file, "\n}}\n")
                .expect("couldn't write test file module end");
        }
    }
}

fn extract_tests(entry_path: PathBuf, extended_format: bool) -> Vec<Test> {
    let f = File::open(entry_path)
        .expect("couldn't open test file");

    let mut tests = vec![];
    let mut end_matched = false;
    let mut cur_test: Option<Test> = None;
    let mut test_section = 0;
    for maybe_line in BufReader::new(f).lines() {
        let line = maybe_line
            .expect("couldn't read line from test file");

        if end_matched {
            panic!("extra lines discovered after closing test marker");
        }

        let suffix =
            if let Some(suf) = line.strip_prefix(TEST_MARKER_START) {
                suf
            } else if line == TEST_MARKER_SECTION {
                test_section += 1;
                continue;
            } else {
                let mut test = cur_test.take()
                    .expect("lines discovered before first test marker");

                #[allow(clippy::collapsible_else_if)]
                if extended_format {
                    if test_section == 0 {
                        let value = line.strip_prefix("exit_code: ")
                            .expect("missing 'exit_code' key");

                        test.tgt_code = value.parse()
                            .expect("couldn't parse exit code as `i32`");
                    } else if test_section == 1 {
                        test.src += &(line + "\n");
                    } else if test_section == 2 {
                        test.tgt_stdout += &(line + "\n");
                    } else if test_section == 3 {
                        test.tgt_stderr += &(line + "\n");
                    } else {
                        panic!("too many sections defined for extended test");
                    }
                } else {
                    if test_section == 0 {
                        test.src += &(line + "\n");
                    } else if test_section == 1 {
                        test.tgt_stdout += &(line + "\n");
                    } else {
                        panic!("too many sections defined for test");
                    }
                }

                cur_test.replace(test);
                continue;
            };

        if suffix.is_empty() {
            if let Some(t) = cur_test.take() {
                tests.push(t);
            } else {
                panic!("no tests defined");
            }

            end_matched = true;
            continue;
        }

        let test_name =
            if let Some(suf) = suffix.strip_prefix(' ') {
                suf
            } else {
                panic!("expected space before test name");
            };

        if let Some(t) = cur_test.take() {
            if (extended_format && test_section < 3)
                    || (!extended_format && test_section < 1) {

                panic!("expected output not defined for test '{}'", t.name);
            }

            tests.push(t);
        }

        cur_test = Some(Test{
            name: String::from(test_name),
            src: String::from(""),
            tgt_code: 0,
            tgt_stdout: String::from(""),
            tgt_stderr: String::from(""),
        });
        test_section = 0;
    }

    if !end_matched {
        panic!("test file didn't end with closing test marker");
    }

    tests
}

#[derive(Clone)]
struct Test {
    name: String,
    src: String,
    tgt_code: i32,
    tgt_stdout: String,
    tgt_stderr: String,
}

const TEST_MARKER_START: &str =
    "==================================================";

const TEST_MARKER_SECTION: &str =
    "--------------------------------------------------";

fn write_test_file_header(test_file: &mut File) {
    let header = indoc!{"
        use std::fs;
        use std::path::Path;

        use crate::assert_cmd::Command;

        struct Test {
            src: String,
            exp: TestExpectation,
        }

        struct TestExpectation {
            code: i32,
            stdout: String,
            stderr: String,
        }

        fn run_test(test_dir: &str, test_file_path: &str, test: Test) {
            let Test{src, exp} = test;

            let path = Path::new(test_dir).join(test_file_path);
            fs::write(&path, src)
                .unwrap_or_else(|_| {
                    panic!(\"couldn't create test file '{}'\", path.display());
                });

            let mut cmd = Command::cargo_bin(env!(\"CARGO_PKG_NAME\")).unwrap();
            let assert = cmd
                .current_dir(test_dir)
                .arg(test_file_path)
                .assert();

            assert
                .code(exp.code)
                .stdout(exp.stdout)
                .stderr(exp.stderr);
        }
    "};
    write!(test_file, "{}", header)
        .expect("couldn't write test file header");
}

fn write_test(
    test_file: &mut File,
    root_test_dir: &Path,
    file_test_dir_name: &str,
    test: &Test,
) {
    let file_test_dir = root_test_dir.join(file_test_dir_name);

    fs::create_dir_all(&file_test_dir)
        .expect("couldn't create directories for file tests");

    let test_file_path =
        Path::new(file_test_dir_name).join(test.name.clone() + ".sd");

    // TODO Indent rendered code.
    write!(
        test_file,
        indoc!{"

            #[test]
            fn {name}() {{
                run_test(
                    \"{test_dir}\",
                    \"{test_file_path}\",
                    Test{{
                        src: String::from(r#\"{src}\"#),
                        exp: TestExpectation{{
                            code: {tgt_code},
                            stdout: String::from(r#\"{tgt_stdout}\"#),
                            stderr: String::from(r#\"{tgt_stderr}\"#),
                        }},
                    }}
                );
            }}
        "},
        name = test.name,
        test_dir = root_test_dir.display(),
        test_file_path = test_file_path.display(),
        src = test.src,
        tgt_code = test.tgt_code,
        tgt_stdout = test.tgt_stdout,
        tgt_stderr = test.tgt_stderr,
    )
        .unwrap_or_else(|_| panic!(
            "couldn't write test to test file '{:?}'",
            test_file_path,
        ));
}
