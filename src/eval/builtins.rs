// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

pub use super::value::ObjectRef;
pub use super::value::Value;

pub struct Builtins {
    pub std: ObjectRef,
    pub type_functions: TypeFunctions,
}

pub struct TypeFunctions {
    pub bools: ObjectRef,
    pub ints: ObjectRef,
    pub strs: ObjectRef,
    pub lists: ObjectRef,
    pub objects: ObjectRef,
    pub funcs: ObjectRef,
}
