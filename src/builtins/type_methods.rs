// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::BTreeMap;

use crate::eval::builtins::TypeMethods;
use crate::eval::value::ValRefWithSource;

pub fn type_methods() -> TypeMethods {
    TypeMethods{
        strs: BTreeMap::<String, ValRefWithSource>::new(),
        funcs: BTreeMap::<String, ValRefWithSource>::new(),
    }
}
