use super::*;
use anyhow::*;

impl Statement {
    fn execute(&self, env: &mut Environment, builtins: &mut Builtins) -> Result<()> {
        match self {
            Statement::Expression(expr) => {
                expr.evaluate(env, builtins)?;
            }
        }
        Ok(())
    }
}

impl Expression {
    fn evaluate(&self, env: &mut Environment, builtins: &mut Builtins) -> Result<Value> {
        Ok(match self {
            Self::Constant(value) => value.clone(),
            Self::Variable(name) => env.get(name)?,
            Self::FunctionCall { name, arguments } => {
                let mut evaluated_arguments = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    evaluated_arguments.push(argument.evaluate(env, builtins)?);
                }
                if let Some(function) = builtins.functions.get_mut(name) {
                    function(env, evaluated_arguments)
                } else {
                    bail!("Function {} not found!", name);
                }
            }
        })
    }
}

impl Scope {
    pub fn execute(&self, env: &mut Environment, builtins: &mut Builtins) -> Result<()> {
        env.local.push(HashMap::new());
        for statement in &self.statements {
            statement.execute(env, builtins)?;
        }
        env.local.pop();
        Ok(())
    }
}

impl Environment {
    pub fn get(&self, name: &str) -> Result<Value> {
        for local in self.local.iter().rev() {
            if let Some(value) = local.get(name) {
                return Ok(value.clone());
            }
        }
        self.global
            .get(name)
            .context(format!("Undefined variable '{}'!", name))
            .cloned()
    }

    pub fn get_global(&mut self, name: &str, default: Value) -> &mut Value {
        self.global.entry(name.to_owned()).or_insert(default)
    }
}

impl Program {
    pub fn execute(&self, env: &mut Environment, builtins: &mut Builtins) -> Result<()> {
        self.scope.execute(env, builtins)
    }
}

impl Executor {
    pub fn new(program: Program) -> Self {
        Self {
            env: Environment::default(),
            program,
        }
    }

    pub fn execute(&mut self, builtins: &mut Builtins) -> Result<()> {
        self.program.execute(&mut self.env, builtins)
    }
}
