// Copyright 2023-2025 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::num::TryFromIntError;
use std::string::FromUtf8Error;

use snafu::Snafu;

use crate::ast::BinaryOp;
use crate::eval::Value;

pub type Result<T> = std::result::Result<T, Error>;

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
    #[snafu(display("object property name isn't a variable"))]
    ObjectPropShorthandNotVar,
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
    #[snafu(display("expected at least {} arguments, got {}", minimum, got))]
    TooFewArgs{minimum: usize, got: usize},
    #[snafu(display(
        "can't apply '{}' to '{}' and '{}'",
        op_symbol(op),
        render_type(lhs),
        render_type(rhs),
    ))]
    InvalidOpTypes{op: BinaryOp, lhs: Value, rhs: Value},
    #[snafu(display("'break' can't be used outside of a loop"))]
    BreakOutsideLoop,
    #[snafu(display("'continue' can't be used outside of a loop"))]
    ContinueOutsideLoop,
    #[snafu(display("'return' can't be used outside of a function"))]
    ReturnOutsideFunction,
    #[snafu(display("'for' iterator must be a 'list', 'object' or 'string'"))]
    ForIterNotIterable,
    #[snafu(display("only 'list's, 'object's or 'string's can be indexed"))]
    ValueNotIndexable,
    #[snafu(display("only 'list's or 'object's can update indices"))]
    ValueNotIndexAssignable,
    #[snafu(display("only 'list's can update range indices"))]
    ValueNotRangeIndexAssignable,
    #[snafu(display("type properties cannot be assigned to"))]
    AssignToTypeProp,
    #[snafu(display("index '{}' is outside the string bounds", index))]
    OutOfStringBounds{index: usize},
    #[snafu(display("index '{}' is outside the list bounds", index))]
    OutOfListBounds{index: usize},
    #[snafu(display("range [{}:{}] is outside the string bounds", start, end))]
    RangeOutOfStringBounds{start: usize, end: usize},
    // TODO Update usage of `RangeOutOfListBounds` to use more specific
    // variants, like `RangeStartOutOfListBounds`.
    #[snafu(display("range [{}:{}] is outside the list bounds", start, end))]
    RangeOutOfListBounds{start: usize, end: usize},
    #[snafu(display(
        "range start ({}) is greater than list length ({})",
        start,
        list_len,
    ))]
    RangeStartOutOfListBounds{start: usize, list_len: usize},
    #[snafu(display(
        "range end ({}) must be greater than range start ({})",
        end,
        start,
    ))]
    RangeStartNotBeforeEnd{start: usize, end: usize},
    #[snafu(display(
        "range end ({}) is greater than list length ({})",
        end,
        list_len,
    ))]
    RangeEndOutOfListBounds{end: usize, list_len: usize},
    #[snafu(display("only 'list's or 'string's can be range-indexed"))]
    ValueNotRangeIndexable,
    #[snafu(display("index can't be negative"))]
    NegativeIndex{index: i64},
    #[snafu(display("cannot collect 'list' items outside a destructure"))]
    ListCollectOutsideDestructure,
    #[snafu(display("cannot collect 'object' items outside a destructure"))]
    ObjectCollectOutsideDestructure,
    #[snafu(display("only the last item in the destructure can collect"))]
    ObjectCollectIsNotLast,
    #[snafu(display(
        "only lists can be spread in lists, got '{}'",
        render_type(value),
    ))]
    SpreadNonListInList{value: Value},
    #[snafu(display(
        "only objects can be spread in objects, got '{}'",
        render_type(value),
    ))]
    SpreadNonObjectInObject{value: Value},
    #[snafu(display(
        "only 'list's or 'string's can be assigned to range indexes, got '{}'",
        render_type(value),
    ))]
    RangeIndexAssignOnNonIndexable{value: Value},
    #[snafu(display(
        "only objects can be destructured into objects, got '{}'",
        render_type(value),
    ))]
    ObjectDestructureOnNonObject{value: Value},
    #[snafu(display("can't use spread operator in object destructuring"))]
    SpreadOnObjectDestructure,
    #[snafu(display(
        "only lists can be destructured into lists, got '{}'",
        render_type(value),
    ))]
    ListDestructureOnNonList{value: Value},
    #[snafu(display(
        "cannot bind {} item(s) to {} variable name(s)",
        rhs_len,
        lhs_len,
    ))]
    ListDestructureItemMismatch{lhs_len: usize, rhs_len: usize},
    #[snafu(display(
        "cannot bind {} item(s) to {} variable name(s)",
        rhs_len,
        lhs_len,
    ))]
    ListCollectTooFew{lhs_len: usize, rhs_len: usize},
    #[snafu(display(
        "cannot use spread operator (at index {}) of list destructure",
        index,
    ))]
    SpreadInListDestructure{index: usize},
    #[snafu(display(
        "cannot bind {} item(s) to {} index(s)",
        rhs_len,
        range_len,
    ))]
    RangeIndexItemMismatch{range_len: usize, rhs_len: usize},
    #[snafu(display("object doesn't contain property '{}'", name))]
    PropNotFound{name: String},
    #[snafu(display(
        "there is no type function '{}' for '{}'",
        name,
        render_type(value),
    ))]
    TypeFunctionNotFound{value: Value, name: String},
    #[snafu(display("cannot access type function on 'null'"))]
    TypeFunctionOnNull,
    #[snafu(display(
        "properties can only be accessed on objects, got '{}'",
        render_type(value),
    ))]
    PropAccessOnNonObject{value: Value},
    #[snafu(display(
        "interpolated values can only be strings, got '{}'",
        render_type(value),
    ))]
    InterpolatedValueNotString{value: Value},
    #[snafu(display("couldn't parse interpolation slot: {}", source_str))]
    InterpolateStringParseFailed{source_str: String},
    #[snafu(display("'{}' is not defined", name))]
    OpOnUndefinedIndex{name: String},
    #[snafu(display("'{}' is not defined", name))]
    OpOnUndefinedProp{name: String},
    #[snafu(display("cannot perform this operation on a range-index"))]
    OpOnRangeIndex,
    #[snafu(display("cannot perform this operation on an object destructure"))]
    OpOnObjectDestructure,
    #[snafu(display("cannot perform this operation on an list destructure"))]
    OpOnListDestructure,
    #[snafu(display(
        "'{} {} {}' caused an integer overflow",
        lhs,
        op_symbol(op),
        rhs,
    ))]
    IntOverflow{op: BinaryOp, lhs: i64, rhs: i64},
    #[snafu(display("'{}' is already declared at [{}:{}]", name, line, col))]
    DupParamName{name: String, line: usize, col: usize},
    #[snafu(display("can't use spread operator in parameter list"))]
    PropSpreadInParamList,
    #[snafu(display("can't use spread operator in parameter list"))]
    ItemSpreadInParamList,

    #[snafu(display("{}", msg))]
    BuiltinFuncErr{msg: String},

    #[snafu(display("dev error: {}", msg))]
    Dev{msg: String},

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

    CastFailed{source: TryFromIntError},

    BindFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    BindObjectCollectFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    BindObjectSingleFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    BindObjectPairFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    BindListItemFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    BindNextFailed{
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
    EvalIfConditionFailed{
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
    EvalWhileConditionFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalWhileStatementsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalForIterFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    ConvertForIterToPairsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalForStatementsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    ValidateArgsFailed{
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
    BinOpAssignListIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    BinOpAssignObjectIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    BinOpAssignPropFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalListItemsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalListItemFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalSourceExprFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalIndexToI64Failed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStringIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalListIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalObjectIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalObjectPropFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStartIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalEndIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStringRangeIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalListRangeIndexFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalRangeStartFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalRangeEndFailed{
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
    EvalCallFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
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
    EvalPropFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    InterpolateStringFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    InterpolateStringEvalExprFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    AssertArgsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    AssertThisFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    AssertNoThisFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    AssertStrFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
}

pub fn render_type(v: &Value) -> String {
    let s =
        match v {
            Value::Null => "null",

            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Str(_) => "string",

            Value::List(_) => "list",
            Value::Object(_) => "object",

            Value::BuiltinFunc{..} | Value::Func{..} => "func",
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

            BinaryOp::RefEq => "===",
            BinaryOp::RefNe => "!==",
        };

    s.to_string()
}
