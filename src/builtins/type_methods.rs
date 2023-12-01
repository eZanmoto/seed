// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use std::collections::HashMap;

use crate::eval::builtins::TypeMethods;
use crate::eval::value::ValRefWithSource;

pub fn type_methods() -> TypeMethods {
    TypeMethods{
        strs: HashMap::<String, ValRefWithSource>::new(),
        funcs: HashMap::<String, ValRefWithSource>::new(),
    }
}
