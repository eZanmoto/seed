// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::string::FromUtf8Error;

use snafu::Snafu;

use eval::Value;

// TODO Ideally `Error` would be defined in `src/eval/mod.rs`, since these are
// errors that occur during evaluation. However, we define it here because
// `value::Value::BuiltInFunc` refers to it. We could make the error type for
// `value::Value::BuiltInFunc` generic, but this generic type would spread
// throughout the codebase for little benefit, so we take the current approach
// for now.
#[derive(Clone, Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    // TODO Consider adding a rendered version of the source expression to
    // highlight what the interpreter attempted to evaluate.
    #[snafu(display("value is not a function"))]
    CannotCallNonFunc{v: Value},
    #[snafu(display("'{}' is not defined", name))]
    Undefined{name: String},
    #[snafu(display("cannot bind to {}", descr))]
    InvalidBindTarget{descr: String},
    #[snafu(display("'{}' is bound multiple times in this binding", name))]
    AlreadyInBinding{name: String},
    #[snafu(display("'{}' is already defined in the current scope", name))]
    AlreadyInScope{name: String},
    #[snafu(display(
        "{} must be '{}', got '{}'",
        descr,
        exp_type,
        render_type(value),
    ))]
    IncorrectType{descr: String, exp_type: String, value: Value},
    #[snafu(display("couldn't create {} string: {}", descr, source))]
    StringConstructionFailed{source: FromUtf8Error, descr: String},

    #[snafu(display("{}", msg))]
    BuiltinFuncErr{msg: String},

    // NOTE This is a somewhat hacky way of adding location information to
    // errors in a generic way. Ideally this information could be better
    // decoupled from the core error type, but we take this approach for now
    // for simplicity.
    #[snafu(display("{}:{}: {}", line, col, source))]
    AtLoc{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
        line: usize,
        col: usize,
    },

    BindFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalProgFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtsInNewScopeFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtsWithScopeStackFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalListItemFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalPropNameFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalPropValueFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
        name: String,
    },
    EvalCallArgsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalCallFuncFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalExprFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
}

fn render_type(v: &Value) -> String {
    let s =
        match v {
            Value::Null => "null",

            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Str(_) => "string",

            Value::List(_) => "list",
            Value::Object(_) => "object",

            Value::BuiltInFunc{..} => "function",
        };

    s.to_string()
}
