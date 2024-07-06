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
use crate::eval::value::Value;

pub fn type_functions() -> TypeFunctions {
    TypeFunctions{
        bools: BTreeMap::<String, ValRefWithSource>::from([
            (
                "type".to_string(),
                value::new_built_in_func("bool->type".to_string(), any_type),
            ),
        ]),
        ints: BTreeMap::<String, ValRefWithSource>::from([
            (
                "type".to_string(),
                value::new_built_in_func("int->type".to_string(), any_type),
            ),
        ]),
        strs: BTreeMap::<String, ValRefWithSource>::from([
            (
                "len".to_string(),
                value::new_built_in_func("str->len".to_string(), str_len),
            ),
            (
                "type".to_string(),
                value::new_built_in_func("str->type".to_string(), any_type),
            ),
        ]),
        lists: BTreeMap::<String, ValRefWithSource>::from([
            (
                "type".to_string(),
                value::new_built_in_func("list->type".to_string(), any_type),
            ),
        ]),
        objects: BTreeMap::<String, ValRefWithSource>::from([
            (
                "type".to_string(),
                value::new_built_in_func("object->type".to_string(), any_type),
            ),
        ]),
        funcs: BTreeMap::<String, ValRefWithSource>::from([
            (
                "type".to_string(),
                value::new_built_in_func("func->type".to_string(), any_type),
            ),
        ]),
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

#[allow(clippy::needless_pass_by_value)]
pub fn any_type(this: Option<ValRefWithSource>, vs: List)
    -> Result<ValRefWithSource, Error>
{
    fns::assert_args("type", 0, &vs)
        .context(AssertArgsFailed)?;

    let this = fns::assert_this(this)
        .context(AssertThisFailed)?;

    let unlocked_value = &(*this.lock().unwrap()).v;
    let s = render_type(unlocked_value);

    Ok(value::new_str_from_string(s))
}

// TODO Duplicated from `src/eval/error.rs`.
fn render_type(v: &Value) -> String {
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
