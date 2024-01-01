// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::PathBuf;

pub mod bind;
pub mod builtins;
pub mod error;
pub mod scope;
pub mod value;

use snafu::ResultExt;

#[allow(clippy::wildcard_imports)]
use ast::*;
use self::bind::BindType;
use self::builtins::Builtins;
// We use a wildcard import for `error` to import the many error variant
// constructors created by Snafu.
#[allow(clippy::wildcard_imports)]
use self::error::*;
use self::error::Error;
use self::scope::ScopeStack;
use self::value::BuiltinFunc;
use self::value::List;
use self::value::ValRefWithSource;
use self::value::Value;
use self::value::ValWithSource;

macro_rules! match_eval_expr {
    (
        ( $context:ident, $scopes:ident, $expr:expr )
        { $( $key:pat => $value:expr , )* }
    ) => {{
        let value = eval_expr($context, $scopes, $expr)
            .context(EvalExprFailed)?;
        let unlocked_value = &mut (*value.lock().unwrap()).v;
        match unlocked_value {
            $( $key => $value , )*
        }
    }};
}

pub struct EvaluationContext<'a> {
    pub builtins: &'a Builtins,
    pub cur_script_dir: PathBuf,

    // `global_bindings` are added to the global scope when the program starts.
    //
    // TODO Consider grouping `global_bindings` with `builtins`.
    pub global_bindings: &'a Vec<(RawExpr, ValRefWithSource)>,
}

pub fn eval_prog(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    global_bindings: Vec<(RawExpr, ValRefWithSource)>,
    Prog::Body{stmts}: &Prog,
)
    -> Result<(), Error>
{
    let bindings =
        global_bindings
            .into_iter()
            .map(|(raw_expr, v)| ((raw_expr, (0, 0)), v))
            .collect();

    let v = eval_stmts(context, scopes, bindings, stmts)
        .context(EvalStmtsFailed)?;

    match v {
        Escape::None => Ok(()),
        Escape::Break{loc} => {
            let (line, col) = loc;

            Err(Error::AtLoc{
                source: Box::new(Error::BreakOutsideLoop),
                line,
                col,
            })
        },
        Escape::Continue{loc} => {
            let (line, col) = loc;

            Err(Error::AtLoc{
                source: Box::new(Error::ContinueOutsideLoop),
                line,
                col,
            })
        },
        Escape::Return{loc, ..} => {
            let (line, col) = loc;

            Err(Error::AtLoc{
                source: Box::new(Error::ReturnOutsideFunction),
                line,
                col,
            })
        },
    }
}

// `eval_stmts` evaluates `stmts` in a new scope pushed onto `scopes`, with the
// given `new_bindings` declared in the new scope.
pub fn eval_stmts(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    new_bindings: Vec<(Expr, ValRefWithSource)>,
    stmts: &Block,
)
    -> Result<Escape, Error>
{
    let mut inner_scopes = scopes.new_from_push(HashMap::new());

    for (lhs, rhs) in new_bindings {
        bind::bind(&mut inner_scopes, &lhs, rhs, BindType::Declaration)
            .context(BindFailed)?;
    }

    let v = eval_stmts_with_scope_stack(context, &mut inner_scopes, stmts)
        .context(EvalStmtsWithScopeStackFailed)?;

    Ok(v)
}

pub fn eval_stmts_with_scope_stack(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    stmts: &Block,
)
    -> Result<Escape, Error>
{
    for stmt in stmts {
        let v = eval_stmt(context, scopes, stmt)
            .context(EvalStmtFailed)?;

        match v {
            Escape::None => {},
            _ => return Ok(v),
        }
    }

    Ok(Escape::None)
}

pub enum Escape {
    None,
    Break{loc: Location},
    Continue{loc: Location},
    Return{value: ValRefWithSource, loc: Location},
}

