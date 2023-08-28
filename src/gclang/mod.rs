mod executor;
mod parser;
mod std;

pub use executor::Library;
pub use executor::Value;
pub use parser::Program;
pub use executor::Scopes;

#[macro_export]
macro_rules! library_function {
    ($library: ident += $name: ident ($scopes: ident, $args: ident) $function: block) => {
        $library.functions.insert(
            String::from(stringify!($name)),
            Box::from(|$scopes: &mut gclang::Scopes, $args: Vec<Value>| $function),
        );
    };
}

pub use library_function;

// * ----------------------------------- Programs ----------------------------------- * //
pub fn bash() -> Program {
    Program::parse(include_str!("programs/bash.gc"))
}
