// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use ast::Location;
use value::ValRefWithSource;

#[derive(Clone, Debug)]
pub struct ScopeStack(Vec<Arc<Mutex<Scope>>>);

pub type Scope = HashMap<String, (ValRefWithSource, Location)>;

impl ScopeStack {
    pub fn new(scopes: Vec<Arc<Mutex<Scope>>>) -> ScopeStack {
        ScopeStack(scopes)
    }

    pub fn new_from_push(&self, scope: Scope) -> ScopeStack {
        let mut scopes = self.0.clone();
        scopes.push(Arc::new(Mutex::new(scope)));

        ScopeStack::new(scopes)
    }

    // `declare` returns `Err` if `name` is already defined in the current
    // scope, and the `Err` will contain the location of the previous
    // definition.
    pub fn declare(&mut self, name: &str, loc: Location, v: ValRefWithSource)
        -> Result<(), Location>
    {
        let mut cur_scope =
            self.0.last()
                .expect("`ScopeStack` stack shouldn't be empty")
                .lock()
                .unwrap();

        if let Some((_, loc)) = cur_scope.get(name) {
            return Err(*loc);
        }

        cur_scope.insert(name.to_string(), (v, loc));

        Ok(())
    }

    pub fn get(&self, name: &String) -> Option<ValRefWithSource> {
        for scope in self.0.iter().rev() {
            let unlocked_scope = scope.lock().unwrap();
            if let Some((v, _)) = unlocked_scope.get(name) {
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

            if let Some((slot, _)) = unlocked_scope.get_mut(name) {
                set(slot, v);

                return true;
            }
        }

        false
    }
}

pub fn set(slot: &mut ValRefWithSource, v: ValRefWithSource) {
    *slot = v;
}