#[allow(clippy::too_many_lines)]
fn eval_stmt(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    stmt: &Stmt,
)
    -> Result<Escape, Error>
{
    match stmt {
        Stmt::Block{block} => {
            eval_stmts_in_new_scope(context, scopes, block)
                .context(EvalBlockFailed)?;
        },

        Stmt::Expr{expr} => {
            eval_expr(context, scopes, expr)
                .context(EvalExprFailed)?;
        },

        Stmt::Declare{lhs, rhs} => {
            let v = eval_expr(context, scopes, rhs)
                .context(EvalDeclarationRhsFailed)?;

            bind::bind(scopes, lhs, v, BindType::Declaration)
                .context(DeclarationBindFailed)?;
        },

        Stmt::Assign{lhs, rhs} => {
            let v = eval_expr(context, scopes, rhs)
                .context(EvalAssignmentRhsFailed)?;

            bind::bind(scopes, lhs, v, BindType::Assignment)
                .context(AssignmentBindFailed)?;
        },

        Stmt::OpAssign{lhs, lhs_loc, op, op_loc, rhs} => {
            let (name, name_loc) =
                if let (RawExpr::Var{name}, name_loc) = lhs {
                    (name, name_loc)
                } else {
                    let (line, col) = lhs_loc;

                    return Err(Error::AtLoc{
                        source: Box::new(Error::OpAssignLhsNotVar),
                        line: *line,
                        col: *col,
                    })
                };

            let lhs_val = eval_expr(context, scopes, lhs)
                .context(EvalBinOpLhsFailed)?;

            let rhs_val = eval_expr(context, scopes, rhs)
                .context(EvalBinOpRhsFailed)?;

            // See comment above call to `apply_binary_operation` in
            // `eval_expr` for details on why we clone the value from inside
            // the `lhs` `Mutex`.
            let raw_v = apply_binary_operation(
                op,
                op_loc,
                &clone_value(&lhs_val),
                &rhs_val.lock().unwrap().v,
            )
                .context(ApplyBinOpFailed)?;

            let v = value::new_val_ref(raw_v);

            bind::bind_name(scopes, name, name_loc, v, BindType::Assignment)
                .context(OpAssignmentBindFailed)?;
        },

        Stmt::If{branches, else_stmts} => {
            for Branch{cond, stmts} in branches {
                let b = eval_expr_to_bool(context, scopes, "condition", cond)
                    .context(EvalIfConditionFailed)?;

                if b {
                    let v = eval_stmts_in_new_scope(context, scopes, stmts)
                        .context(EvalIfStatementsFailed)?;

                    return Ok(v);
                }
            }

            if let Some(stmts) = else_stmts {
                let v = eval_stmts_in_new_scope(context, scopes, stmts)
                    .context(EvalElseStatementsFailed)?;

                return Ok(v);
            }
        },

        Stmt::While{cond, stmts} => {
            loop {
                let b = eval_expr_to_bool(context, scopes, "condition", cond)
                    .context(EvalWhileConditionFailed)?;

                if !b {
                    break;
                }

                let escape = eval_stmts_in_new_scope(context, scopes, stmts)
                    .context(EvalWhileStatementsFailed)?;

                match escape {
                    Escape::None => {},
                    Escape::Break{..} => break,
                    Escape::Continue{..} => continue,
                    Escape::Return{..} => return Ok(escape),
                }
            }
        },

        Stmt::For{lhs, iter, stmts} => {
            let iter_val = eval_expr(context, scopes, iter)
                    .context(EvalForIterFailed)?;

            let pairs = value_to_pairs(&(*iter_val.lock().unwrap()).v)
                    .context(ConvertForIterToPairsFailed)?;

            for (key, value) in pairs {
                let pair = value::new_list(vec![key, value]);

                let new_bindings = vec![(lhs.clone(), pair)];

                let escape = eval_stmts(context, scopes, new_bindings, stmts)
                    .context(EvalForStatementsFailed)?;

                match escape {
                    Escape::None => {},
                    Escape::Break{..} => break,
                    Escape::Continue{..} => continue,
                    Escape::Return{..} => return Ok(escape),
                }
            }
        },

        Stmt::Break{loc} => {
            return Ok(Escape::Break{loc: *loc});
        },

        Stmt::Continue{loc} => {
            return Ok(Escape::Continue{loc: *loc});
        },

        Stmt::Func{name: (name, loc), args, stmts} => {
            let closure = scopes.clone();
            let func = value::new_func(
                Some(name.clone()),
                args.clone(),
                stmts.clone(),
                closure,
            );

            bind::bind_name(scopes, name, loc, func, BindType::Declaration)
                .context(DeclareFunctionFailed)?;
        },

        Stmt::Return{loc, expr} => {
            let v = eval_expr(context, scopes, expr)
                .context(EvalReturnExprFailed)?;

            return Ok(Escape::Return{value: v, loc: *loc});
        },
    }

    Ok(Escape::None)
}

