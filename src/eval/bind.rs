// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashSet;

#[allow(clippy::wildcard_imports)]
use ast::*;
use super::error::Error;
use value::ScopeStack;
use value::ValRefWithSource;

// `bind` associates the values on `rhs` with the names deconstructed from
// `lhs`.
pub fn bind(
    scopes: &mut ScopeStack,
    lhs: Expr,
    rhs: ValRefWithSource,
)
    -> Result<(), Error>
{
    bind_next(scopes, &mut HashSet::new(), lhs, rhs)
}

// `bind_next` performs a bind, but returns an error if a name that's in
// `names_in_binding` gets reused.
fn bind_next(
    scopes: &mut ScopeStack,
    names_in_binding: &mut HashSet<String>,
    lhs: Expr,
    rhs: ValRefWithSource,
)
    -> Result<(), Error>
{
    let (raw_lhs, (line, col)) = lhs;
    let new_invalid_bind_error = |s: &str| {
        let source = Error::InvalidBindTarget{descr: s.to_string()};

        Err(Error::AtLoc{source: Box::new(source), line, col})
    };

    match raw_lhs {
        RawExpr::Var{name} => {
            bind_next_name(scopes, names_in_binding, name, (line, col), rhs)
        },
        RawExpr::Null => new_invalid_bind_error("`null`"),
        RawExpr::Bool{..} => new_invalid_bind_error("a boolean literal"),
        RawExpr::Int{..} => new_invalid_bind_error("an integer literal"),
        RawExpr::Str{..} => new_invalid_bind_error("a string literal"),
        RawExpr::List{..} => new_invalid_bind_error("a list literal"),
        RawExpr::Object{..} => new_invalid_bind_error("an object literal"),
        RawExpr::Call{..} => new_invalid_bind_error("a function call"),
    }
}

fn bind_next_name(
    scopes: &mut ScopeStack,
    names_in_binding: &mut HashSet<String>,
    name: String,
    name_loc: (usize, usize),
    rhs: ValRefWithSource,
)
    -> Result<(), Error>
{
    let (line, col) = name_loc;
    let new_loc_error = |source| {
        Err(Error::AtLoc{source: Box::new(source), line, col})
    };

    if names_in_binding.contains(&name) {
        return new_loc_error(Error::AlreadyInBinding{name});
    }
    names_in_binding.insert(name.clone());

    if !scopes.declare(name.clone(), rhs) {
        return new_loc_error(Error::AlreadyInScope{name});
    }

    Ok(())
}
