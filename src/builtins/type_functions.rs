// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;

use snafu::ResultExt;

use super::fns;
use crate::eval::builtins::TypeFunctions;
use crate::eval::error::AssertArgsFailed;
use crate::eval::error::AssertStrFailed;
use crate::eval::error::AssertThisFailed;
use crate::eval::error::Error;
use crate::eval::value;
use crate::eval::value::List;
use crate::eval::value::ValRefWithSource;

pub fn type_functions() -> TypeFunctions {
    TypeFunctions{
        strs: BTreeMap::<String, ValRefWithSource>::from([
            (
                "len".to_string(),
                value::new_built_in_func("str->len".to_string(), str_len),
            ),
        ]),
        funcs: BTreeMap::<String, ValRefWithSource>::new(),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn str_len(this: Option<ValRefWithSource>, vs: List)
    -> Result<ValRefWithSource, Error>
{
    fns::assert_args("len", 0, &vs)
        .context(AssertArgsFailed)?;

    let this = fns::assert_this(this)
        .context(AssertThisFailed)?;

    let s = fns::assert_str("this", &this)
        .context(AssertStrFailed)?;

    Ok(value::new_int(s.len() as i64))
}
