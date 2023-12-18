// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use value::ValRefWithSource;

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
    pub fn declare(&mut self, name: &str, v: ValRefWithSource) -> bool
    {
        let mut cur_scope =
            self.0.last()
                .expect("`ScopeStack` stack shouldn't be empty")
                .lock()
                .unwrap();

        if cur_scope.contains_key(name) {
            return false;
        }
        cur_scope.insert(name.to_string(), v);

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


    // `assign` replaces `name` in the topmost scope of this `ScopeStack` and
    // returns `true`, or else it returns `false` if `name` wasn't found in
    // this `ScopeStack`. `assign` returns an error if attempting to assign to
    // a constant binding.
    pub fn assign(&mut self, name: &str, v: ValRefWithSource) -> bool {
        for scope in self.0.iter().rev() {
            let mut unlocked_scope = scope.lock().unwrap();
            if unlocked_scope.get(name).is_some() {
                // This should ideally overwrite the value stored in this
                // variable instead of introducing a new variable with a new
                // binding, but this isn't possible at present with the current
                // structure of `ValRefWithSource`; see the comment above
                // `ValRefWithSource` for more details.
                unlocked_scope.insert(name.to_string(), v);

                return true;
            }
        }

        false
    }
}
