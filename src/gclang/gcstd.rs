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
        library_function!(library += pop(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value)] => Value::String(value[..value.len().max(1) - 1].to_owned()),
                [Value::Array(value)] => Value::Array(value[..value.len().max(1) - 1].to_owned()),
                _ => bail!(r#"Usage: string = pop(string);"#)
            })
        });
        library_function!(library += remove(_scopes, args) {
            let mut args = args;
            Ok(match args.as_mut_slice() {
                [Value::String(value), Value::Int(index)] => {
                    if *index < 0 || *index as usize > value.len() {
                        bail!("Remove index is out of bounds! Index: {}", index);
                    }
                    value.remove(*index as _);
                    Value::String(value.clone())
                }
                [Value::Array(value), Value::Int(index)] => {
                    if *index < 0 || *index as usize > value.len() {
                        bail!("Remove index is out of bounds! Index: {}", index);
                    }
                    value.remove(*index as _);
                    Value::Array(value.clone())
                }
                [Value::Table(value), index] => {
                    value.remove(index).context("Removing non-existing key from table!")?;
                    Value::Table(value.clone())
                }
                _ => bail!(r#"Usage: string = remove("Some text", 8); or table = remove({{ a = 1; }}, "a");"#)
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
            };
            Ok(match &args[..] {
                [Value::String(filename)] => Value::String(String::from(*files.get(filename.as_str()).context(format!("File '{}' not found!", filename))?)),
                _ => bail!(r#"Usage: builtin_file("bash.gc")"#)
            })
        });
        library
    }
}
