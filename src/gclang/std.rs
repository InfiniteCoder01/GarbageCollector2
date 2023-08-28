use super::*;
use anyhow::*;

impl Library<'_> {
    pub fn with_std() -> Self {
        let mut library = Self::default();
        library_function!(library += eval(scopes, args) {
            match &args[..] {
                [Value::String(code)] => Program::parse(code).eval(scopes, &mut Library::with_std())?,
                _ => bail!(r#"Usage: eval("some_global_variable = \"Evaluated\";");"#),
            }
            Ok(Value::Unit)
        });
        library_function!(library += len(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value)] => Value::Int(value.len() as _),
                [Value::Table(value)] => Value::Int(value.len() as _),
                _ => bail!(r#"Usage: len("Some text") or len({{0 = "Some table";}})"#)
            })
        });
        library_function!(library += contains(_scopes, args) {
            Ok(match &args[..] {
                [Value::String(value), Value::String(searched)] => Value::Bool(value.contains(searched)),
                _ => bail!(r#"Usage: contains("Some text", "te")"#)
            })
        });
        library
    }
}
