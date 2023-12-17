// Copyright 2023 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use crate::eval::error::Error;
use crate::eval::value;
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

        Value::Bool(b) => {
            s += &format!("{}", b);
        },

        Value::Int(n) => {
            s += &format!("{}", n);
        },

        Value::Str(raw_str) => {
            let rendered_str =
                match String::from_utf8(raw_str.clone()) {
                    Ok(p) => p,
                    Err(e) => return Err(Error::BuiltinFuncErr{msg: format!(
                        "couldn't convert error message to UTF-8: {}",
                        e,
                    )}),
                };

            s += &rendered_str;
        },

        Value::List(items) => {
            s += "[\n";
            for item in items {
                let rendered_item = render(item)?;
                let indented = rendered_item.replace('\n', "\n    ");
                s += &format!("    {},\n", indented);
            }
            s += "]";
        },

        Value::Object(props) => {
            s += "{\n";
            for (name, prop) in props {
                let rendered_prop = render(prop)?;
                let indented = rendered_prop.replace('\n', "\n    ");
                s += &format!("    \"{}\": {},\n", name, indented);
            }
            s += "}";
        },

        Value::BuiltInFunc{..} => {
            s += "<built-in function>";
        },
    }

    Ok(s.to_string())
}
