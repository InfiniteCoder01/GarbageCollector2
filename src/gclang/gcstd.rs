use super::*;
use anyhow::*;
use map_macro::*;

impl Library<'_> {
    pub fn with_std() -> Self {
        let mut library = Self::default();
        library_function!(library += len(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value)] => Value::Int(value.len() as _),
                [Value::Table(value)] => Value::Int(value.len() as _),
                _ => bail!(r#"Usage: len("Some text") or len({{0 = "Some table";}})"#)
            })
        });
        library_function!(library += contains(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value), Value::String(key)] => Value::Bool(value.contains(key)),
                [Value::Table(value), key] => Value::Bool(value.contains_key(key)),
                _ => bail!(r#"Usage: contains("Some text", "te")"#)
            })
        });
        library_function!(library += builtin_file(_scopes, args) {
            let files = hash_map! {
                "gcsh.gc" => include_str!("programs/gcsh.gc"),
                "bash.gc" => include_str!("programs/bash.gc"),
                "fish.gc" => include_str!("programs/fish.gc"),
                "sh.gc" => include_str!("programs/sh.gc"),
            };
            Ok(match &args[..] {
                [Value::String(filename)] => Value::String(String::from(*files.get(filename.as_str()).context(format!("File '{}' not found!", filename))?)),
                _ => bail!(r#"Usage: builtin_file("bash.gc")"#)
            })
        });
        library
    }
}