// `value_to_pairs` returns the "index, value" pairs in `v`, if `v` represents
// an "iterable" type.
fn value_to_pairs(v: &Value)
    -> Result<Vec<(ValRefWithSource, ValRefWithSource)>, Error>
{
    let pairs =
        match v {
            Value::Str(s) =>
                s
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        // TODO Handle issues caused by casting.
                        (value::new_int(i as i64), value::new_str(vec![*c]))
                    })
                    .collect(),

            Value::List(items) =>
                items
                    .iter()
                    .enumerate()
                    .map(|(i, value)| {
                        // TODO Handle issues caused by casting.
                        (value::new_int(i as i64), value.clone())
                    })
                    .collect(),

            Value::Object(props) =>
                props
                    .iter()
                    .map(|(key, value)| {
                        (
                            value::new_str_from_string(key.to_string()),
                            value.clone(),
                        )
                    })
                    .collect(),

            _ =>
                return Err(Error::ForIterNotIterable),
        };

    Ok(pairs)
}

pub fn eval_stmts_in_new_scope(
    context: &EvaluationContext,
    outer_scopes: &mut ScopeStack,
    stmts: &Block,
)
    -> Result<Escape, Error>
{
    eval_stmts(context, outer_scopes, vec![], stmts)
}

#[allow(clippy::too_many_lines)]
fn eval_expr(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    expr: &Expr,
) -> Result<ValRefWithSource, Error> {
    let (raw_expr, (line, col)) = expr;
    let new_loc_error = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    match raw_expr {
        RawExpr::Null => Ok(value::new_null()),

        RawExpr::Bool{b} => Ok(value::new_bool(*b)),

        RawExpr::Int{n} => Ok(value::new_int(*n)),

        RawExpr::Str{s} => Ok(value::new_str_from_string(s.clone())),

        RawExpr::Var{name} => {
            let v =
                match scopes.get(name) {
                    Some(v) => v,
                    None => return new_loc_error(
                        Error::Undefined{name: name.clone()},
                    ),
                };

            Ok(v)
        },

        RawExpr::BinaryOp{op, op_loc, lhs, rhs} => {
            let lhs_val = eval_expr(context, scopes, lhs)
                .context(EvalBinOpLhsFailed)?;

            let rhs_val = eval_expr(context, scopes, rhs)
                .context(EvalBinOpRhsFailed)?;

            // We clone the value from inside the `lhs_val` `Mutex` in a
            // separate scope instead of using `&lhs_val.lock().unwrap().v`.
            // This is because, with the latter approach, if `lhs_val` and
            // `rhs_val` refer to the same `Mutex` value, the first lock on it
            // (`lhs_val.lock()`) will cause a deadlock when a second lock
            // (`rhs_val.lock()`) is attempted in the same scope.
            let v = apply_binary_operation(
                op,
                op_loc,
                &clone_value(&lhs_val),
                &rhs_val.lock().unwrap().v,
            )
                .context(ApplyBinOpFailed)?;

            Ok(value::new_val_ref(v))
        },

        RawExpr::List{items} => {
            let mut vals = vec![];

            for item in items {
                let v = eval_expr(context, scopes, item)
                    .context(EvalListItemFailed)?;

                vals.push(v);
            }

            Ok(value::new_list(vals))
        },

        RawExpr::Index{expr, location: locat} => {
            // We don't use `match_eval_expr` here because we need to maintain
            // a reference to `source` for use in object lookups.
            let source_val = eval_expr(context, scopes, expr)
                .context(EvalIndexExprFailed)?;

            let unlocked_source_val = &(*source_val.lock().unwrap()).v;

            match unlocked_source_val {
                Value::Str(s) => {
                    let i = eval_expr_to_i64(context, scopes, "index", locat)
                        .context(EvalStringIndexFailed)?;

                    if i < 0 {
                        return new_loc_error(
                            Error::NegativeStringIndex{index: i},
                        );
                    }

                    let index: usize = i.try_into()
                        .context(ConvertStringIndexToUsizeFailed)?;

                    let v =
                        match s.get(index) {
                            Some(v) => value::new_str(vec![*v]),
                            None => return new_loc_error(
                                Error::OutOfStringBounds{index},
                            ),
                        };

                    Ok(v)
                },

                Value::List(list) => {
                    let i = eval_expr_to_i64(context, scopes, "index", locat)
                        .context(EvalListIndexFailed)?;

                    if i < 0 {
                        return new_loc_error(
                            Error::NegativeListIndex{index: i},
                        );
                    }

                    let index: usize = i.try_into()
                        .context(ConvertListIndexToUsizeFailed)?;

                    let v =
                        match list.get(index) {
                            Some(v) => v.clone(),
                            None => return new_loc_error(
                                Error::OutOfListBounds{index},
                            ),
                        };

                    Ok(v)
                },

                Value::Object(props) => {
                    // TODO Consider whether non-UTF-8 strings can be used to
                    // perform key lookups on objects.
                    let key = eval_expr_to_str(context, scopes, "key", locat)
                        .context(EvalObjectIndexFailed)?;

                    let v =
                        match props.get(&key) {
                            Some(v) => {
                                let prop_val = &(*v.lock().unwrap()).v;

                                value::new_val_ref_with_source(
                                    prop_val.clone(),
                                    source_val.clone(),
                                )
                            },
                            None => {
                                return new_loc_error(Error::NoSuchKey{key});
                            },
                        };

                    Ok(v)
                },

                _ => {
                    new_loc_error(Error::ValueNotIndexable)
                },
            }
        },

        RawExpr::Object{props} => {
            let mut vals = BTreeMap::<String, ValRefWithSource>::new();

            for prop in props {
                match prop {
                    PropItem::Pair{name: name_expr, value} => {
                        let descr = "property name";
                        let name =
                            eval_expr_to_str(context, scopes, descr, name_expr)
                                .context(EvalPropNameFailed)?;

                        let v = eval_expr(context, scopes, value)
                            .context(EvalPropValueFailed{name: name.clone()})?;

                        vals.insert(name, v);
                    },
                }
            }

            Ok(value::new_object(vals))
        },

        RawExpr::Call{func, args} => {
            let arg_vals = eval_exprs(context, scopes, args)
                .context(EvalCallArgsFailed)?;

            let func_val = eval_expr(context, scopes, func)
                .context(EvalCallFuncFailed)?;

            let (func_name, v) =
                {
                    let ValWithSource{v, source} = &*func_val.lock().unwrap();
                    match v {
                        Value::BuiltinFunc{name, f} => {
                            let this = source.as_ref().cloned();

                            (
                                Some(name.clone()),
                                CallBinding::BuiltinFunc{
                                    f: *f,
                                    this,
                                    args: arg_vals,
                                },
                            )
                        },

                        Value::Func{name, args: arg_names, stmts, closure} => {
                            let need = arg_names.len();
                            let got = arg_vals.len();
                            if got != need {
                                return new_loc_error(
                                    Error::ArgNumMismatch{need, got}
                                );
                            }

                            let mut bindings: Vec<(Expr, ValRefWithSource)> =
                                arg_names
                                    .clone()
                                    .into_iter()
                                    .zip(arg_vals)
                                    .collect();

                            if let Some(this) = source {
                                // TODO Consider how to avoid creating a
                                // new AST variable node here.
                                bindings.push((
                                    (
                                        RawExpr::Var{name: "this".to_string()},
                                        (0, 0),
                                    ),
                                    this.clone(),
                                ));
                            }

                            (
                                name.clone(),
                                CallBinding::Func{
                                    bindings,
                                    closure: closure.clone(),
                                    stmts: stmts.clone(),
                                },
                            )
                        },

                        _ => {
                            return new_loc_error(
                                Error::CannotCallNonFunc{v: v.clone()},
                            );
                        },
                    }
                };

            let v =
                match v {
                    CallBinding::BuiltinFunc{f, this, args} => {
                        f(this, args)
                            .context(EvalBuiltinFuncCallFailed{
                                func_name,
                                call_loc: (*line, *col),
                            })?
                    },

                    CallBinding::Func{bindings, mut closure, stmts} => {
                        let v = eval_stmts(
                            context,
                            &mut closure,
                            bindings,
                            &stmts,
                        )
                            .context(EvalFuncCallFailed{
                                func_name,
                                call_loc: (*line, *col),
                            })?;

                        match v {
                            Escape::None =>
                                value::new_null(),
                            Escape::Break{..} =>
                                return Err(Error::BreakOutsideLoop),
                            Escape::Continue{..} =>
                                return Err(Error::ContinueOutsideLoop),
                            Escape::Return{value, ..} =>
                                value,
                        }
                    },
                };

            Ok(v)
        },
    }
}

