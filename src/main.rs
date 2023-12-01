// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

#[cfg(test)]
extern crate assert_matches;
extern crate snafu;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Error as IoError;
use std::path::Path;
use std::path::PathBuf;
use std::process;

mod ast;
mod builtins;
mod eval;
mod lexer;

use snafu::OptionExt;
use snafu::ResultExt;
use snafu::Snafu;

use ast::Expr;
use builtins::fns;
use builtins::type_methods;
use eval::builtins::Builtins;
use eval::EvaluationContext;
use eval::value;
use eval::value::Error as EvalError;
use eval::value::ScopeStack;
use lexer::Lexer;
use parser::ProgParser;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(clippy::pedantic)]
    #[allow(dead_code)]
    parser
);

fn main() {
    if let Err(e) = f() {
        match e {
            Error::ScriptArgMissing => {
                eprintln!("missing script argument");
            },
            Error::GetCurrentDirFailed{source} => {
                eprintln!("couldn't get current directory: {}", source);
            },
            Error::ReadScriptFailed{path, source} => {
                let p = path.to_string_lossy();
                eprintln!("couldn't read script at '{}': {}", p, source);
            },
            Error::EvalFailed{source} => {
                eprintln!("runtime error: {}", source);
            },
        }
        process::exit(1);
    }
}

fn f() -> Result<(), Error> {
    let mut args = std::env::args();
    let _prog = args.next()
        .expect("couldn't get program name");
    let raw_cur_rel_script_path = args.next()
        .context(ScriptArgMissing)?;
    let cur_rel_script_path = Path::new(&raw_cur_rel_script_path);

    let cur_script_dir = env::current_dir()
        .context(GetCurrentDirFailed)?;
    let mut cur_script_path = cur_script_dir.clone();
    cur_script_path.push(cur_rel_script_path);

    let src = fs::read_to_string(&cur_script_path)
        .context(ReadScriptFailed{path: cur_script_path})?;

    let global_bindings = vec![
        (
            Expr::Var{name: "print".to_string()},
            value::new_built_in_func(fns::print),
        ),
    ];

    let mut scopes = ScopeStack::new(vec![]);
    let lexer = Lexer::new(&src);
    let ast = ProgParser::new().parse(lexer).unwrap();
    eval::eval_prog(
        &EvaluationContext{
            builtins: &Builtins{
                std: HashMap::new(),
                type_methods: type_methods::type_methods(),
            },
            global_bindings: &global_bindings,
            cur_script_dir,
        },
        &mut scopes,
        global_bindings.clone(),
        &ast,
    )
        .context(EvalFailed)?;

    Ok(())
}

#[derive(Debug, Snafu)]
enum Error {
    ScriptArgMissing,
    GetCurrentDirFailed{source: IoError},
    ReadScriptFailed{path: PathBuf, source: IoError},
    EvalFailed{source: EvalError},
}
