// Copyright 2023-2024 Sean Kelleher. All rights reserved.
// Use of this source code is governed by an MIT
// licence that can be found in the LICENCE file.

use snafu::ResultExt;

use crate::eval::error::AssertArgsFailed;
use crate::eval::error::AssertNoThisFailed;
use crate::eval::error::Error;
use crate::eval::value;
use crate::eval::value::Func;
use crate::eval::value::SourcedValue;
use crate::eval::value::Value;

// TODO Duplicated from `src/eval/mod.rs`.
macro_rules! deref {
    ( $val_ref_with_source:ident ) => {
        *$val_ref_with_source.lock().unwrap()
    };
}

#[allow(clippy::needless_pass_by_value)]
pub fn print(this: Option<SourcedValue>, args: Vec<SourcedValue>)
    -> Result<SourcedValue, Error>
{
    assert_args("print", 1, &args)
        .context(AssertArgsFailed)?;

    assert_no_this(&this)
        .context(AssertNoThisFailed)?;

    let s = render(&args[0])?;

    println!("{}", s);

    Ok(value::new_null())
}

fn render(v: &SourcedValue) -> Result<String, Error> {
    let mut s = String::new();

    match v.v.clone() {
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
                match String::from_utf8(raw_str) {
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
            for item in &deref!(items) {
                let rendered_item = render(item)?;
                let indented = rendered_item.replace('\n', "\n    ");
                s += &format!("    {},\n", indented);
            }
            s += "]";
        },

        Value::Object(props) => {
            s += "{\n";
            for (name, prop) in &deref!(props) {
                let rendered_prop = render(prop)?;
                let indented = rendered_prop.replace('\n', "\n    ");
                s += &format!("    \"{}\": {},\n", name, indented);
            }
            s += "}";
        },

        Value::BuiltinFunc{name, ..} => {
            s += &format!("<built-in function '{}'>", name);
        },

        Value::Func(f) => {
            let Func{name, ..} = &deref!(f);

            s += &format!("<function '{:?}'>", name);
        },
    }

    Ok(s.to_string())
}

// `assert_args` asserts that the correct number of arguments were passed for
// built-in functions.
pub fn assert_args(fn_name: &str, exp_args: usize, args: &[SourcedValue])
    -> Result<(), Error>
{
    let args_len = args.len();

    if args_len != exp_args {
        let mut plural = "";
        if exp_args != 1 {
            plural = "s";
        }

        return Err(Error::BuiltinFuncErr{msg: format!(
            "`{}` only takes {} argument{} (got {})",
            fn_name,
            exp_args,
            plural,
            args_len,
        )})
    }

    Ok(())
}

pub fn assert_no_this(this: &Option<SourcedValue>) -> Result<(), Error> {
    if this.is_none() {
        Ok(())
    } else {
        Err(Error::Dev{msg: "'this' shouldn't exist".to_string()})
    }
}

pub fn assert_this(this: Option<SourcedValue>) -> Result<SourcedValue, Error> {
    if let Some(v) = this {
        Ok(v)
    } else {
        Err(Error::Dev{msg: "'this' doesn't exist".to_string()})
    }
}

pub fn assert_str(val_name: &str, v: &SourcedValue) -> Result<String, Error> {
    if let Value::Str(raw_str) = &v.v {
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
