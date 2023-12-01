// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use snafu::Snafu;

// TODO Ideally `Error` would be defined in `src/eval/mod.rs`, since these are
// errors that occur during evaluation. However, we define it here because
// `Value::BuiltInFunc` refers to it. We could make the error type for
// `Value::BuiltInFunc` generic, but this generic type would spread throughout
// the codebase for little benefit, so we take the current approach for now.
#[derive(Clone, Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    #[snafu(display("value is not a function"))]
    CannotCallNonFunc{v: Value},
    #[snafu(display("'{}' is not defined", name))]
    Undefined{name: String},
    #[snafu(display("cannot bind to {}", descr))]
    InvalidBindTarget{descr: String},
    #[snafu(display("'{}' is bound multiple times in this binding", name))]
    AlreadyInBinding{name: String},
    #[snafu(display("'{}' is already defined in the current scope", name))]
    AlreadyInScope{name: String},

    #[snafu(display("{}", msg))]
    BuiltinFuncErr{msg: String},

    EvalProgFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtsInNewScopeFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtsWithScopeStackFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtsFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
    EvalStmtFailed{
        #[snafu(source(from(Error, Box::new)))]
        source: Box<Error>,
    },
}

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

    Str(Str),

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

pub type Object = HashMap<String, ValRefWithSource>;

pub type BuiltinFunc =
    fn(Option<ValRefWithSource>, List) -> Result<ValRefWithSource, Error>;

pub fn new_null() -> ValRefWithSource {
    new_val_ref(Value::Null)
}

pub fn new_str_from_string(s: String) -> ValRefWithSource {
    new_val_ref(Value::Str(s.into_bytes()))
}

pub fn new_built_in_func(f: BuiltinFunc) -> ValRefWithSource {
    new_val_ref(Value::BuiltInFunc{f})
}
