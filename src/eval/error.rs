// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::string::FromUtf8Error;

use snafu::Snafu;

use ast::BinaryOp;
use eval::Value;

// TODO Ideally `Error` would be defined in `src/eval/mod.rs`, since these are
// errors that occur during evaluation. However, we define it here because
// `value::Value::BuiltinFunc` refers to it. We could make the error type for
// `value::Value::BuiltinFunc` generic, but this generic type would spread
// throughout the codebase for little benefit, so we take the current approach
// for now.
#[derive(Clone, Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    // TODO Consider adding a rendered version of the source expression to
    // highlight what the interpreter attempted to evaluate.
    #[snafu(display("can't call '{}' as a function", render_type(v)))]
    CannotCallNonFunc{v: Value},
    #[snafu(display("'{}' is not defined", name))]
    Undefined{name: String},
    #[snafu(display("cannot bind to {}", descr))]
    InvalidBindTarget{descr: String},
    // TODO Add the location of the previous definition of this name.
    #[snafu(display("'{}' is bound multiple times in this binding", name))]
    AlreadyInBinding{name: String},
    #[snafu(display(
        "'{}' is already defined in the current scope at [{}:{}]",
        name,
        prev_line,
        prev_col,
    ))]
    AlreadyInScope{name: String, prev_line: usize, prev_col: usize},
    #[snafu(display(
        "{} must be '{}', got '{}'",
        descr,
        exp_type,
        render_type(value),
    ))]
    IncorrectType{descr: String, exp_type: String, value: Value},
    #[snafu(display("couldn't create {} string: {}", descr, source))]
    StringConstructionFailed{source: FromUtf8Error, descr: String},
    #[snafu(display("expected {} arguments, got {}", need, got))]
    ArgNumMismatch{need: usize, got: usize},
    #[snafu(display(
        "can't apply '{}' to '{}' and '{}'",
        op_symbol(op),
        render_type(lhs),
        render_type(rhs),
    ))]
    InvalidOpTypes{op: BinaryOp, lhs: Value, rhs: Value},
    #[snafu(display("the LHS of an operation-assignment must be a variable"))]
    OpAssignLhsNotVar,
    #[snafu(display("'return' can't be used outside of a function"))]
    ReturnOutsideFunction,

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
    EvalDeclarationRhsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    DeclarationBindFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalAssignmentRhsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    AssignmentBindFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    OpAssignmentBindFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalConditionFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalIfStatementsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalElseStatementsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    DeclareFunctionFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalReturnExprFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalBlockFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalBinOpLhsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalBinOpRhsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    ApplyBinOpFailed{
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
    EvalBuiltinFuncCallFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
        func_name: Option<String>,
        call_loc: (usize, usize),
    },
    EvalFuncCallFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
        func_name: Option<String>,
        call_loc: (usize, usize),
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

            Value::BuiltinFunc{..} | Value::Func{..} => "function",
        };

    s.to_string()
}

fn op_symbol(op: &BinaryOp) -> String {
    let s =
        match op {
            BinaryOp::Sum => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",

            BinaryOp::And => "&&",
            BinaryOp::Or => "||",

            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Gt => ">",
            BinaryOp::Gte => ">=",
            BinaryOp::Lt => "<",
            BinaryOp::Lte => "<=",
        };

    s.to_string()
}