enum CallBinding {
    BuiltinFunc{
        f: BuiltinFunc,
        this: Option<ValRefWithSource>,
        args: List,
    },
    Func{
        bindings: Vec<(Expr, ValRefWithSource)>,
        closure: ScopeStack,
        stmts: Block,
    },
}

fn clone_value(v: &ValRefWithSource) -> Value {
    let unlocked_v = &(*v.lock().unwrap()).v;

    unlocked_v.clone()
}

fn apply_binary_operation(
    op: &BinaryOp,
    op_loc: &Location,
    lhs: &Value,
    rhs: &Value,
)
    -> Result<Value, Error>
{
    let (line, col) = op_loc;
    let new_invalid_op_types = || {
        Error::AtLoc{
            source: Box::new(Error::InvalidOpTypes{
                op: op.clone(),
                lhs: lhs.clone(),
                rhs: rhs.clone(),
            }),
            line: *line,
            col: *col,
        }
    };

    match op {
        BinaryOp::Eq |
        BinaryOp::Ne => {
            if let Some(v) = eq(lhs, rhs) {
                match op {
                    BinaryOp::Eq => Ok(Value::Bool(v)),
                    _ => Ok(Value::Bool(!v)),
                }
            } else {
                Err(new_invalid_op_types())
            }
        },

        BinaryOp::Sum => {
            match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) =>
                    Ok(Value::Int(a + b)),
                (Value::Str(a), Value::Str(b)) =>
                    Ok(Value::Str([a.clone(), b.clone()].concat())),
                _ =>
                    Err(new_invalid_op_types()),
            }
        },

        BinaryOp::Sub |
        BinaryOp::Mul |
        BinaryOp::Div |
        BinaryOp::Mod => {
            match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => {
                    let v =
                        match op {
                            BinaryOp::Sub => a - b,
                            BinaryOp::Mul => a * b,
                            BinaryOp::Div => a / b,
                            BinaryOp::Mod => a % b,

                            _ => panic!("unexpected operation"),
                        };

                    Ok(Value::Int(v))
                },

                _ => {
                    Err(new_invalid_op_types())
                },
            }
        },

        BinaryOp::And |
        BinaryOp::Or => {
            match (lhs, rhs) {
                (Value::Bool(a), Value::Bool(b)) => {
                    let v =
                        match op {
                            BinaryOp::And => *a && *b,
                            BinaryOp::Or => *a || *b,

                            _ => panic!("unexpected operation"),
                        };

                    Ok(Value::Bool(v))
                },

                _ => {
                    Err(new_invalid_op_types())
                },
            }
        },

        BinaryOp::Gt |
        BinaryOp::Gte |
        BinaryOp::Lt |
        BinaryOp::Lte => {
            match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => {
                    let v =
                        match op {
                            BinaryOp::Gt => a > b,
                            BinaryOp::Gte => a >= b,
                            BinaryOp::Lt => a < b,
                            BinaryOp::Lte => a <= b,

                            _ => panic!("unexpected operation"),
                        };

                    Ok(Value::Bool(v))
                },

                _ => {
                    Err(new_invalid_op_types())
                },
            }
        },
    }
}

