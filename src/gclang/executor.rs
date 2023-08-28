use crate::ensure_type;

use super::parser::*;
use anyhow::*;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Int(i32),
    Bool(bool),
    String(String),
    Table(BTreeMap<Value, Value>),
    Unit,
    Never,
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            Value::String(value) => value.clone(),
            Value::Table(value) => format!("{:#?}", value),
            Value::Unit => "Unit".to_owned(),
            Value::Never => "Never".to_owned(),
        }
    }
}

#[derive(Default)]
pub struct Scopes {
    global: HashMap<String, Value>,
    local: Vec<StackFrame>,
}

#[derive(Default)]
struct StackFrame {
    variables: HashMap<String, Value>,
    functions: HashMap<String, Function>,
}

#[derive(Clone)]
struct Function {
    args: Vec<ArgDef>,
    statement: Statement,
}

impl Scopes {
    fn get(&self, name: &str) -> Result<&Value> {
        match self
            .local
            .iter()
            .rev()
            .find_map(|frame| frame.variables.get(name))
        {
            Some(value) => Ok(value),
            None => match self.global.get(name) {
                Some(value) => Ok(value),
                None => bail!("Variable `{name}` not found!"),
            },
        }
    }

    fn get_mut(&mut self, name: &str) -> Result<&mut Value> {
        match self
            .local
            .iter_mut()
            .rev()
            .find_map(|frame| frame.variables.get_mut(name))
        {
            Some(value) => Ok(value),
            None => match self.global.get_mut(name) {
                Some(value) => Ok(value),
                None => bail!("Variable `{name}` not found!"),
            },
        }
    }

    fn get_function(&self, name: &str) -> Result<&Function> {
        match self
            .local
            .iter()
            .rev()
            .find_map(|frame| frame.functions.get(name))
        {
            Some(functoin) => Ok(functoin),
            None => bail!("Function `{name}` not found!"),
        }
    }

    pub fn get_global_or_insert(&mut self, name: &str, default: Value) -> &mut Value {
        self.global.entry(name.to_owned()).or_insert(default)
    }
}

type LibFunction<'a> = Box<dyn FnMut(&mut Scopes, Vec<Value>) -> Result<Value> + 'a>;

#[derive(Default)]
pub struct Library<'a> {
    pub functions: HashMap<String, LibFunction<'a>>,
}

trait Eval {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value>;
}

impl Eval for Statement {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        match self {
            Statement::GlobalDecl(_, name, _, value, _) => {
                if !scopes.global.contains_key(name.ident()) {
                    let value = value.eval(scopes, library)?;
                    scopes.global.insert(name.ident().to_owned(), value);
                }
                Ok(Value::Unit)
            }
            Statement::LocalDecl(_, name, _, value, _) => {
                let value = value.eval(scopes, library)?;
                match scopes
                    .local
                    .last_mut()
                    .context("Nowhere to create a local variable!")?
                    .variables
                    .entry(name.ident().to_owned())
                {
                    std::collections::hash_map::Entry::Occupied(_) => {
                        bail!("Variable '{}' already exists!", name.ident())
                    }
                    std::collections::hash_map::Entry::Vacant(variable) => {
                        variable.insert(value);
                    }
                }
                Ok(Value::Unit)
            }
            Statement::FnDecl(decl) => decl.eval(scopes, library),
            Statement::If(statement) => statement.eval(scopes, library),
            Statement::Block(statement) => statement.eval(scopes, library),
            Statement::Expression(expr) => expr.eval(scopes, library),
            Statement::End(_) => Ok(Value::Never),
        }
    }
}

impl Eval for FnDecl {
    fn eval(&self, scopes: &mut Scopes, _library: &mut Library) -> Result<Value> {
        match scopes
            .local
            .last_mut()
            .context("Nowhere to create a local function!")?
            .functions
            .entry(self.name.ident().to_owned())
        {
            std::collections::hash_map::Entry::Occupied(_) => {
                bail!("Function '{}' already exists!", self.name.ident())
            }
            std::collections::hash_map::Entry::Vacant(function) => {
                function.insert(Function {
                    args: self.args.0.clone(),
                    statement: self.statement.clone(),
                });
            }
        }
        Ok(Value::Unit)
    }
}

impl Eval for IfStatement {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let condition = self.condition.eval(scopes, library)?;
        let condition = ensure_type!(
            condition,
            Bool,
            "If's can be only used with conditions of type bool"
        );
        if condition {
            self.statement.eval(scopes, library)?;
        } else if let Some(else_statement) = &self.else_statement {
            else_statement.statement.eval(scopes, library)?;
        }
        Ok(Value::Unit)
    }
}

impl Eval for BlockStatement {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        if let Some(statements) = &self.statements {
            for statement in &statements.0 {
                statement.eval(scopes, library)?;
            }
        }
        Ok(Value::Unit)
    }
}

impl Eval for ExpressionStatement {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        match self {
            ExpressionStatement::Expression(expr, _) => expr.eval(scopes, library),
            ExpressionStatement::Assign(expr) => expr.eval(scopes, library),
        }
    }
}

