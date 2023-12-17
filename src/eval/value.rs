// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use eval::Error;

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

    BuiltInFunc{f: BuiltinFunc},
}

#[derive(Clone, Debug)]
pub struct ScopeStack(Vec<Arc<Mutex<Scope>>>);

pub type Scope = HashMap<String, ValRefWithSource>;

impl ScopeStack {
    pub fn new(scopes: Vec<Arc<Mutex<Scope>>>) -> ScopeStack {
        ScopeStack(scopes)
    }

    pub fn new_from_push(&self, scope: Scope) -> ScopeStack {
        let mut scopes = self.0.clone();
        scopes.push(Arc::new(Mutex::new(scope)));

        ScopeStack::new(scopes)
    }

    // `declare` returns `false` if `name` is already defined in the current
    // scope.
    pub fn declare(&mut self, name: String, v: ValRefWithSource) -> bool
    {
        let mut cur_scope =
            self.0.last()
                .expect("`ScopeStack` stack shouldn't be empty")
                .lock()
                .unwrap();

        if cur_scope.contains_key(&name) {
            return false;
        }
        cur_scope.insert(name, v);

        true
    }

    pub fn get(&self, name: &String) -> Option<ValRefWithSource> {
        for scope in self.0.iter().rev() {
            let unlocked_scope = scope.lock().unwrap();
            if let Some(v) = unlocked_scope.get(name) {
                // TODO Remove `clone()`.
                return Some(v.clone());
            }
        }

        None
    }
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

pub fn new_str_from_string(s: String) -> ValRefWithSource {
    new_val_ref(Value::Str(s.into_bytes()))
}

pub fn new_list(list: List) -> ValRefWithSource {
    new_val_ref(Value::List(list))
}

pub fn new_object(object: Object) -> ValRefWithSource {
    new_val_ref(Value::Object(object))
}

pub fn new_built_in_func(f: BuiltinFunc) -> ValRefWithSource {
    new_val_ref(Value::BuiltInFunc{f})
}
