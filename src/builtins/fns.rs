// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use snafu::ResultExt;

use crate::eval::error::AssertArgsFailed;
use crate::eval::error::AssertNoThisFailed;
use crate::eval::error::Error;
use crate::eval::value;
use crate::eval::value::List;
use crate::eval::value::ValRefWithSource;
use crate::eval::value::Value;

#[allow(clippy::needless_pass_by_value)]
pub fn print(this: Option<ValRefWithSource>, vs: List)
    -> Result<ValRefWithSource, Error>
{
    assert_args("print", 1, &vs)
        .context(AssertArgsFailed)?;

    assert_no_this(&this)
        .context(AssertNoThisFailed)?;

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

        Value::BuiltinFunc{name, ..} => {
            s += &format!("<built-in function '{}'>", name);
        },

        Value::Func{name, ..} => {
            s += &format!("<function '{:?}'>", name);
        },
    }

    Ok(s.to_string())
}

// `assert_args` asserts that the correct number of arguments were passed for
// built-in functions.
pub fn assert_args(fn_name: &str, exp_args: usize, args: &List)
    -> Result<(), Error>
{
    if args.len() != exp_args {
        let mut plural = "";
        if exp_args != 1 {
            plural = "s";
        }

        return Err(Error::BuiltinFuncErr{msg: format!(
            "`{}` only takes {} argument{} (got {})",
            fn_name,
            exp_args,
            plural,
            args.len(),
        )})
    }

    Ok(())
}

pub fn assert_no_this(this: &Option<ValRefWithSource>)
    -> Result<(), Error>
{
    if this.is_none() {
        Ok(())
    } else {
        Err(Error::Dev{msg: "'this' shouldn't exist".to_string()})
    }
}

pub fn assert_this(this: Option<ValRefWithSource>)
    -> Result<ValRefWithSource, Error>
{
    if let Some(v) = this {
        Ok(v)
    } else {
        Err(Error::Dev{msg: "'this' doesn't exist".to_string()})
    }
}

pub fn assert_str(val_name: &str, v: &ValRefWithSource)
    -> Result<String, Error>
{
    let unlocked_value = &v.lock().unwrap().v;

    if let Value::Str(raw_str) = unlocked_value {
        match String::from_utf8(raw_str.clone()) {
            Ok(s) => Ok(s),
            Err(e) => Err(Error::BuiltinFuncErr{msg: format!(
                "couldn't convert `{}` string to UTF-8: {}",
                val_name,
                e,
            )}),
        }
    } else {
        // TODO Add type information for the received type.
        let m = "dev err: expected 'string'";

        Err(Error::Dev{msg: m.to_string()})
    }
}
