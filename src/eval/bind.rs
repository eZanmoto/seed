// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashSet;

#[allow(clippy::wildcard_imports)]
use ast::*;
use value::Error;
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
    match lhs {
        Expr::Var{name} => {
            bind_next_name(scopes, names_in_binding, name, rhs)
        },
        Expr::Null => {
            Err(Error::InvalidBindTarget{
                descr: "`null`".to_string(),
            })
        },
        Expr::Str{..} => {
            Err(Error::InvalidBindTarget{
                descr: "a string literal".to_string(),
            })
        },
        Expr::Call{..} => {
            Err(Error::InvalidBindTarget{
                descr: "a function call".to_string(),
            })
        },
    }
}

fn bind_next_name(
    scopes: &mut ScopeStack,
    names_in_binding: &mut HashSet<String>,
    name: String,
    rhs: ValRefWithSource,
)
    -> Result<(), Error>
{
    if names_in_binding.contains(&name) {
        return Err(Error::AlreadyInBinding{name});
    }
    names_in_binding.insert(name.clone());

    if !scopes.declare(name.clone(), rhs) {
        return Err(Error::AlreadyInScope{name});
    }

    Ok(())
}