// `eq` returns `None` if `lhs` and `rhs` are of different types.
fn eq(lhs: &Value, rhs: &Value) -> Option<bool> {
    match (lhs, rhs) {
        (Value::Null, Value::Null) =>
            Some(true),

        (Value::Bool(a), Value::Bool(b)) =>
            Some(a == b),

        (Value::Int(a), Value::Int(b)) =>
            Some(a == b),

        (Value::Str(a), Value::Str(b)) =>
            Some(a == b),

        (Value::List(xs), Value::List(ys)) => {
            if xs.len() != ys.len() {
                return Some(false);
            }

            for (i, x) in xs.iter().enumerate() {
                let y = &ys[i];

                // See comment above call to `apply_binary_operation` in
                // `eval_expr` for details on why we clone the value from
                // inside the `lhs` `Mutex`.
                let equal = eq(
                    &clone_value(x),
                    &y.lock().unwrap().v,
                )?;

                if !equal {
                    return Some(false);
                }
            }

            Some(true)
        },

        (Value::Object(xs), Value::Object(ys)) => {
            if xs.len() != ys.len() {
                return Some(false);
            }

            for (k, x) in xs.iter() {
                let y =
                    if let Some(y) = ys.get(k) {
                        y
                    } else {
                        return Some(false);
                    };

                // See comment above call to `apply_binary_operation` in
                // `eval_expr` for details on why we clone the value from
                // inside the `lhs` `Mutex`.
                let equal = eq(
                    &clone_value(x),
                    &y.lock().unwrap().v,
                )?;

                if !equal {
                    return Some(false);
                }
            }

            Some(true)
        },

        _ =>
            None,
    }
}

