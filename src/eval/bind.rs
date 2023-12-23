// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashSet;

#[allow(clippy::wildcard_imports)]
use ast::*;
use super::error::Error;
use super::scope::ScopeStack;
use value::ValRefWithSource;

// `bind` associates the values on `rhs` with the names deconstructed from
// `lhs`.
pub fn bind(
    scopes: &mut ScopeStack,
    lhs: &Expr,
    rhs: ValRefWithSource,
    bind_type: BindType,
)
    -> Result<(), Error>
{
    bind_next(scopes, &mut HashSet::new(), lhs, rhs, bind_type)
}

#[derive(Clone, Copy)]
pub enum BindType {
    Declaration,
    Assignment,
}

// `bind_next` performs a bind, but returns an error if a name that's in
// `names_in_binding` gets reused.
fn bind_next(
    scopes: &mut ScopeStack,
    names_in_binding: &mut HashSet<String>,
    lhs: &Expr,
    rhs: ValRefWithSource,
    bind_type: BindType,
)
    -> Result<(), Error>
{
    let (raw_lhs, loc) = lhs;
    let new_invalid_bind_error = |s: &str| {
        let source = Error::InvalidBindTarget{descr: s.to_string()};
        let (line, col) = loc;

        Err(Error::AtLoc{source: Box::new(source), line: *line, col: *col})
    };

    match raw_lhs {
        RawExpr::Var{name} => {
            bind_next_name(scopes, names_in_binding, name, loc, rhs, bind_type)
        },
        RawExpr::Null => new_invalid_bind_error("`null`"),
        RawExpr::Bool{..} => new_invalid_bind_error("a boolean literal"),
        RawExpr::Int{..} => new_invalid_bind_error("an integer literal"),
        RawExpr::Str{..} => new_invalid_bind_error("a string literal"),
        RawExpr::BinaryOp{..} => new_invalid_bind_error("a binary operation"),
        RawExpr::List{..} => new_invalid_bind_error("a list literal"),
        RawExpr::Object{..} => new_invalid_bind_error("an object literal"),
        RawExpr::Call{..} => new_invalid_bind_error("a function call"),
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
