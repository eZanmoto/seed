// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use ast::Block;
use eval::Error;
use eval::Expr;
use super::scope::ScopeStack;

pub fn new_val_ref_from_value(v: Value) -> ValRefWithSource {
    ValRefWithSource{
        v: Arc::new(Mutex::new(v)),
        source: None,
    }
}

pub fn new_val_ref_with_no_source(v: ValRef) -> ValRefWithSource {
    ValRefWithSource{
        v,
        source: None,
    }
}

pub fn new_val_ref_with_source(v: ValRef, source: ValRef)
    -> ValRefWithSource
{
    ValRefWithSource{
        v,
        source: Some(source),
    }
}

// `ValRefWithSource` is intended to be used as a regular `ValRef` would, but
// it includes the most recent object it was referenced from. For example, in
// the case of `x['f']`, the `ValRef` is the value stored at the location
// `'f'`, and the `source` of this value is `x`.
#[derive(Clone, Debug)]
pub struct ValRefWithSource {
    pub v: ValRef,
    pub source: Option<ValRef>,
}

type ValRef = Arc<Mutex<Value>>;

#[derive(Clone, Debug)]
pub enum Value {
    Null,

    Bool(bool),
    Int(i64),
    Str(Str),

    List(List),
    Object(Object),

    BuiltinFunc{name: String, f: BuiltinFunc},
    Func{
        name: Option<String>,
        args: Vec<Expr>,
        collect_args: bool,
        stmts: Block,
        closure: ScopeStack,
    },
}

pub type Str = Vec<u8>;

pub type List = Vec<ValRefWithSource>;

// We use a `BTreeMap` instead of a `HashMap` for representing `Object`s in
// order to get a deterministic order when printing objects, which simplifies
// "output" tests.
pub type Object = BTreeMap<String, ValRefWithSource>;

pub type BuiltinFunc =
    fn(Option<ValRefWithSource>, List) -> Result<ValRefWithSource, Error>;

pub fn new_null() -> ValRefWithSource {
    new_val_ref_from_value(Value::Null)
}

pub fn new_bool(b: bool) -> ValRefWithSource {
    new_val_ref_from_value(Value::Bool(b))
}

pub fn new_int(n: i64) -> ValRefWithSource {
    new_val_ref_from_value(Value::Int(n))
}

pub fn new_str(s: Str) -> ValRefWithSource {
        new_val_ref_from_value(Value::Str(s))
}

pub fn new_str_from_string(s: String) -> ValRefWithSource {
    new_val_ref_from_value(Value::Str(s.into_bytes()))
}

pub fn new_list(list: List) -> ValRefWithSource {
    new_val_ref_from_value(Value::List(list))
}

pub fn new_object(object: Object) -> ValRefWithSource {
    new_val_ref_from_value(Value::Object(object))
}

pub fn new_func(
    name: Option<String>,
    args: Vec<Expr>,
    collect_args: bool,
    stmts: Block,
    closure: ScopeStack,
)
    -> ValRefWithSource
{
    new_val_ref_from_value(
        Value::Func{name, args, collect_args, stmts, closure},
    )
}

pub fn new_built_in_func(name: String, f: BuiltinFunc) -> ValRefWithSource {
    new_val_ref_from_value(Value::BuiltinFunc{name, f})
}
