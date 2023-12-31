// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use ast::Block;
use eval::Error;
use eval::Expr;
use super::scope::ScopeStack;

// TODO Consider renaming to `new_val_ref_with_no_source`.
pub fn new_val_ref(v: Value) -> ValRefWithSource {
    Arc::new(Mutex::new(ValWithSource{
        v,
        source: None,
    }))
}

// `ValRefWithSource` is intended to be used as a regular `ValRef` would, but
// it includes the most recent object it was referenced from. For example, in
// the case of `x['f']`, the `ValRef` is the value stored at the location
// `'f'`, and the `source` of this value is `x`.
pub type ValRefWithSource = Arc<Mutex<ValWithSource>>;

#[derive(Clone, Debug)]
pub struct ValWithSource {
    pub v: Value,
    pub source: Option<ValRefWithSource>,
}

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
    new_val_ref(Value::Null)
}

pub fn new_bool(b: bool) -> ValRefWithSource {
    new_val_ref(Value::Bool(b))
}

pub fn new_int(n: i64) -> ValRefWithSource {
    new_val_ref(Value::Int(n))
}

pub fn new_str(s: Str) -> ValRefWithSource {
        new_val_ref(Value::Str(s))
}

pub fn new_str_from_string(s: String) -> ValRefWithSource {
    new_val_ref(Value::Str(s.into_bytes()))
}

pub fn new_list(list: List) -> ValRefWithSource {
    new_val_ref(Value::List(list))
}

pub fn new_object(object: Object) -> ValRefWithSource {
    new_val_ref(Value::Object(object))
}

pub fn new_func(
    name: Option<String>,
    args: Vec<Expr>,
    stmts: Block,
    closure: ScopeStack,
)
    -> ValRefWithSource
{
    new_val_ref(Value::Func{name, args, stmts, closure})
}

pub fn new_built_in_func(name: String, f: BuiltinFunc) -> ValRefWithSource {
    new_val_ref(Value::BuiltinFunc{name, f})
}
