// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryInto;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

pub mod bind;
pub mod builtins;
pub mod error;
pub mod scope;
#[macro_use]
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
use self::value::Func;
use self::value::ListRef;
use self::value::SourcedValue;
use self::value::Str;
use self::value::Value;

use lexer::Lexer;
use parser::ExprParser;

macro_rules! match_eval_expr {
    (
        ( $context:ident, $scopes:ident, $expr:expr )
        { $( $key:pat => $value:expr , )* }
    ) => {{
        let value = eval_expr($context, $scopes, $expr)
            .context(EvalExprFailed)?;
        match value.v {
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
    pub global_bindings: &'a Vec<(RawExpr, SourcedValue)>,
}

pub fn eval_prog(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    global_bindings: Vec<(RawExpr, SourcedValue)>,
    Prog::Body{stmts}: &Prog,
)
    -> Result<()>
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
    new_bindings: Vec<(Expr, SourcedValue)>,
    stmts: &Block,
)
    -> Result<Escape>
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
    -> Result<Escape>
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
    Return{value: SourcedValue, loc: Location},
}

#[allow(clippy::too_many_lines)]
fn eval_stmt(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    stmt: &Stmt,
)
    -> Result<Escape>
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

        Stmt::OpAssign{lhs, op, op_loc, rhs} => {
            let rhs_val = eval_expr(context, scopes, rhs)
                .context(EvalBinOpRhsFailed)?;

            bind::bind_next(
                context,
                scopes,
                &mut HashSet::new(),
                lhs,
                rhs_val,
                Some((op.clone(), *op_loc)),
                BindType::Assignment,
            )
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

            let pairs = value_to_pairs(&iter_val.v)
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
fn value_to_pairs(v: &Value) -> Result<Vec<(SourcedValue, SourcedValue)>> {
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

            Value::List(items) => {
                let items = &deref!(items);

                items
                    .iter()
                    .enumerate()
                    .map(|(i, value)| {
                        // TODO Handle issues caused by casting.
                        (value::new_int(i as i64), value.clone())
                    })
                    .collect()
            },

            Value::Object(props) => {
                let props = &deref!(props);

                props
                    .iter()
                    .map(|(key, value)| {
                        (
                            value::new_str_from_string(key.to_string()),
                            value.clone(),
                        )
                    })
                    .collect()
            },

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
    -> Result<Escape>
{
    eval_stmts(context, outer_scopes, vec![], stmts)
}

#[allow(clippy::too_many_lines)]
fn eval_expr(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    expr: &Expr,
) -> Result<SourcedValue> {
    let (raw_expr, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    match raw_expr {
        RawExpr::Null => Ok(value::new_null()),

        RawExpr::Bool{b} => Ok(value::new_bool(*b)),

        RawExpr::Int{n} => Ok(value::new_int(*n)),

        RawExpr::Str{s, interpolation_slots} => {
            if let Some(slots) = interpolation_slots {
                let v =
                    interpolate_string(
                        context,
                        scopes,
                        s,
                        slots,
                        (line, col),
                    )
                    .context(InterpolateStringFailed)?;
                Ok(value::new_str_from_string(v))
            } else {
                Ok(value::new_str_from_string(s.clone()))
            }
        },

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

            let v = apply_binary_operation(op, op_loc, &lhs_val.v, &rhs_val.v)
                .context(ApplyBinOpFailed)?;

            Ok(value::new_val_ref_with_no_source(v))
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

            match source_val.v {
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
                        match deref!(list).get(index) {
                            Some(v) => v.clone(),
                            None => return new_loc_err(
                                Error::OutOfListBounds{index},
                            ),
                        };

                    Ok(v)
                },

                Value::Object(ref props) => {
                    // TODO Consider whether non-UTF-8 strings can be used to
                    // perform key lookups on objects.
                    let name =
                        eval_expr_to_str(context, scopes, "property", locat)
                            .context(EvalObjectIndexFailed)?;

                    let v =
                        match deref!(props).get(&name) {
                            Some(value) => {
                                value.v.clone()
                            },
                            None => {
                                return new_loc_err(Error::PropNotFound{name});
                            },
                        };

                    Ok(value::new_val_ref_with_source(v, source_val.v.clone()))
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
                        match get_str_range_index(&s, start_val, end_val) {
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
                    let range_values =
                        get_list_range_index(&items, start_val, end_val);

                    let v =
                        match range_values {
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
            let mut vals = BTreeMap::<String, SourcedValue>::new();

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
                                    for (name, value) in &deref!(props) {
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
                                                value,
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

            let namespace =
                if *type_prop {
                    match source.v {
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
                    match source.v {
                        Value::Object(ref props) => props,

                        value => {
                            return new_loc_err(Error::PropAccessOnNonObject{
                                value,
                            })
                        },
                    }
                };

            let v =
                if let Some(value) = deref!(namespace).get(name) {
                    Ok(value::new_val_ref_with_source(
                        value.v.clone(),
                        source.v.clone(),
                    ))
                } else if *type_prop {
                    new_loc_err(Error::TypeFunctionNotFound{
                        value: source.v.clone(),
                        name: name.clone(),
                    })
                } else {
                    new_loc_err(Error::PropNotFound{
                        name: name.clone(),
                    })
                };

            v
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
        this: Option<SourcedValue>,
        args: Vec<SourcedValue>,
    },
    Func{
        bindings: Vec<(Expr, SourcedValue)>,
        closure: ScopeStack,
        stmts: Block,
    },
}

#[allow(clippy::too_many_lines)]
fn apply_binary_operation(
    op: &BinaryOp,
    op_loc: &Location,
    lhs: &Value,
    rhs: &Value,
)
    -> Result<Value>
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
    let new_int_overflow = |lhs: &i64, rhs: &i64| {
        Error::AtLoc{
            source: Box::new(Error::IntOverflow{
                op: op.clone(),
                lhs: *lhs,
                rhs: *rhs,
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
                (Value::Int(a), Value::Int(b)) => {
                    if let Some(v) = a.checked_add(*b) {
                        Ok(Value::Int(v))
                    } else {
                        Err(new_int_overflow(a, b))
                    }
                },
                (Value::Str(a), Value::Str(b)) => {
                    Ok(Value::Str([a.clone(), b.clone()].concat()))
                },
                (Value::List(a), Value::List(b)) => {
                    let a = deref!(a).clone();
                    let b = deref!(b).clone();

                    Ok(Value::List(Arc::new(Mutex::new([a, b].concat()))))
                },
                _ => {
                    Err(new_invalid_op_types())
                },
            }
        },

        BinaryOp::Sub |
        BinaryOp::Mul |
        BinaryOp::Div |
        BinaryOp::Mod => {
            match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => {
                    match op {
                        BinaryOp::Sub => {
                            if let Some(v) = a.checked_sub(*b) {
                                Ok(Value::Int(v))
                            } else {
                                Err(new_int_overflow(a, b))
                            }
                        },
                        BinaryOp::Mul => {
                            if let Some(v) = a.checked_mul(*b) {
                                Ok(Value::Int(v))
                            } else {
                                Err(new_int_overflow(a, b))
                            }
                        },
                        BinaryOp::Div => {
                            if let Some(v) = a.checked_div(*b) {
                                Ok(Value::Int(v))
                            } else {
                                Err(new_int_overflow(a, b))
                            }
                        },
                        BinaryOp::Mod => {
                            Ok(Value::Int(a % b))
                        },
                        _ => {
                            panic!("unexpected operation");
                        },
                    }
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
            if value::ref_eq(xs, ys) {
                return Some(true);
            }

            if deref!(xs).len() != deref!(ys).len() {
                return Some(false);
            }

            for (i, x) in deref!(xs).iter().enumerate() {
                let y = &deref!(ys)[i];

                let equal = eq(&x.v, &y.v)?;

                if !equal {
                    return Some(false);
                }
            }

            Some(true)
        },

        (Value::Object(xs), Value::Object(ys)) => {
            if value::ref_eq(xs, ys) {
                return Some(true);
            }

            if deref!(xs).len() != deref!(ys).len() {
                return Some(false);
            }

            for (k, x) in &deref!(xs) {
                let ys = &deref!(ys);
                let y =
                    if let Some(y) = ys.get(k) {
                        y
                    } else {
                        return Some(false);
                    };

                let equal = eq(&x.v, &y.v)?;

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
    -> Result<String>
{
    let (_, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    let raw_str =
        match_eval_expr!((context, scopes, expr) {
            Value::Str(s) => s,
            value => return new_loc_err(Error::IncorrectType{
                descr: descr.to_string(),
                exp_type: "string".to_string(),
                value,
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
    -> Result<bool>
{
    let (_, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    match_eval_expr!((context, scopes, expr) {
        Value::Bool(b) =>
            Ok(b),

        value =>
            new_loc_err(Error::IncorrectType{
                descr: descr.to_string(),
                exp_type: "bool".to_string(),
                value,
            }),
    })
}

fn eval_expr_to_i64(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    descr: &str,
    expr: &Expr,
)
    -> Result<i64>
{
    let (_, (line, col)) = expr;
    let new_loc_err = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    match_eval_expr!((context, scopes, expr) {
        Value::Int(n) =>
            Ok(n),

        value =>
            new_loc_err(Error::IncorrectType{
                descr: descr.to_string(),
                exp_type: "int".to_string(),
                value,
            }),
    })
}

fn eval_expr_to_index(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    expr: &Expr,
)
    -> Result<usize>
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
    -> Result<SourcedValue>
{
    let start = maybe_start.get_or_insert(0);
    let end = maybe_end.get_or_insert(s.len());

    if let Some(vs) = s.get(*start .. *end) {
        return Ok(value::new_str(vs.to_vec()));
    }

    Err(Error::RangeOutOfStringBounds{start: *start, end: *end})
}

fn get_list_range_index(
    list: &ListRef,
    mut maybe_start: Option<usize>,
    mut maybe_end: Option<usize>,
)
    -> Result<SourcedValue>
{
    let start = maybe_start.get_or_insert(0);
    let end = maybe_end.get_or_insert(deref!(list).len());

    if let Some(vs) = deref!(list).get(*start .. *end) {
        return Ok(value::new_list(vs.to_vec()));
    }

    Err(Error::RangeOutOfListBounds{start: *start, end: *end})
}

fn eval_list_items(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    items: &Vec<ListItem>,
)
    -> Result<Vec<SourcedValue>>
{
    let mut vals = vec![];

    for item in items {
        let v = eval_expr(context, scopes, &item.expr)
            .context(EvalListItemFailed)?;

        if !item.is_spread {
            vals.push(v);

            continue;
        }

        match v.v {
            Value::List(items) => {
                for item in &deref!(items) {
                    vals.push(item.clone());
                }
            },

            value => {
                let (_, (line, col)) = item.expr;

                return Err(Error::AtLoc{
                    source: Box::new(Error::SpreadNonListInList{value}),
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
    -> Result<SourcedValue>
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
            let SourcedValue{v, source} = func_val;

            match v {
                Value::BuiltinFunc{name, f} => {
                    let this = source.map(value::new_val_ref_with_no_source);

                    (
                        Some(name),
                        CallBinding::BuiltinFunc{f, this, args: arg_vals},
                    )
                },

                Value::Func(f) => {
                    let Func{
                        name,
                        args: arg_names,
                        collect_args,
                        stmts,
                        closure,
                    } = &deref!(f);

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

                    let mut bindings: Vec<(Expr, SourcedValue)> = vec![];

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
                            value::new_val_ref_with_no_source(this),
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
                    return new_loc_err(Error::CannotCallNonFunc{v});
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

fn interpolate_string(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    s: &str,
    interpolation_slots: &Vec<(usize, usize)>,
    loc: (&usize, &usize),
)
    -> Result<String>
{
    let (line, col) = loc;
    let new_loc_err = |source, col| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col})
    };

    let parser = ExprParser::new();

    let mut result: Vec<String> = vec![];

    let mut last_slot_end = 0;

    for cur_slot in interpolation_slots {
        let (cur_slot_start, cur_slot_end) = cur_slot;
        result.push(s[last_slot_end .. *cur_slot_start].to_string());

        // We shorten the slot to skip the delimiters (`${` at the start and
        // `}` at the end).
        let directive = &s[(cur_slot_start+2) .. (cur_slot_end-1)];

        let slot_col = col + cur_slot_start + 4;

        let mut lexer = Lexer::new(directive);

        let ast =
            match parser.parse(&mut lexer) {
                Ok(v) => v,
                Err(e) => return new_loc_err(
                    Error::InterpolateStringParseFailed{
                        source_str: format!("{:?}", e),
                    },
                    slot_col,
                ),
            };

        // We catch the evaluation error manually so that we can modify the
        // location of the error to account for the string location.
        let v =
            match eval_expr(context, scopes, &ast) {
                Ok(v) => v,
                Err(e) => return new_loc_err(
                    Error::InterpolateStringEvalExprFailed{
                        source: Box::new(e),
                    },
                    slot_col,
                ),
            };

        match v.v {
            Value::Str(s) => {
                match String::from_utf8(s.clone()) {
                    Ok(s) => result.push(s),
                    Err(source) => return new_loc_err(
                        Error::StringConstructionFailed{
                            source,
                            descr: "interpolated slot".to_string(),
                        },
                        slot_col,
                    ),
                }
            },
            value => {
                return new_loc_err(
                    Error::InterpolatedValueNotString{value},
                    slot_col,
                );
            },
        }

        last_slot_end = *cur_slot_end;
    }

    result.push(s[last_slot_end ..].to_string());

    Ok(result.join(""))
}
