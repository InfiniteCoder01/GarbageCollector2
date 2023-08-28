use super::*;
use anyhow::*;

impl Library {
    pub fn with_std() -> Self {
        let mut library = Self::default();
        library_function!(library += eval(scopes, args) {
            ensure!(args.len() == 1, r#"Usage: eval("some_global_variable = \"Evaluated\";");"#);
            let code = ensure_type!(&args[0], String, r#"Usage: eval("some_global_variable = \"Evaluated\";");"#);
                Program::parse(code).eval(scopes, &mut Library::with_std())?;
            Ok(Value::Unit)
        });
        library_function!(library += len(_scopes, args) {
            ensure!(args.len() == 1, r#"Usage: len("Some text") or len({{0 = "Some table";}})"#);
            Ok(match &args[0] {
                Value::String(value) => Value::Int(value.len() as _),
                Value::Table(value) => Value::Int(value.len() as _),
                _ => bail!(r#"Usage: len("Some text") or len({{0 = "Some table";}})"#)
            })
        });
        library
    }
}