macro_rules! eval_expression {
    ($type: ident $($lhs_type: ident ($lhs: ident) $rhs_type: ident ($rhs: ident) => $expr: expr;)+) => {
        impl Eval for $type {
            fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
                Ok(match self {
                    Self::One(expr) => expr.eval(scopes, library)?,
                    Self::More(lhs, _, rhs) => match (lhs.eval(scopes, library)?, rhs.eval(scopes, library)?) {
                        $((Value::$lhs_type($lhs), Value::$rhs_type($rhs)) => $expr,)+
                        _ => bail!("Type mismatch!"),
                    },
                })
            }
        }
    };

    ($type: ident $lhs: ident $rhs: ident $($path: path => $expr: expr;)+) => {
        impl Eval for $type {
            fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
                Ok(match self {
                    Self::One(expr) => expr.eval(scopes, library)?,
                    Self::More(lhs, op, rhs) => {
                        let $lhs = lhs.eval(scopes, library)?;
                        let $rhs = rhs.eval(scopes, library)?;
                        match op {
                            $($path(_) => $expr,)+
                        }
                    }
                })
            }
        }
    }
}

eval_expression! {
    Expression
    Bool(lhs) Bool(rhs) => Value::Bool(lhs || rhs);
}

eval_expression! {
    AndExp
    Bool(lhs) Bool(rhs) => Value::Bool(lhs && rhs);
}

eval_expression! {
    EqExp lhs rhs
    EqOps::Eq => Value::Bool(lhs == rhs);
    EqOps::Ne => Value::Bool(lhs != rhs);
}

eval_expression! {
    RelExp lhs rhs
    RelOps::Lt => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs < rhs),
        _ => bail!("Type mismatch!"),
    };
    RelOps::Gt => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs > rhs),
        _ => bail!("Type mismatch!"),
    };
    RelOps::Le => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs <= rhs),
        _ => bail!("Type mismatch!"),
    };
    RelOps::Ge => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs >= rhs),
        _ => bail!("Type mismatch!"),
    };
}

eval_expression! {
    AddExp lhs rhs
    AddOps::Add => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
        (Value::String(lhs), rhs) => Value::String(lhs + &rhs.to_string()),
        (lhs, Value::String(rhs)) => Value::String(lhs.to_string() + &rhs),
        _ => bail!("Type mismatch!"),
    };
    AddOps::Sub => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs - rhs),
        _ => bail!("Type mismatch!"),
    };
}

eval_expression! {
    MulExp lhs rhs
    MulOps::Mul => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs * rhs),
        (Value::String(lhs), Value::Int(rhs)) => Value::String(lhs.repeat(rhs as _)),
        _ => bail!("Type mismatch!"),
    };
    MulOps::Div => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs * rhs),
        _ => bail!("Type mismatch!"),
    };
    MulOps::Mod => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs % rhs),
        _ => bail!("Type mismatch!"),
    };
}

impl Eval for UnaryExp {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        match self {
            UnaryExp::Unary(op, expr) => {
                let value = expr.eval(scopes, library)?;
                Ok(match op {
                    UnaryOps::Pos(_) => value,
                    UnaryOps::Neg(_) => match value {
                        Value::Int(value) => Value::Int(-value),
                        _ => bail!("Type mismatch!"),
                    },
                    UnaryOps::Not(_) => match value {
                        Value::Bool(value) => Value::Bool(!value),
                        _ => bail!("Type mismatch!"),
                    },
                })
            }
            UnaryExp::Primary(expr) => expr.eval(scopes, library),
        }
    }
}

impl Eval for PrimaryExp {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        Ok(match self {
            PrimaryExp::ParenExp(expr) => expr.eval(scopes, library)?,
            PrimaryExp::FuncCall(expr) => expr.eval(scopes, library)?,
            PrimaryExp::Access(access) => access.eval(scopes, library)?,
            PrimaryExp::LInt(value) => Value::Int(value.inner() as i32),
            PrimaryExp::LString(value) => Value::String(value.inner().to_owned()),
            PrimaryExp::Table(table) => table.eval(scopes, library)?,
        })
    }
}

impl Eval for ParenExp {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        self.exp.eval(scopes, library)
    }
}

impl Eval for Table {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let mut table = BTreeMap::new();
        if let Some(seq) = &self.values {
            for entry in &seq.0 {
                match entry {
                    TableEntry::Property(index, _, value, _) => {
                        table.insert(
                            Value::String(index.ident().to_owned()),
                            value.eval(scopes, library)?,
                        );
                    }
                    TableEntry::Indexed(index, _, value, _) => {
                        table.insert(index.eval(scopes, library)?, value.eval(scopes, library)?);
                    }
                }
            }
        }
        Ok(Value::Table(table))
    }
}

impl Eval for Access {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let index = self
            .index
            .as_ref()
            .map(|index| index.index.eval(scopes, library))
            .transpose()?;

