use super::*;
use anyhow::*;

impl Statement {
    fn execute(&self, env: &mut Environment, builtins: &mut Builtins) -> Result<()> {
        match self {
            Statement::GlobalVariable(name, value) => {
                if !env.global.contains_key(name) {
                    let value = value.evaluate(env, builtins)?;
                    env.get_global(name, value);
                }
            }
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
            Self::Assignment(name, value) => {
                let value = value.evaluate(env, builtins)?;
                let variable = env.get_mut(name)?;
                match (&variable, &value) {
                    (Value::Int(_), Value::Int(_)) | (Value::String(_), Value::String(_)) => (),
                    _ => bail!("This \"language\" is statically typed! You're assigning to a variable '{}'.", name),
                }
                *variable = value;
                variable.clone()
            }
            Self::Binary(lhs, op, rhs) => {
                let lhs = lhs.evaluate(env, builtins)?;
                let rhs = rhs.evaluate(env, builtins)?;
                match op.as_str() {
                    "+" => {
                        if matches!(lhs, Value::String(_)) || matches!(rhs, Value::String(_)) {
                            Value::String(lhs.to_string() + rhs.to_string().as_str())
                        } else {
                            match (lhs, rhs) {
                                (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
                                _ => bail!("Mismatched types!"),
                            }
                        }
                    }
                    "-" => match (lhs, rhs) {
                        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs - rhs),
                        _ => bail!("Mismatched types!"),
                    },
                    "*" => match (lhs, rhs) {
                        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs * rhs),
                        (Value::String(lhs), Value::Int(rhs)) => {
                            Value::String(lhs.repeat(rhs as _))
                        }
                        _ => bail!("Mismatched types!"),
                    },
                    "/" => match (lhs, rhs) {
                        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs / rhs),
                        _ => bail!("Mismatched types!"),
                    },
                    "%" => match (lhs, rhs) {
                        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs % rhs),
                        _ => bail!("Mismatched types!"),
                    },
                    "==" => Value::Bool(lhs == rhs),
                    "!=" => Value::Bool(lhs != rhs),
                    ">" => match (lhs, rhs) {
                        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs > rhs),
                        _ => bail!("Mismatched types!"),
                    },
                    op => bail!("Internal error: unknown binary operator '{}'!", op),
                }
            }

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

    pub fn get_mut(&mut self, name: &str) -> Result<&mut Value> {
        for local in self.local.iter_mut().rev() {
            if let Some(value) = local.get_mut(name) {
                return Ok(value);
            }
        }
        self.global
            .get_mut(name)
            .context(format!("Undefined variable '{}'!", name))
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
