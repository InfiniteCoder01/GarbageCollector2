use super::*;
use map_macro::*;

impl Library<'_> {
    pub fn with_std() -> Self {
        let mut library = Self::default();
        // * ------------------------------------- Math ------------------------------------- * //
        library_function!(library += min(_scopes, args) {
            Ok(Value::Int(**args.iter().map(|arg| match arg {
                Value::Int(arg) => Ok(arg),
                _ => bail!("Usage: min(1, 2, 3)")
            }).collect::<std::result::Result<Vec<_>, _>>()?.iter().min().context("Usage: min(1, 2, 3)")?))
        });
        library_function!(library += max(_scopes, args) {
            Ok(Value::Int(**args.iter().map(|arg| match arg {
                Value::Int(arg) => Ok(arg),
                _ => bail!("Usage: max(1, 2, 3)")
            }).collect::<std::result::Result<Vec<_>, _>>()?.iter().max().context("Usage: max(1, 2, 3)")?))
        });
        // * ---------------------------------- Containers ---------------------------------- * //
        library_function!(library += len(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value)] => Value::Int(value.len() as _),
                [Value::Array(value)] => Value::Int(value.len() as _),
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
                    if *index < 0 || *index as usize >= value.len() {
                        bail!(late effect "Remove index is out of bounds! Index: {}", index);
                    }
                    value.remove(*index as _);
                    Value::String(value.clone())
                }
                [Value::Array(value), Value::Int(index)] => {
                    if *index < 0 || *index as usize >= value.len() {
                        bail!(late effect "Remove index is out of bounds! Index: {}", index);
                    }
                    value.remove(*index as _);
                    Value::Array(value.clone())
                }
                [Value::Table(value), index] => {
                    if value.remove(index).is_none() {
                        bail!(late effect "Removing non-existing key from table!");
                    }
                    Value::Table(value.clone())
                }
                _ => bail!(r#"Usage: string = remove("Some text", 8); or table = remove({{ a = 1; }}, "a");"#)
            })
        });
        library_function!(library += contains(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value), Value::String(key)] => Value::Bool(value.contains(key)),
                [Value::Array(value), key] => Value::Bool(value.contains(key)),
                [Value::Table(value), key] => Value::Bool(value.contains_key(key)),
                _ => bail!(r#"Usage: contains("Some text", "te")"#)
            })
        });
        // * ------------------------------------- Misc ------------------------------------- * //
        library_function!(library += trim(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value)] => Value::String(value.trim().to_owned()),
                _ => bail!(r#"Usage: trim("   Some text   ")"#)
            })
        });
        library_function!(library += builtin_filesystem(_scopes, args) {
            ensure!(args.is_empty(), "builtin_filesystem was not ment to be used with args!");

            macro_rules! define_file {
                ($($path: literal = $value: expr;)*) => {
                    Value::Table(btree_map! {
                        $(Value::String(String::from($path)) => $value,)*
                    })
                };
                ($path: literal) => {
                    Value::String(String::from(include_str!($path)))
                };
            }

            macro_rules! embed_file {
                ($content: literal) => {
                    Value::String(String::from($content))
                };
            }

            Ok(define_file! {
                "home" = define_file! {};
                "bin" = define_file! {
                    "gcsh" = define_file!("programs/gcsh.gc");

                    "bash" = define_file!("programs/bash.gc");
                    "fish" = define_file!("programs/bash.gc");
                    "zsh" = define_file!("programs/bash.gc");
                    "sh" = define_file!("programs/bash.gc");

                    "neofetch" = define_file!("programs/neofetch.gc");
                    "sl" = define_file!("programs/sl.gc");

                    "clear" = embed_file!("screen_buffer = \"\";");
                    "ls" = define_file!("programs/ls.gc");
                    "edit" = define_file!("programs/edit.gc");
                };
                "lib" = define_file! {
                    "curses.gc" = define_file!("programs/curses.gc");
                };
            })
        });
        library
    }
}
