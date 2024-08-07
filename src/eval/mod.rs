// Copyright 2023-2024 Sean Kelleher. All rights reserved.
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
use self::value::Str;
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
    let mut new_scopes = scopes.new_from_push(HashMap::new());

    for (lhs, rhs) in new_bindings {
        bind::bind(context, &mut new_scopes, &lhs, rhs, BindType::Declaration)
            .context(BindFailed)?;
    }

    let v = eval_stmts_with_scope_stack(context, &mut new_scopes, stmts)
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

            bind::bind(context, scopes, lhs, v, BindType::Declaration)
                .context(DeclarationBindFailed)?;
        },

        Stmt::Assign{lhs, rhs} => {
            let v = eval_expr(context, scopes, rhs)
                .context(EvalAssignmentRhsFailed)?;

            bind::bind(context, scopes, lhs, v, BindType::Assignment)
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

        Stmt::Func{name: (name, loc), args, collect_args, stmts} => {
            let closure = scopes.clone();
            let func = value::new_func(
                Some(name.clone()),
                args.clone(),
                *collect_args,
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
    let new_loc_err = |source| {
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
                    None => return new_loc_err(
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

        RawExpr::List{items, collect} => {
            if *collect {
                return new_loc_err(Error::ListCollectOutsideDestructure);
            }

            let vals = eval_list_items(context, scopes, items)
                .context(EvalListItemsFailed)?;

            Ok(value::new_list(vals))
        },

        RawExpr::Index{expr, location: locat} => {
            // We don't use `match_eval_expr` here because we need to maintain
            // a reference to `source` for use in object lookups.
            let source_val = eval_expr(context, scopes, expr)
                .context(EvalSourceExprFailed)?;

            let unlocked_source_val = &(*source_val.lock().unwrap()).v;

            match unlocked_source_val {
                Value::Str(s) => {
                    let index = eval_expr_to_index(context, scopes, locat)
                        .context(EvalStringIndexFailed)?;

                    let v =
                        match s.get(index) {
                            Some(v) => value::new_str(vec![*v]),
                            None => return new_loc_err(
                                Error::OutOfStringBounds{index},
                            ),
                        };

                    Ok(v)
                },

                Value::List(list) => {
                    let index = eval_expr_to_index(context, scopes, locat)
                        .context(EvalListIndexFailed)?;

                    let v =
                        match list.get(index) {
                            Some(v) => v.clone(),
                            None => return new_loc_err(
                                Error::OutOfListBounds{index},
                            ),
                        };

                    Ok(v)
                },

                Value::Object(props) => {
                    // TODO Consider whether non-UTF-8 strings can be used to
                    // perform key lookups on objects.
                    let name =
                        eval_expr_to_str(context, scopes, "property", locat)
                            .context(EvalObjectIndexFailed)?;

                    let v =
                        match props.get(&name) {
                            Some(v) => {
                                let prop_val = &(*v.lock().unwrap()).v;

                                value::new_val_ref_with_source(
                                    prop_val.clone(),
                                    source_val.clone(),
                                )
                            },
                            None => {
                                return new_loc_err(Error::PropNotFound{name});
                            },
                        };

                    Ok(v)
                },

                _ => {
                    new_loc_err(Error::ValueNotIndexable)
                },
            }
        },

        RawExpr::RangeIndex{expr, start: maybe_start, end: maybe_end} => {
            let start_val =
                if let Some(start) = maybe_start {
                    let start_val = eval_expr_to_index(context, scopes, start)
                            .context(EvalStartIndexFailed)?;

                    Some(start_val)
                } else {
                    None
                };

            let end_val =
                if let Some(end) = maybe_end {
                    let end_val = eval_expr_to_index(context, scopes, end)
                            .context(EvalEndIndexFailed)?;

                    Some(end_val)
                } else {
                    None
                };

            match_eval_expr!((context, scopes, expr) {
                Value::Str(s) => {
                    let v =
                        match get_str_range_index(s, start_val, end_val) {
                            Ok(v) => v,

                            // TODO Instead of using `new_loc_err`, check
                            // whether the `start` or `end` is out of bounds,
                            // and output the index of the expression that
                            // corresponds to the error.
                            Err(source) => return new_loc_err(
                                Error::EvalStringRangeIndexFailed{
                                    source: Box::new(source),
                                },
                            ),
                        };

                    Ok(v)
                },

                Value::List(items) => {
                    let v =
                        match get_list_range_index(items, start_val, end_val) {
                            Ok(v) => v,

                            // TODO Instead of using `new_loc_err`, check
                            // whether the `start` or `end` is out of bounds,
                            // and output the index of the expression that
                            // corresponds to the error.
                            Err(source) => return new_loc_err(
                                Error::EvalListRangeIndexFailed{
                                    source: Box::new(source),
                                },
                            ),
                        };

                    Ok(v)
                },

                _ => {
                    new_loc_err(Error::ValueNotRangeIndexable)
                },
            })
        },

        RawExpr::Range{start, end} => {
            let start = eval_expr_to_i64(context, scopes, "range start", start)
                .context(EvalRangeStartFailed)?;

            let end = eval_expr_to_i64(context, scopes, "range end", end)
                .context(EvalRangeEndFailed)?;

            let range =
                (start..end)
                    .map(value::new_int)
                    .collect();

            Ok(value::new_list(range))
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

                    PropItem::Single{expr, is_spread, collect} => {
                        if *collect {
                            return new_loc_err(
                                Error::ObjectCollectOutsideDestructure,
                            )
                        }

                        if *is_spread {
                            match_eval_expr!((context, scopes, expr) {
                                Value::Object(props) => {
                                    for (name, value) in props.iter() {
                                        vals.insert(
                                            name.to_string(),
                                            value.clone(),
                                        );
                                    }
                                },

                                value => {
                                    let (_, (line, col)) = expr;

                                    return Err(Error::AtLoc{
                                        source: Box::new(
                                            Error::SpreadNonObjectInObject{
                                                value: value.clone(),
                                            },
                                        ),
                                        line: *line,
                                        col: *col,
                                    });
                                },
                            });
                        } else {
                            let (raw_expr, (line, col)) = expr;

                            if let RawExpr::Var{name} = raw_expr {
                                let v =
                                    match scopes.get(name) {
                                        Some(v) => v.clone(),
                                        None => return Err(Error::AtLoc{
                                            source: Box::new(Error::Undefined{
                                                name: name.clone()
                                            }),
                                            line: *line,
                                            col: *col,
                                        }),
                                    };

                                vals.insert(name.to_string(), v);
                            } else {
                                return Err(Error::AtLoc{
                                    source: Box::new(
                                        Error::ObjectPropShorthandNotVar,
                                    ),
                                    line: *line,
                                    col: *col,
                                });
                            }
                        }
                    },
                }
            }

            Ok(value::new_object(vals))
        },

        RawExpr::Prop{expr, name, type_prop} => {
            let source = eval_expr(context, scopes, expr)
                .context(EvalPropFailed)?;

            let unlocked_source = &mut (*source.lock().unwrap()).v;

            let namespace =
                if *type_prop {
                    match unlocked_source {
                        Value::Bool(_) =>
                            &context.builtins.type_functions.bools,
                        Value::Int(_) =>
                            &context.builtins.type_functions.ints,
                        Value::Str(_) =>
                            &context.builtins.type_functions.strs,
                        Value::List(_) =>
                            &context.builtins.type_functions.lists,
                        Value::Object(_) =>
                            &context.builtins.type_functions.objects,
                        Value::BuiltinFunc{..} | Value::Func{..}  =>
                            &context.builtins.type_functions.funcs,

                        Value::Null => {
                            return new_loc_err(Error::TypeFunctionOnNull)
                        },
                    }
                } else {
                    match unlocked_source {
                        Value::Object(props) => props,

                        value => {
                            return new_loc_err(Error::PropAccessOnNonObject{
                                value: value.clone(),
                            })
                        },
                    }
                };

            match namespace.get(name) {
                Some(v) => {
                    let value = &(*v.lock().unwrap()).v;

                    Ok(value::new_val_ref_with_source(
                        value.clone(),
                        source.clone(),
                    ))
                },
                None => {
                    if *type_prop {
                        new_loc_err(Error::TypeFunctionNotFound{
                            value: unlocked_source.clone(),
                            name: name.clone(),
                        })
                    } else {
                        new_loc_err(Error::PropNotFound{
                            name: name.clone(),
                        })
                    }
                },
            }
        },

        RawExpr::Func{args, collect_args, stmts} => {
            let closure = scopes.clone();

            Ok(value::new_func(
                None,
                args.clone(),
                *collect_args,
                stmts.clone(),
                closure,
            ))
        },

        RawExpr::Call{func, args} => {
            let v = eval_call(context, scopes, func, args, (line, col))
                .context(EvalCallFailed)?;

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

fn eval_expr_to_index(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    expr: &Expr,
)
    -> Result<usize, Error>
{
    let (_, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    let index = eval_expr_to_i64(context, scopes, "index", expr)
        .context(EvalIndexToI64Failed)?;

    if index < 0 {
        return new_loc_err(Error::NegativeIndex{index});
    }

    let i: usize = index.try_into()
        .context(ConvertIndexToUsizeFailed)?;

    Ok(i)
}

fn get_str_range_index(
    s: &Str,
    mut maybe_start: Option<usize>,
    mut maybe_end: Option<usize>,
)
    -> Result<ValRefWithSource, Error>
{
    let start = maybe_start.get_or_insert(0);
    let end = maybe_end.get_or_insert(s.len());

    if let Some(vs) = s.get(*start .. *end) {
        return Ok(value::new_str(vs.to_vec()));
    }

    Err(Error::RangeOutOfStringBounds{start: *start, end: *end})
}

fn get_list_range_index(
    list: &List,
    mut maybe_start: Option<usize>,
    mut maybe_end: Option<usize>,
)
    -> Result<ValRefWithSource, Error>
{
    let start = maybe_start.get_or_insert(0);
    let end = maybe_end.get_or_insert(list.len());

    if let Some(vs) = list.get(*start .. *end) {
        return Ok(value::new_list(vs.to_vec()));
    }

    Err(Error::RangeOutOfListBounds{start: *start, end: *end})
}

fn eval_list_items(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    items: &Vec<ListItem>,
)
    -> Result<Vec<ValRefWithSource>, Error>
{
    let mut vals = vec![];

    for item in items {
        let v = eval_expr(context, scopes, &item.expr)
            .context(EvalListItemFailed)?;

        if !item.is_spread {
            vals.push(v);

            continue;
        }

        match &(*v.lock().unwrap()).v {
            Value::List(items) => {
                for item in items {
                    vals.push(item.clone());
                }
            },

            value => {
                let (_, (line, col)) = item.expr;

                return Err(Error::AtLoc{
                    source: Box::new(Error::SpreadNonListInList{
                        value: value.clone(),
                    }),
                    line,
                    col,
                })
            },
        };
    }

    Ok(vals)
}

#[allow(clippy::too_many_lines)]
fn eval_call(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    func: &Expr,
    args: &Vec<ListItem>,
    loc: (&usize, &usize),
)
    -> Result<ValRefWithSource, Error>
{
    let (line, col) = loc;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    let arg_vals = eval_list_items(context, scopes, args)
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

                Value::Func{
                    name,
                    args: arg_names,
                    collect_args,
                    stmts,
                    closure,
                } => {
                    let num_params = arg_names.len();
                    let got = arg_vals.len();
                    if *collect_args {
                        let minimum = num_params-1;
                        if minimum > got {
                            return new_loc_err(
                                Error::TooFewArgs{minimum, got},
                            );
                        }
                    } else if num_params != got {
                        return new_loc_err(
                            Error::ArgNumMismatch{need: num_params, got}
                        );
                    }

                    let mut bindings: Vec<(Expr, ValRefWithSource)> =
                        vec![];

                    for i in 0 .. num_params {
                        let arg_val =
                            if *collect_args && i == num_params-1 {
                                let rest = arg_vals[num_params-1 ..].to_vec();

                                value::new_list(rest)
                            } else {
                                arg_vals[i].clone()
                            };

                        bindings.push((arg_names[i].clone(), arg_val));
                    }

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
                    return new_loc_err(
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
}
