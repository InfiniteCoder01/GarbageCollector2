use std::collections::HashMap;

mod executor;
mod parser;

// * ------------------------------------- Spec ------------------------------------- * //
#[derive(Debug)]
pub struct Program {
    scope: Scope,
}

#[derive(Debug, Default)]
pub struct Scope {
    statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Expression(Expression),
}

#[derive(Debug)]
pub enum Expression {
    Constant(Value),
    Variable(String),
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
    },
}

#[derive(Clone, Debug)]
pub enum Value {
    Int(i32),
    String(String),
    Void,
}

// * Exec
pub struct Executor {
    pub env: Environment,
    program: Program,
}

#[derive(Default)]
pub struct Environment {
    pub global: HashMap<String, Value>,
    pub local: Vec<HashMap<String, Value>>,
}

pub type BuiltinFunction = Box<dyn FnMut(&mut Environment, Vec<Value>) -> Value>;

#[derive(Default)]
pub struct Builtins {
    pub functions: HashMap<String, BuiltinFunction>,
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::String(value) => value.clone(),
            Value::Void => "Nothing".to_owned(),
        }
    }
}

// * ----------------------------------- Programs ----------------------------------- * //
impl Program {
    pub fn bash() -> Self {
        Program::parse(include_str!("bash.gc")).expect("Failed to build bash")
    }
}
