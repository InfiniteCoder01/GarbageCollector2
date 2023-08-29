use crate::ensure_type;

use super::parser::*;
use anyhow::*;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Int(i32),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Table(BTreeMap<Value, Value>),
    Function(Function),
    Unit,
    Never,
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            Value::String(value) => value.clone(),
            Value::Array(value) => format!(
                "[{}]",
                value
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Table(value) => format!(
                "{{\n{}}}",
                value
                    .iter()
                    .map(|(key, value)| format!("{} = {};\n", key.to_string(), value.to_string()))
                    .collect::<Vec<_>>()
                    .join("")
            ),
            Value::Function(function) => format!("{:#?}", function),
            Value::Unit => "Unit".to_owned(),
            Value::Never => "Never".to_owned(),
        }
    }
}

impl Value {
    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(_), Value::Int(_)) => true,
            (Value::Bool(_), Value::Bool(_)) => true,
            (Value::String(_), Value::String(_)) => true,
            (Value::Array(lhs), Value::Array(rhs)) => {
                if !lhs.is_empty() && !rhs.is_empty() {
                    lhs[0].matches(&rhs[0])
                } else {
                    true
                }
            }
            (Value::Table(lhs), Value::Table(rhs)) => {
                if !lhs.is_empty() && !rhs.is_empty() {
                    let lhs = lhs.iter().next().unwrap();
                    let rhs = rhs.iter().next().unwrap();
                    lhs.0.matches(rhs.0) && lhs.1.matches(rhs.1)
                } else {
                    true
                }
            }
            (Value::Function(_), Value::Function(_)) => true,
            (Value::Unit, Value::Unit) => true,
            (Value::Never, Value::Never) => true,
            _ => false,
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

#[derive(Clone, Debug)]
pub struct Function {
    args: Vec<ArgDef>,
    statement: Statement,
}
impl Function {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library, args: Vec<Value>) -> Result<Value> {
        ensure!(
            args.len() == self.args.len(),
            "Function argument count mismatch!",
        );
        scopes.local.push(StackFrame::default());
        for (index, arg) in args.into_iter().enumerate() {
            let arg_def = &self.args[index];
            match (&arg, arg_def.arg_type.inner()) {
                (Value::Int(_), Type::Int) => (),
                (Value::Bool(_), Type::Bool) => (),
                (Value::String(_), Type::String) => (),
                (Value::Table(_), Type::Table) => (),
                _ => bail!(
                    "Function argument type mismatch in argument '{}'!",
                    arg_def.name.ident(),
                ),
            }
            scopes
                .local
                .last_mut()
                .context("Internal error: nowhere to create an argument variable!")?
                .variables
                .insert(arg_def.name.ident().to_owned(), arg);
        }
        self.statement.eval(scopes, library)?;
        scopes.local.pop();
        Ok(Value::Unit)
    }
}

impl PartialEq for Function {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Eq for Function {}
impl PartialOrd for Function {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Function {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        format!("{:?}", self.statement).cmp(&format!("{:?}", other.statement))
    }
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
            None => match self.get(name) {
                Result::Ok(Value::Function(function)) => Ok(function),
                Result::Ok(_) => {
                    bail!("Function `{name}` not found! Note: variable with this name exists.")
                }
                Err(_) => bail!("Function `{name}` not found!"),
            },
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
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let block = match self.block.eval(scopes, library)? {
            Value::Function(function) => function,
            _ => unreachable!(),
        };
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
                function.insert(block);
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
    EqOps::Eq => {
        if !lhs.matches(&rhs) {
            bail!("Type mismatch in equality!");
        }
        Value::Bool(lhs == rhs)
    };
    EqOps::Ne => {
        if !lhs.matches(&rhs) {
            bail!("Type mismatch in non-equality!");
        }
        Value::Bool(lhs != rhs)
    };
}

eval_expression! {
    RelExp lhs rhs
    RelOps::Lt => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs < rhs),
        _ => bail!("Type mismatch in operator '<'!"),
    };
    RelOps::Gt => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs > rhs),
        _ => bail!("Type mismatch in operator '>'!"),
    };
    RelOps::Le => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs <= rhs),
        _ => bail!("Type mismatch in operator '<='!"),
    };
    RelOps::Ge => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Bool(lhs >= rhs),
        _ => bail!("Type mismatch in operator '>='!"),
    };
}

eval_expression! {
    AddExp lhs rhs
    AddOps::Add =>{
        let matches = lhs.matches(&rhs);
        match (lhs, rhs) {
            (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs + rhs),
            (Value::String(lhs), rhs) => Value::String(lhs + &rhs.to_string()),
            (lhs, Value::String(rhs)) => Value::String(lhs.to_string() + &rhs),
            (Value::Table(mut lhs), Value::Table(mut rhs)) => {
                if !matches {
                    bail!("Type mismatch when joining tables!");
                }
                lhs.append(&mut rhs);
                Value::Table(lhs)
            },
            (Value::Array(mut lhs), Value::Array(mut rhs)) => {
                if !matches {
                    bail!("Type mismatch when joining arrays!");
                }
                lhs.append(&mut rhs);
                Value::Array(lhs)
            },
            _ => bail!("Type mismatch in operator '+'!"),
        }
    };
    AddOps::Sub => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs - rhs),
        _ => bail!("Type mismatch in operator '-'!"),
    };
}

