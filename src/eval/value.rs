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

pub fn new_val_ref_with_no_source(v: Value) -> ValRefWithSource {
    ValRefWithSource{
        v,
        source: None,
    }
}

pub fn new_val_ref_with_source(v: Value, source: Value) -> ValRefWithSource {
    ValRefWithSource{
        v,
        source: Some(source),
    }
}

// `ValRefWithSource` is intended to be used as a regular `ValRef` would, but
// it includes the most recent object it was referenced from. For example, in
// the case of `x['f']`, the `ValRef` is the value stored at the location
// `'f'`, and the `source` of this value is `x`.
//
// Note that `source` will only be an `Object` when `v` has been accessed using
// a property/index access, but it can be of any type when `v` has been
// accessed as a type property.
#[derive(Clone, Debug)]
pub struct ValRefWithSource {
    pub v: Value,
    pub source: Option<Value>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Null,

    Bool(bool),
    Int(i64),
    Str(Str),

    List(ListRef),
    Object(ObjectRef),

    BuiltinFunc{name: String, f: BuiltinFunc},
    Func(Arc<Mutex<Func>>),
}

pub type Str = Vec<u8>;

pub type ListRef = Arc<Mutex<List>>;

pub type List = Vec<ValRefWithSource>;

pub type ObjectRef = Arc<Mutex<Object>>;

// We use a `BTreeMap` instead of a `HashMap` for representing `Object`s in
// order to get a deterministic order when printing objects, which simplifies
// "output" tests.
pub type Object = BTreeMap<String, ValRefWithSource>;

pub type BuiltinFunc =
    fn(Option<ValRefWithSource>, Vec<ValRefWithSource>)
        -> Result<ValRefWithSource, Error>;

#[derive(Clone, Debug)]
pub struct Func {
    pub name: Option<String>,
    pub args: Vec<Expr>,
    pub collect_args: bool,
    pub stmts: Block,
    pub closure: ScopeStack,
}

pub fn new_null() -> ValRefWithSource {
    new_val_ref_with_no_source(Value::Null)
}

pub fn new_bool(b: bool) -> ValRefWithSource {
    new_val_ref_with_no_source(Value::Bool(b))
}

pub fn new_int(n: i64) -> ValRefWithSource {
    new_val_ref_with_no_source(Value::Int(n))
}

pub fn new_str(s: Str) -> ValRefWithSource {
    new_val_ref_with_no_source(Value::Str(s))
}

pub fn new_str_from_string(s: String) -> ValRefWithSource {
    new_val_ref_with_no_source(Value::Str(s.into_bytes()))
}

pub fn new_list(list: List) -> ValRefWithSource {
    new_val_ref_with_no_source(Value::List(Arc::new(Mutex::new(list))))
}

pub fn new_object(object: Object) -> ValRefWithSource {
    new_val_ref_with_no_source(Value::Object(Arc::new(Mutex::new(object))))
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
    new_val_ref_with_no_source(
        Value::Func(Arc::new(Mutex::new(Func{
            name,
            args,
            collect_args,
            stmts,
            closure,
        }))),
    )
}

pub fn new_built_in_func(name: String, f: BuiltinFunc) -> ValRefWithSource {
    new_val_ref_with_no_source(Value::BuiltinFunc{name, f})
}

pub fn ref_eq<T>(a: &Arc<Mutex<T>>, b: &Arc<Mutex<T>>) -> bool {
    Arc::ptr_eq(a, b)
}
