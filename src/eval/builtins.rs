// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

pub use super::value::Object;
pub use super::value::Value;

pub struct Builtins {
    pub std: Object,
    pub type_functions: TypeFunctions,
}

pub struct TypeFunctions {
    pub bools: Object,
    pub ints: Object,
    pub strs: Object,
    pub lists: Object,
    pub objects: Object,
    pub funcs: Object,
}