eval_expression! {
    MulExp lhs rhs
    MulOps::Mul => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs * rhs),
        (Value::String(lhs), Value::Int(rhs)) => Value::String(lhs.repeat(rhs as _)),
        (Value::Array(lhs), Value::Int(rhs)) => {
            let count = lhs.len() * rhs as usize;
            Value::Array(lhs.into_iter().cycle().take(count).collect())
        }
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
            PrimaryExp::Array(array) => array.eval(scopes, library)?,
            PrimaryExp::Table(table) => table.eval(scopes, library)?,
            PrimaryExp::Lambda(_, function) => function.eval(scopes, library)?,
        })
    }
}

impl Eval for ParenExp {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        self.exp.eval(scopes, library)
    }
}

impl Eval for Array {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let mut array = Vec::<Value>::with_capacity(self.values.0.len());
        for (index, value) in self.values.0.iter().enumerate() {
            let value = value.eval(scopes, library)?;
            if index > 0 && !array[0].matches(&value) {
                bail!("Array has mismatched types of elements!");
            }
            array.push(value);
        }
        Ok(Value::Array(array))
    }
}

impl Eval for Table {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        let mut table = BTreeMap::new();
        if let Some(seq) = &self.values {
            for entry in &seq.0 {
                let (key, value) = match entry {
                    TableEntry::Property(index, _, value, _) => (
                        Value::String(index.ident().to_owned()),
                        value.eval(scopes, library)?,
                    ),
                    TableEntry::Indexed(index, _, value, _) => {
                        (index.eval(scopes, library)?, value.eval(scopes, library)?)
                    }
                };
                if !table.is_empty() {
                    let (key0, value0) = table.iter().next().unwrap();
                    if !key.matches(key0) || !value.matches(value0) {
                        bail!("Table has mismatched types of keys or values!");
                    }
                }
                table.insert(key, value);
            }
        }
        Ok(Value::Table(table))
    }
}

impl Eval for FnBlock {
    fn eval(&self, _scopes: &mut Scopes, _library: &mut Library) -> Result<Value> {
        Ok(Value::Function(Function {
            args: self.args.0.clone(),
            statement: self.statement.clone(),
        }))
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
                        .context(format!("String index out of bounds! Index: '{}'", index))?
                        .to_owned(),
                ),
                (Value::Array(value), Value::Int(index)) => value
                    .get(index as usize)
                    .context(format!("Array index out of bounds! Index: {:?}", index))?
                    .clone(),
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
        if self.name.ident() == "eval" {
            match &args[..] {
                [Value::String(code)] => {
                    Program::parse(code)?.eval(scopes, library)?;
                    Ok(Value::Unit)
                }
                _ => bail!(r#"Usage: eval("some_global_variable = \"Evaluated\";");"#),
            }
        } else if self.name.ident() == "for" {
            match &args[..] {
                [Value::String(string), Value::Function(body)] => {
                    for character in string.chars() {
                        body.eval(
                            scopes,
                            library,
                            vec![Value::String(String::from(character))],
                        )?;
                    }
                    Ok(Value::Unit)
                }
                [Value::Array(array), Value::Function(body)] => {
                    for element in array {
                        body.eval(scopes, library, vec![element.clone()])?;
                    }
                    Ok(Value::Unit)
                }
                [Value::Table(table), Value::Function(body)] => {
                    for (key, value) in table {
                        body.eval(scopes, library, vec![key.clone(), value.clone()])?;
                    }
                    Ok(Value::Unit)
                }
                [Value::Int(initial), Value::Int(limit), Value::Function(body)] => {
                    for index in *initial..*limit {
                        body.eval(scopes, library, vec![Value::Int(index)])?;
                    }
                    Ok(Value::Unit)
                }
                _ => bail!(concat!(
                    r#"Usage: for([1, 2, 3], fn (number: int) {{ println(number); }})\n"#,
                    r#"or for(0, 2, fn (index: int) {{ println(index); }})\n"#,
                    r#"or for({ a = 1; b = 2; }, fn (key: String, value: int) {{ println(key, value); }})"#,
                )),
            }
        } else if let Some(function) = library.functions.get_mut(self.name.ident()) {
            function(scopes, args)
        } else {
            let function = scopes.get_function(self.name.ident())?.clone();
            function.eval(scopes, library, args)
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
            _ => bail!("Can't assign to this primary!"),
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
                        bail!("String index out of bounds! Index: {}", index);
                    }
                    target.replace_range(index as usize..=index as usize, &value);
                }
                (Value::Table(target), index, value) => {
                    target.insert(index, value);
                }
                (Value::Array(target), Value::Int(index), value) => {
                    if index < 0 || index as usize >= target.len() {
                        bail!("Array index out of bounds! Index: {}", index);
                    }
                    let target = target.get_mut(index as usize).unwrap();
                    if !target.matches(&value) {
                        bail!("Type mismatch in assignment!");
                    }
                    *target = value;
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

        if !target.matches(&value) {
            bail!("Type mismatch in assignment!");
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
