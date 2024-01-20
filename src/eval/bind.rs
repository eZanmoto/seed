// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashSet;

use snafu::ResultExt;

#[allow(clippy::wildcard_imports)]
use ast::*;
use eval;
use eval::EvaluationContext;
#[allow(clippy::wildcard_imports)]
use super::error::*;
use super::error::Error;
use super::scope::ScopeStack;
use value::ValRefWithSource;
use value::Value;

// TODO Duplicated from `src/eval/mod.rs`.
macro_rules! match_eval_expr {
    (
        ( $context:ident, $scopes:ident, $expr:expr )
        { $( $key:pat => $value:expr , )* }
    ) => {{
        let value = eval::eval_expr($context, $scopes, $expr)
            .context(EvalExprFailed)?;
        let unlocked_value = &mut (*value.lock().unwrap()).v;
        match unlocked_value {
            $( $key => $value , )*
        }
    }};
}

// `bind` associates the values on `rhs` with the names deconstructed from
// `lhs`.
pub fn bind(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    lhs: &Expr,
    rhs: ValRefWithSource,
    bind_type: BindType,
)
    -> Result<(), Error>
{
    bind_next(context, scopes, &mut HashSet::new(), lhs, rhs, bind_type)
}

#[derive(Clone, Copy)]
pub enum BindType {
    Declaration,
    Assignment,
}

// `bind_next` performs a bind, but returns an error if a name that's in
// `names_in_binding` gets reused.
fn bind_next(
    context: &EvaluationContext,
    scopes: &mut ScopeStack,
    names_in_binding: &mut HashSet<String>,
    lhs: &Expr,
    rhs: ValRefWithSource,
    bind_type: BindType,
)
    -> Result<(), Error>
{
    let (raw_lhs, loc) = lhs;
    let new_loc_err = |source| {
        let (line, col) = loc;

        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };
    let new_invalid_bind_error = |s: &str| {
        new_loc_err(Error::InvalidBindTarget{descr: s.to_string()})
    };

    match raw_lhs {
        RawExpr::Var{name} => {
            bind_next_name(scopes, names_in_binding, name, loc, rhs, bind_type)
        },

        RawExpr::Index{expr, location: locat} => {
            match_eval_expr!((context, scopes, expr) {
                Value::List(items) => {
                    let n = eval::eval_expr_to_index(context, scopes, locat)
                        .context(EvalListIndexFailed)?;

                    if n >= items.len() {
                        return new_loc_err(Error::OutOfListBounds{index: n});
                    }

                    items[n as usize] = rhs;

                    Ok(())
                },

                Value::Object(props) => {
                    // TODO Consider whether non-UTF-8 strings can be used to
                    // perform key lookups on objects.
                    let descr = "property";
                    let name =
                        eval::eval_expr_to_str(context, scopes, descr, locat)
                            .context(EvalObjectIndexFailed)?;

                    props.insert(name, rhs);

                    Ok(())
                },

                _ => {
                    new_loc_err(Error::ValueNotIndexAssignable)
                },
            })
        },

        RawExpr::Prop{expr, name} => {
            match_eval_expr!((context, scopes, expr) {
                Value::Object(props) => {
                    props.insert(name.clone(), rhs);

                    Ok(())
                },

                value => {
                    new_loc_err(Error::PropAccessOnNonObject{
                        value: value.clone(),
                    })
                },
            })
        },

        RawExpr::Null =>
            new_invalid_bind_error("`null`"),
        RawExpr::Bool{..} =>
            new_invalid_bind_error("a boolean literal"),
        RawExpr::Int{..} =>
            new_invalid_bind_error("an integer literal"),
        RawExpr::Str{..} =>
            new_invalid_bind_error("a string literal"),
        RawExpr::BinaryOp{..} =>
            new_invalid_bind_error("a binary operation"),
        RawExpr::List{..} =>
            new_invalid_bind_error("a list literal"),
        RawExpr::RangeIndex{..} =>
            new_invalid_bind_error("a range-index operation"),
        RawExpr::Range{..} =>
            new_invalid_bind_error("a range operation"),
        RawExpr::Object{..} =>
            new_invalid_bind_error("an object literal"),
        RawExpr::Func{..} =>
            new_invalid_bind_error("an anonymous function"),
        RawExpr::Call{..} =>
            new_invalid_bind_error("a function call"),
    }
}

pub fn bind_name(
    scopes: &mut ScopeStack,
    name: &str,
    name_loc: &(usize, usize),
    rhs: ValRefWithSource,
    bind_type: BindType,
)
    -> Result<(), Error>
{
    bind_next_name(scopes, &mut HashSet::new(), name, name_loc, rhs, bind_type)
}

fn bind_next_name(
    scopes: &mut ScopeStack,
    names_in_binding: &mut HashSet<String>,
    name: &str,
    name_loc: &(usize, usize),
    rhs: ValRefWithSource,
    bind_type: BindType,
)
    -> Result<(), Error>
{
    let (line, col) = name_loc;
    let new_loc_error = |source| {
        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    if names_in_binding.contains(name) {
        return new_loc_error(Error::AlreadyInBinding{name: name.to_string()});
    }
    names_in_binding.insert(name.to_string());

    match bind_type {
        BindType::Declaration => {
            if let Err((line, col)) = scopes.declare(name, *name_loc, rhs) {
                return new_loc_error(Error::AlreadyInScope{
                    name: name.to_string(),
                    prev_line: line,
                    prev_col: col,
                });
            }
        },
        BindType::Assignment => {
            if !scopes.assign(name, rhs) {
                return new_loc_error(Error::Undefined{
                    name: name.to_string(),
                });
            }
        },
    };

    Ok(())
}