        let value = scopes.get(self.ident.ident())?;
        Ok(if let Some(index) = index {
            match (value, index) {
                (Value::String(value), Value::Int(index)) => Value::String(
                    value
                        .get(index as usize..=index as usize)
                        .context(format!("Index outside of the range! Index: '{}'", index))?
                        .to_owned(),
                ),
                (Value::Table(value), index) => value
                    .get(&index)
                    .context(format!("Index not found! Index: {:?}", index))?
                    .clone(),
                (target, index) => {
                    bail!("You can't index {:?}[{:?}], type mismatch!", target, index)
                }
            }
        } else {
            value.clone()
        })
    }
}

impl Eval for Assign {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let value = self.rval.eval(scopes, library)?;
        self.lval.assign(scopes, library, value)?;
        Ok(Value::Unit)
    }
}

impl Eval for FunctionCall {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let args = self.args.0.iter();
        let args = args
            .map(|e| e.eval(scopes, library))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if let Some(function) = library.functions.get_mut(self.name.ident()) {
            function(scopes, args)
        } else {
            let function = scopes.get_function(self.name.ident())?.clone();
            ensure!(
                args.len() == function.args.len(),
                "Function argument count mismatch when calling function {}!",
                self.name.ident()
            );
            scopes.local.push(StackFrame::default());
            for (index, arg) in args.into_iter().enumerate() {
                let arg_def = &function.args[index];
                match (&arg, arg_def.arg_type.inner()) {
                    (Value::Int(_), Type::Int) => (),
                    (Value::Bool(_), Type::Bool) => (),
                    (Value::String(_), Type::String) => (),
                    (Value::Table(_), Type::Table) => (),
                    _ => bail!(
                        "Function argument type mismatch when calling function {}!",
                        self.name.ident()
                    ),
                }
                scopes
                    .local
                    .last_mut()
                    .context("Internal error: nowhere to create an argument variable!")?
                    .variables
                    .insert(arg_def.name.ident().to_owned(), arg);
            }
            function.statement.eval(scopes, library)?;
            scopes.local.pop();
            Ok(Value::Unit)
        }
    }
}

trait AssignTo {
    fn assign(&self, scopes: &mut Scopes, library: &mut Library, value: Value) -> Result<()>;
}

macro_rules! impl_assign {
    ($($name: ident $error: literal;)+) => {
        $(impl AssignTo for $name {
            fn assign(&self, scopes: &mut Scopes, library: &mut Library, value: Value) -> Result<()> {
                match self {
                    laps::ast::NonEmptySepList::One(expr) => expr.assign(scopes, library, value),
                    _ => bail!($error),
                }
            }
        })+
    }
}

impl_assign! {
    Expression "Can't assign to '||'!";
    AndExp "Can't assign to '&&'!";
    EqExp "Can't assign to comparisons!";
    RelExp "Can't assign to comparisons!";
    AddExp "Can't assign to arithmetics!";
    MulExp "Can't assign to arithmetics!";
}

impl AssignTo for UnaryExp {
    fn assign(&self, scopes: &mut Scopes, library: &mut Library, value: Value) -> Result<()> {
        match self {
            UnaryExp::Primary(expr) => expr.assign(scopes, library, value),
            _ => bail!("Can't assign to unary operations"),
        }
    }
}

impl AssignTo for PrimaryExp {
    fn assign(&self, scopes: &mut Scopes, library: &mut Library, value: Value) -> Result<()> {
        match self {
            PrimaryExp::Access(expr) => expr.assign(scopes, library, value),
            _ => bail!("Can't assign to this!"),
        }
    }
}

impl AssignTo for Access {
    fn assign(&self, scopes: &mut Scopes, library: &mut Library, value: Value) -> Result<()> {
        let index = self
            .index
            .as_ref()
            .map(|index| index.index.eval(scopes, library))
            .transpose()?;

        let target = scopes.get_mut(self.ident.ident())?;
        if let Some(index) = index {
            match (target, index, value) {
                (Value::String(target), Value::Int(index), Value::String(value)) => {
                    if index < 0 || index as usize >= target.len() {
                        bail!("Index outside of the range! Index: '{}'", index);
                    }
                    target.replace_range(index as usize..=index as usize, &value);
                }
                (Value::Table(target), index, value) => {
                    target.insert(index, value);
                }
                (target, index, value) => bail!(
                    "You can't assign to index {:?}[{:?}] = {:?}, type mismatch!",
                    target,
                    index,
                    value
                ),
            }
            return Ok(());
        }

        match (&target, &value) {
            (Value::Int(_), Value::Int(_)) => (),
            (Value::Bool(_), Value::Bool(_)) => (),
            (Value::String(_), Value::String(_)) => (),
            (Value::Table(_), Value::Table(_)) => (),
            _ => bail!("Type mismatch in assignment!"),
        }
        *target = value;
        Ok(())
    }
}

impl Program {
    pub fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<()> {
        scopes.local.push(StackFrame::default());
        for statement in &self.statements {
            statement.eval(scopes, library)?;
        }
        scopes.local.pop();
        Ok(())
    }
}
