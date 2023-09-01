mod executor;
mod gcstd;
mod parser;

pub use anyhow::{anyhow, Context, Error};
pub use executor::Library;
pub use executor::Scopes;
pub use executor::Value;
pub use executor::{bail, ensure, Effect, Exception, Ok, Result};
pub use parser::Program;

#[macro_export]
macro_rules! library_function {
    ($library: ident += $name: ident ($scopes: ident, $args: ident) $function: block) => {
        $library.functions.insert(
            String::from(stringify!($name)),
            Box::from(|$scopes: &mut $crate::gclang::Scopes, $args: Vec<Value>| $function),
        );
    };
}

pub use library_function;

#[macro_export]
macro_rules! ensure_type {
    ($value: expr, $type: ident, $err: literal) => {
        if let $crate::gclang::Value::$type(value) = $value {
            value
        } else {
            bail!($err);
        }
    };
}

pub use ensure_type;

// * ----------------------------------- Programs ----------------------------------- * //
pub fn gcsh() -> Program {
    Program::parse(include_str!("programs/gcsh.gc")).expect("Failed to compile gcsh!")
}
