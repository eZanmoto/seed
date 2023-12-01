// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use crate::eval::value;
use crate::eval::value::Error;
use crate::eval::value::List;
use crate::eval::value::ValRefWithSource;
use crate::eval::value::Value;

#[allow(clippy::needless_pass_by_value)]
pub fn print(_this: Option<ValRefWithSource>, vs: List)
    -> Result<ValRefWithSource, Error>
{
    if vs.len() != 1 {
        return Err(Error::BuiltinFuncErr{msg: format!(
            "`print` only takes 1 argument (got {})",
            vs.len(),
        )})
    }

    let s = render(&vs[0])?;

    println!("{}", s);

    Ok(value::new_null())
}

fn render(v: &ValRefWithSource) -> Result<String, Error> {
    let mut s = String::new();

    match &v.lock().unwrap().v {
        Value::Null => {
            s += "<null>";
        },

        Value::Str(raw_str) => {
            let s_ =
                match String::from_utf8(raw_str.clone()) {
                    Ok(p) => p,
                    Err(e) => return Err(Error::BuiltinFuncErr{msg: format!(
                        "couldn't convert error message to UTF-8: {}",
                        e,
                    )}),
                };

            s += &s_;
        },

        Value::BuiltInFunc{..} => {
            s += "<built-in function>";
        },
    }

    Ok(s.to_string())
}
