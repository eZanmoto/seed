// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

pub use super::value::Object;
pub use super::value::Value;

pub struct Builtins {
    pub std: Object,
    pub type_methods: TypeMethods,
}

pub struct TypeMethods {
    pub strs: Object,
    pub funcs: Object,
}