pub fn eval_exprs(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    exprs: &Vec<Expr>,
)
    -> Result<List, Error>
{
    let mut vals = vec![];

    for expr in exprs {
        let v = eval_expr(context, scopes, expr)
            .context(EvalExprFailed)?;

        vals.push(v);
    }

    Ok(vals)
}

fn eval_expr_to_str(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    descr: &str,
    expr: &Expr,
)
    -> Result<String, Error>
{
    let (_, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    let raw_str =
        match_eval_expr!((context, scopes, expr) {
            Value::Str(s) => s.clone(),
            value => return new_loc_err(Error::IncorrectType{
                descr: descr.to_string(),
                exp_type: "string".to_string(),
                value: value.clone(),
            }),
        });

    let s =
        match String::from_utf8(raw_str) {
            Ok(s) => s,
            Err(source) => return new_loc_err(Error::StringConstructionFailed{
                source,
                descr: descr.to_string(),
            }),
        };

    Ok(s)
}

fn eval_expr_to_bool(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    descr: &str,
    expr: &Expr,
)
    -> Result<bool, Error>
{
    let (_, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    match_eval_expr!((context, scopes, expr) {
        Value::Bool(b) =>
            Ok(*b),

        value =>
            new_loc_err(Error::IncorrectType{
                descr: descr.to_string(),
                exp_type: "bool".to_string(),
                value: value.clone(),
            }),
    })
}

fn eval_expr_to_i64(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    descr: &str,
    expr: &Expr,
)
    -> Result<i64, Error>
{
    let (_, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    match_eval_expr!((context, scopes, expr) {
        Value::Int(n) =>
            Ok(*n),

        value =>
            new_loc_err(Error::IncorrectType{
                descr: descr.to_string(),
                exp_type: "int".to_string(),
                value: value.clone(),
            }),
    })
}
