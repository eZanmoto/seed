// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

pub mod bind;
pub mod builtins;
pub mod error;
pub mod value;

use snafu::ResultExt;

#[allow(clippy::wildcard_imports)]
use ast::*;
use self::builtins::Builtins;
// We use a wildcard import for `error` to import the many error variant
// constructors created by Snafu.
#[allow(clippy::wildcard_imports)]
use self::error::*;
use self::error::Error;
use self::value::BuiltinFunc;
use self::value::List;
use self::value::ScopeStack;
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

    eval_stmts(context, scopes, bindings, stmts)
        .context(EvalProgFailed)?;

    Ok(())
}

// `eval_stmts` evaluates `stmts` in a new scope pushed onto `scopes`, with the
// given `new_bindings` declared in the new scope. It returns `Some(v)` if one
// of the statements is evaluates is a a `return` statement, otherwise it
// returns `None`.
pub fn eval_stmts(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    new_bindings: Vec<(Expr, ValRefWithSource)>,
    stmts: &Block,
)
    -> Result<(), Error>
{
    let mut inner_scopes = scopes.new_from_push(HashMap::new());

    for (lhs, rhs) in new_bindings {
        bind::bind(&mut inner_scopes, &lhs, rhs)
            .context(BindFailed)?;
    }

    eval_stmts_with_scope_stack(context, &mut inner_scopes, stmts)
        .context(EvalStmtsFailed)?;

    Ok(())
}

pub fn eval_stmts_with_scope_stack(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    stmts: &Block,
)
    -> Result<(), Error>
{
    for stmt in stmts {
        eval_stmt(context, scopes, stmt)
            .context(EvalStmtsWithScopeStackFailed)?;
    }

    Ok(())
}

// `eval_stmt` returns `Some(v)` if a `return` value is evaluated, otherwise it
// returns `None`.
fn eval_stmt(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    stmt: &Stmt,
)
    -> Result<(), Error>
{
    match stmt {
        Stmt::Declare{lhs, rhs} => {
            let v = eval_expr(context, scopes, rhs)
                .context(EvalDeclarationLhsFailed)?;

            bind::bind(scopes, lhs, v)
                .context(DeclarationBindFailed)?;
        },

        Stmt::Expr{expr} => {
            eval_expr(context, scopes, expr)
                .context(EvalStmtFailed)?;
        },
    }

    Ok(())
}

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

        RawExpr::List{items} => {
            let mut vals = vec![];

            for item in items {
                let v = eval_expr(context, scopes, item)
                    .context(EvalListItemFailed)?;

                vals.push(v);
            }

            Ok(value::new_list(vals))
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

        RawExpr::Call{expr, args} => {
            let arg_vals = eval_exprs(context, scopes, args)
                .context(EvalCallArgsFailed)?;

            let func_val = eval_expr(context, scopes, expr)
                .context(EvalCallFuncFailed)?;

            let v =
                {
                    let ValWithSource{v, source} = &*func_val.lock().unwrap();
                    match v {
                        Value::BuiltInFunc{f} => {
                            let this = source.as_ref().cloned();

                            CallBinding::BuiltInFunc{
                                f: *f,
                                this,
                                args: arg_vals,
                            }
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
                    CallBinding::BuiltInFunc{f, this, args} => {
                        f(this, args)?
                    },
                };

            Ok(v)
        },
    }
}

enum CallBinding {
    BuiltInFunc{
        f: BuiltinFunc,
        this: Option<ValRefWithSource>,
        args: List,
    },
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
