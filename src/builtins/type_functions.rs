// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::sync::Arc;
use std::sync::Mutex;

use snafu::ResultExt;

use super::fns;
use crate::eval::builtins::TypeFunctions;
use crate::eval::error::AssertArgsFailed;
use crate::eval::error::AssertStrFailed;
use crate::eval::error::AssertThisFailed;
use crate::eval::error::Error;
use crate::eval::value;
use crate::eval::value::ObjectRef;
use crate::eval::value::SourcedValue;
use crate::eval::value::Value;

pub fn type_functions() -> TypeFunctions {
    TypeFunctions{
        bools: new_func_map(vec![
            (
                "type".to_string(),
                value::new_built_in_func("bool->type".to_string(), any_type),
            ),
        ]),
        ints: new_func_map(vec![
            (
                "type".to_string(),
                value::new_built_in_func("int->type".to_string(), any_type),
            ),
        ]),
        strs: new_func_map(vec![
            (
                "len".to_string(),
                value::new_built_in_func("str->len".to_string(), str_len),
            ),
            (
                "type".to_string(),
                value::new_built_in_func("str->type".to_string(), any_type),
            ),
        ]),
        lists: new_func_map(vec![
            (
                "type".to_string(),
                value::new_built_in_func("list->type".to_string(), any_type),
            ),
        ]),
        objects: new_func_map(vec![
            (
                "type".to_string(),
                value::new_built_in_func("object->type".to_string(), any_type),
            ),
        ]),
        funcs: new_func_map(vec![
            (
                "type".to_string(),
                value::new_built_in_func("func->type".to_string(), any_type),
            ),
        ]),
    }
}

pub fn new_func_map(funcs: Vec<(String, SourcedValue)>) -> ObjectRef {
    Arc::new(Mutex::new(BTreeMap::<String, SourcedValue>::from_iter(
        funcs,
    )))
}

#[allow(clippy::needless_pass_by_value)]
pub fn str_len(this: Option<SourcedValue>, vs: Vec<SourcedValue>)
    -> Result<SourcedValue, Error>
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
pub fn any_type(this: Option<SourcedValue>, vs: Vec<SourcedValue>)
    -> Result<SourcedValue, Error>
{
    fns::assert_args("type", 0, &vs)
        .context(AssertArgsFailed)?;

    let this = fns::assert_this(this)
        .context(AssertThisFailed)?;

    let s = render_type(&this.v);

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
