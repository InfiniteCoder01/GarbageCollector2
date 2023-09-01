use crate::ensure_type;

use super::parser::*;
use anyhow::{anyhow, Context, Error};
use laps::ast::NonEmptySepList;
use std::collections::{BTreeMap, HashMap};

pub type Result<T> = std::result::Result<T, Exception>;

pub enum Exception {
    Error(Error),
    Effect(Effect),
    Return(Value),
    Resume(Value),
    EffectUnwind(String, String, Value),
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub effect: String,
    pub handler: String,
    args: Vec<Value>,
}

impl Effect {
    pub fn error(message: String) -> Self {
        Self {
            effect: String::from("exception"),
            handler: String::from("error"),
            args: vec![Value::String(message)],
        }
    }
}

impl From<anyhow::Error> for Exception {
    fn from(error: anyhow::Error) -> Self {
        Self::Error(error)
    }
}

#[macro_export]
macro_rules! ensure {
    ($cond: expr $(,)?) => {
        ensure!($expr, concat!("Ensure failed! Condition: ", stringify!($cond)));
    };
    ($cond: expr, $fmt:expr$(, $($arg:tt)*)?) => {
        if !($cond) {
            bail!($fmt$(, $($arg)*)?);
        }
    };
}

#[macro_export]
macro_rules! bail {
    ($fmt:expr$(, $($arg:tt)*)?) => {
        return Err($crate::gclang::Exception::Error(anyhow!($fmt$(, $($arg)*)?)))
    };
    (late effect $fmt:expr$(, $($arg:tt)*)?) => {
        return Err($crate::gclang::Exception::Effect($crate::gclang::Effect::error(format!($fmt$(, $($arg)*)?))));
    };
    (effect $scopes: ident, $library: ident, $fmt:expr$(, $($arg:tt)*)?) => {
        on_effect(
            Effect::error(format!($fmt$(, $($arg)*)?)),
            $scopes,
            $library,
        )?
    };
    (unresumable $scopes: ident, $library: ident, $fmt:expr$(, $($arg:tt)*)?) => {
        on_effect(
            Effect::Error(format!($fmt$(, $($arg)*)?)),
            $scopes,
            $library,
        )?;
        bail!("Unresumable");
    };
}

#[allow(non_snake_case)]
pub fn Ok<T>(value: T) -> Result<T> {
    Result::Ok(value)
}

pub fn on_effect(effect: Effect, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
    if let Some((index, handler)) = scopes.get_handler(&effect.effect, &effect.handler)? {
        let mut unwind = scopes.local.split_off(index);
        let result = handler.eval(
            scopes,
            library,
            effect.args,
            Some(&format!(
                "handler \"{}\" for effect \"{}\"",
                effect.handler, effect.effect
            )),
        );
        scopes.local.append(&mut unwind);
        match result {
            Err(Exception::Resume(value)) => Ok(value),
            Err(error) => Err(error),
            Result::Ok(result) => Err(Exception::EffectUnwind(
                effect.effect,
                effect.handler,
                result,
            )),
        }
    } else {
        Err(Exception::Effect(effect))
    }
}

pub use bail;
pub use ensure;

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
    Any(Box<Value>),
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
            Value::Any(value) => value.to_string(),
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
            (Value::Any(_), _) => true,
            (_, Value::Any(_)) => true,
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
    effects: HashMap<String, EffectDecl>,
    effect_handlers: HashMap<String, HashMap<String, Function>>,
    included_effects: HashMap<String, Effect>,
}

#[derive(Clone, Debug)]
pub struct Function {
    args: Vec<ArgDef>,
    effects: Vec<EffectTag>,
    expression: Expression,
}

impl Function {
    fn eval(
        &self,
        scopes: &mut Scopes,
        library: &mut Library,
        args: Vec<Value>,
        name_hint: Option<&str>,
    ) -> Result<Value> {
        if let Some(name) = name_hint {
            ensure!(
                args.len() == self.args.len(),
                "Function argument count mismatch in function {}!",
                name
            );
        } else {
            ensure!(
                args.len() == self.args.len(),
                "Function argument count mismatch!"
            );
        }
        scopes.local.push(StackFrame::default());
        for (index, arg) in args.into_iter().enumerate() {
            let arg_def = &self.args[index];
            match (&arg, arg_def.arg_type.inner()) {
                (Value::Int(_), Type::Int) => (),
                (Value::Bool(_), Type::Bool) => (),
                (Value::String(_), Type::String) => (),
                (Value::Table(_), Type::Table) => (),
                (_, Type::Any) => (),
                _ => {
                    scopes.local.pop();
                    bail!(
                        "Function argument type mismatch in argument '{}'!",
                        arg_def.name.ident(),
                    )
                }
            }
            scopes
                .local
                .last_mut()
                .context("Internal error: nowhere to create an argument variable!")?
                .variables
                .insert(arg_def.name.ident().to_owned(), arg);
        }
        for effect in &self.effects {
            let effect = match scopes.get_effect(effect.ident()) {
                Result::Ok(effect) => effect,
                Err(err) => {
                    scopes.local.pop();
                    return Err(err);
                }
            };
            if let Some(handlers) = &effect.handlers {
                let mut included_effects = HashMap::new();
                for handler in &handlers.0 {
                    included_effects.insert(
                        handler.name.ident().to_owned(),
                        Effect {
                            effect: effect.name.ident().to_owned(),
                            handler: handler.name.ident().to_owned(),
                            args: Vec::new(),
                        },
                    );
                }

                scopes
                    .local
                    .last_mut()
                    .context("Internal error: nowhere to create an effect!")?
                    .included_effects = included_effects;
            }
        }
        let result = self.expression.eval(scopes, library);
        scopes.local.pop();
        if let Err(Exception::Return(result)) = result {
            return Ok(result);
        }
        result
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
        format!("{:?}", self.expression).cmp(&format!("{:?}", other.expression))
    }
}

impl Scopes {
    fn get(&self, name: &str) -> Result<&Value> {
        let value = match self
            .local
            .iter()
            .rev()
            .find_map(|frame| frame.variables.get(name))
        {
            Some(value) => value,
            None => match self.global.get(name) {
                Some(value) => value,
                None => bail!("Variable `{name}` not found!"),
            },
        };
        Ok(if let Value::Any(value) = value {
            value.as_ref()
        } else {
            value
        })
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

    fn get_function(&self, name: &str) -> Option<&Function> {
        match self
            .local
            .iter()
            .rev()
            .find_map(|frame| frame.functions.get(name))
        {
            Some(function) => Some(function),
            None => match self.get(name) {
                Result::Ok(Value::Function(function)) => Some(function),
                _ => None,
            },
        }
    }

    fn get_included_effect(&self, name: &str) -> Option<&Effect> {
        self.local
            .iter()
            .rev()
            .find_map(|frame| frame.included_effects.get(name))
    }

    fn get_effect(&self, name: &str) -> Result<&EffectDecl> {
        match self
            .local
            .iter()
            .rev()
            .find_map(|frame| frame.effects.get(name))
        {
            Some(effect) => Ok(effect),
            None => bail!("Effect `{name}` not found!"),
        }
    }

    fn get_handler(&self, effect: &str, handler: &str) -> Result<Option<(usize, Function)>> {
        Ok(
            match self
                .local
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, frame)| {
                    frame
                        .effect_handlers
                        .get(effect)
                        .map(|handlers| (index, handlers))
                }) {
                Some((index, handlers)) => Some((
                    index,
                    handlers.get(handler).cloned().context(format!(
                        "Internal error: No effect handler '{}' for effect '{}'!",
                        handler, effect
                    ))?,
                )),
                None => None,
            },
        )
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
                    .context("Internal error: Nowhere to create a local variable!")?
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
            Statement::EffectDecl(decl) => decl.eval(scopes, library),
            Statement::If(statement) => statement.eval(scopes, library),
            Statement::Return(_, expr, _) => Err(Exception::Return(expr.eval(scopes, library)?)),
            Statement::Resume(_, expr, _) => Err(Exception::Resume(expr.eval(scopes, library)?)),
            Statement::Expression(expr) => expr.eval(scopes, library),
            Statement::End(_) => Ok(Value::Never),
        }
    }
}

impl Eval for FnDecl {
    fn eval(&self, scopes: &mut Scopes, _library: &mut Library) -> Result<Value> {
        let block = self.block.function();
        match scopes
            .local
            .last_mut()
            .context("Internal error: Nowhere to create a local function!")?
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

impl Eval for EffectDecl {
    fn eval(&self, scopes: &mut Scopes, _library: &mut Library) -> Result<Value> {
        match scopes
            .local
            .last_mut()
            .context("Internal error: Nowhere to create a local effect!")?
            .effects
            .entry(self.name.ident().to_owned())
        {
            std::collections::hash_map::Entry::Occupied(_) => {
                bail!("Effect '{}' already exists!", self.name.ident())
            }
            std::collections::hash_map::Entry::Vacant(effect) => {
                effect.insert(self.clone());
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

impl Eval for ExpressionStatement {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        match self {
            ExpressionStatement::Expression(expr, _) => expr.eval(scopes, library),
            ExpressionStatement::Assign(expr) => expr.eval(scopes, library),
        }
    }
}

fn foreach_list<T, S>(
    list: &NonEmptySepList<T, S>,
    last_separator: Option<&S>,
    mut f: impl FnMut(Option<&S>, &T) -> Result<()>,
) -> Result<()> {
    match list {
        NonEmptySepList::One(value) => f(last_separator, value)?,
        NonEmptySepList::More(lhs, separator, rhs) => {
            f(last_separator, lhs)?;
            foreach_list(rhs.as_ref(), Some(separator), f)?;
        }
    }
    Ok(())
}

macro_rules! eval_expression {
    ($type: ident $($lhs_type: ident ($lhs: ident) $rhs_type: ident ($rhs: ident) => $expr: expr;)+) => {
        impl Eval for $type {
            fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
                let mut lhs = None;
                foreach_list(self, None, |_, rhs| {
                    if let Some(value) = lhs.take() {
                        lhs = Some(match (value, rhs.eval(scopes, library)?) {
                            $((Value::$lhs_type($lhs), Value::$rhs_type($rhs)) => $expr,)+
                            _ => bail!("Type mismatch!"),
                        });
                    } else {
                        lhs = Some(rhs.eval(scopes, library)?);
                    }
                    Ok(())
                })?;
                Ok(lhs.unwrap())
            }
        }
    };

    ($type: ident $lhs: ident $rhs: ident $($path: path => $expr: expr;)+) => {
        impl Eval for $type {
            fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
                let mut lhs: Option<Value> = None;
                foreach_list(self, None, |op, rhs| {
                    if let Some($lhs) = lhs.take() {
                        let $rhs = rhs.eval(scopes, library)?;
                        lhs = Some(match op.unwrap() {
                            $($path(_) => $expr,)+
                        });
                    } else {
                        lhs = Some(rhs.eval(scopes, library)?);
                    }
                    Ok(())
                })?;
                Ok(lhs.unwrap())
            }
        }
    }
}

eval_expression! {
    Expression
    Bool(lhs) Bool(rhs) => Value::Bool(lhs || rhs);
}

eval_expression! {
    AndExpression
    Bool(lhs) Bool(rhs) => Value::Bool(lhs && rhs);
}

eval_expression! {
    EqExpression lhs rhs
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
    RelExpression lhs rhs
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
    AddExpression lhs rhs
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
    MulExpression lhs rhs
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
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs / rhs),
        _ => bail!("Type mismatch!"),
    };
    MulOps::Mod => match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Value::Int(lhs % rhs),
        _ => bail!("Type mismatch!"),
    };
}

impl Eval for UnaryExpression {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        match self {
            UnaryExpression::Unary(op, expr) => {
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
            UnaryExpression::Primary(expr) => expr.eval(scopes, library),
        }
    }
}

impl Eval for PrimaryExpression {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        Ok(match self {
            PrimaryExpression::Parens(expr) => expr.eval(scopes, library)?,
            PrimaryExpression::Block(expr) => expr.eval(scopes, library)?,
            PrimaryExpression::FuncCall(expr) => expr.eval(scopes, library)?,
            PrimaryExpression::Access(access) => access.eval(scopes, library)?,
            PrimaryExpression::LInt(value) => Value::Int(value.inner() as i32),
            PrimaryExpression::LBoolTrue(_) => Value::Bool(true),
            PrimaryExpression::LBoolFalse(_) => Value::Bool(false),
            PrimaryExpression::LUnit(_) => Value::Unit,
            PrimaryExpression::LString(value) => Value::String(value.inner().to_owned()),
            PrimaryExpression::Array(array) => array.eval(scopes, library)?,
            PrimaryExpression::Table(_, table) => table.eval(scopes, library)?,
            PrimaryExpression::Lambda(_, function) => function.eval(scopes, library)?,
            PrimaryExpression::Any(_) => Value::Any(Box::new(Value::Unit)),
        })
    }
}

impl Eval for ParenExpression {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        self.exp.eval(scopes, library)
    }
}

impl Eval for BlockExpression {
    fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<Value> {
        scopes.local.push(StackFrame::default());

        if let Some(with_handlers) = &self.with_handlers {
            let mut add_handlers = || -> Result<()> {
                for with_block in &with_handlers.0 {
                    let effect_decl = scopes.get_effect(with_block.effect.ident())?;
                    if let Some(handlers_decl) = &effect_decl.handlers {
                        let effect_name = effect_decl.name.ident().to_owned();
                        ensure!(
                            with_block.handlers.0.len() == handlers_decl.0.len(),
                            "Handler count mismatch for effect '{}'",
                            with_block.effect.ident(),
                        );
                        let mut effect_handlers = HashMap::new();
                        for handler_decl in &handlers_decl.0 {
                            let handler_impl = &with_block
                                .handlers
                                .0
                                .iter()
                                .find(|handler_impl| {
                                    handler_impl.1.ident() == handler_decl.name.ident()
                                })
                                .context(format!(
                                    "Handler '{}' is not implemented for effect '{}'!",
                                    handler_decl.name.ident(),
                                    effect_decl.name.ident()
                                ))?
                                .2;
                            // * Check signature
                            if !handler_decl.signature.args.0.iter().enumerate().all(
                                |(index, arg)| {
                                    if let Some(impl_arg) = handler_impl.signature.args.0.get(index)
                                    {
                                        impl_arg.arg_type.inner() == arg.arg_type.inner()
                                    } else {
                                        false
                                    }
                                },
                            ) {
                                bail!(format!(
                                    "Handler '{}' have incorrect signature for effect '{}'!",
                                    handler_decl.name.ident(),
                                    effect_decl.name.ident()
                                ));
                            }
                            effect_handlers.insert(
                                handler_decl.name.ident().to_owned(),
                                handler_impl.function(),
                            );
                        }
                        scopes
                            .local
                            .last_mut()
                            .context("Internal error: Nowhere to add an effect handler!")?
                            .effect_handlers
                            .insert(effect_name, effect_handlers);
                    } else {
                        ensure!(
                            with_block.handlers.0.is_empty(),
                            "Effect '{}' doesn't have any handlers!",
                            with_block.effect.ident(),
                        );
                    }
                }
                Ok(())
            };
            let result = add_handlers();
            if let Err(err) = result {
                scopes.local.pop();
                return Err(err);
            }
        }

        let mut eval = || -> Result<()> {
            if let Some(statements) = &self.statements {
                for statement in &statements.0 {
                    statement.eval(scopes, library)?;
                }
            }
            Ok(())
        };
        let result = eval();
        scopes.local.pop();
        if let Some(with_handlers) = &self.with_handlers {
            if let Err(Exception::EffectUnwind(effect, handler, value)) = result {
                if with_handlers
                    .0
                    .iter()
                    .find(|with_block| with_block.effect.ident() == effect)
                    .is_some_and(|with_block| {
                        with_block
                            .handlers
                            .0
                            .iter()
                            .any(|effect_handler| effect_handler.1.ident() == handler)
                    })
                {
                    return Ok(value);
                } else {
                    return Err(Exception::EffectUnwind(effect, handler, value));
                }
            } else {
                result?;
            }
        } else {
            result?;
        }
        Ok(Value::Unit)
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

impl FnBlock {
    fn function(&self) -> Function {
        Function {
            args: self.signature.args.0.clone(),
            effects: self.effects.0.clone(),
            expression: self.expression.clone(),
        }
    }
}

impl Eval for FnBlock {
    fn eval(&self, _scopes: &mut Scopes, _library: &mut Library) -> Result<Value> {
        Ok(Value::Function(self.function()))
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
                    if let Some(value) = value.get(index as usize..=index as usize) {
                        value.to_owned()
                    } else {
                        bail!(
                            effect
                            scopes,
                            library,
                            "String index out of bounds! Index: '{}'",
                            index,
                        );
                        bail!("Unresumable");
                    },
                ),
                (Value::Array(value), Value::Int(index)) => {
                    if let Some(value) = value.get(index as usize) {
                        value.clone()
                    } else {
                        bail!(
                            effect
                            scopes,
                            library,
                            "Array index out of bounds! Index: {:?}",
                            index
                        );
                        bail!("Unresumable");
                    }
                }
                (Value::Table(value), index) => {
                    if let Some(value) = value.get(&index) {
                        value.clone()
                    } else {
                        bail!(
                            effect
                            scopes,
                            library,
                            "Index not found in table! Index: {:?}",
                            index
                        );
                        bail!("Unresumable");
                    }
                }
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
            .map(|e| {
                Ok(match e.eval(scopes, library)? {
                    Value::Any(value) => value.as_ref().clone(),
                    value => value,
                })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        if let Some(function) = scopes.get_function(self.name.ident()).cloned() {
            function.eval(scopes, library, args, Some(self.name.ident()))
        } else if let Some(mut effect) = scopes.get_included_effect(self.name.ident()).cloned() {
            effect.args = args;
            on_effect(effect, scopes, library)
        } else {
            let perform = || {
                if self.name.ident() == "eval" {
                    match &args[..] {
                        [Value::String(code)] => {
                            Program::parse(code)?.eval(scopes, library)?;
                            Ok(Value::Unit)
                        }
                        _ => bail!(r#"Usage: eval("some_global_variable = \"Evaluated\";");"#),
                    }
                } else if self.name.ident() == "import" {
                    match &args[..] {
                        [Value::String(code)] => {
                            Program::parse(code)?.import(scopes, library)?;
                            Ok(Value::Unit)
                        }
                        _ => bail!(r#"Usage: import("fn something() {{}}");"#),
                    }
                } else if self.name.ident() == "for" {
                    match &args[..] {
                        [Value::String(string), Value::Function(body)] => {
                            for character in string.chars() {
                                body.eval(
                                    scopes,
                                    library,
                                    vec![Value::String(String::from(character))],
                                    Some("for loop body"),
                                )?;
                            }
                            Ok(Value::Unit)
                        }
                        [Value::Array(array), Value::Function(body)] => {
                            for element in array {
                                body.eval(
                                    scopes,
                                    library,
                                    vec![element.clone()],
                                    Some("for loop body"),
                                )?;
                            }
                            Ok(Value::Unit)
                        }
                        [Value::Table(table), Value::Function(body)] => {
                            for (key, value) in table {
                                body.eval(
                                    scopes,
                                    library,
                                    vec![key.clone(), value.clone()],
                                    Some("for loop body"),
                                )?;
                            }
                            Ok(Value::Unit)
                        }
                        [Value::Int(initial), Value::Int(limit), Value::Function(body)] => {
                            for index in *initial..*limit {
                                body.eval(
                                    scopes,
                                    library,
                                    vec![Value::Int(index)],
                                    Some("for loop body"),
                                )?;
                            }
                            Ok(Value::Unit)
                        }
                        _ => bail!(concat!(
                            r#"Usage: for([1, 2, 3], fn (number: int) {{ println(number); }});\n"#,
                            r#"or for(0, 2, fn (index: int) {{ println(index); }});\n"#,
                            r#"or for({ a = 1; b = 2; }, fn (key: String, value: int) {{ println(key, value); }});"#,
                        )),
                    }
                } else if let Some(function) = library.functions.get_mut(self.name.ident()) {
                    let result = function(scopes, args);
                    if let Err(Exception::Effect(effect)) = result {
                        on_effect(effect, scopes, library)
                    } else {
                        result
                    }
                } else {
                    bail!("Function `{}` not found!", self.name.ident());
                }
            };
            let result = perform();
            if let Err(Exception::Effect(effect)) = result {
                on_effect(effect, scopes, library)
            } else {
                result
            }
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
    AndExpression "Can't assign to '&&'!";
    EqExpression "Can't assign to comparisons!";
    RelExpression "Can't assign to comparisons!";
    AddExpression "Can't assign to arithmetics!";
    MulExpression "Can't assign to arithmetics!";
}

impl AssignTo for UnaryExpression {
    fn assign(&self, scopes: &mut Scopes, library: &mut Library, value: Value) -> Result<()> {
        match self {
            UnaryExpression::Primary(expr) => expr.assign(scopes, library, value),
            _ => bail!("Can't assign to unary operations"),
        }
    }
}

impl AssignTo for PrimaryExpression {
    fn assign(&self, scopes: &mut Scopes, library: &mut Library, value: Value) -> Result<()> {
        match self {
            PrimaryExpression::Access(expr) => expr.assign(scopes, library, value),
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
                        bail!(
                            effect
                            scopes,
                            library,
                            "String index out of bounds! Index: {}",
                            index
                        );
                        bail!("Unresumable");
                    }
                    target.replace_range(index as usize..=index as usize, &value);
                }
                (Value::Table(target), index, value) => {
                    target.insert(index, value);
                }
                (Value::Array(target), Value::Int(index), value) => {
                    if index < 0 || index as usize >= target.len() {
                        bail!(
                            effect
                            scopes,
                            library,
                            "Array index out of bounds! Index: {}",
                            index
                        );
                        bail!("Unresumable");
                    }
                    let target = target.get_mut(index as usize).unwrap();
                    if let Value::Any(target) = target {
                        *target.as_mut() = value;
                    } else {
                        if !target.matches(&value) {
                            bail!("Type mismatch in assignment!");
                        }
                        *target = value;
                    }
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

        if let Value::Any(target) = target {
            *target.as_mut() = value;
        } else {
            if !target.matches(&value) {
                bail!("Type mismatch in assignment!");
            }
            *target = value;
        }
        Ok(())
    }
}

impl Program {
    pub fn import(&self, scopes: &mut Scopes, library: &mut Library) -> Result<()> {
        for statement in &self.statements {
            statement.eval(scopes, library)?;
        }
        Ok(())
    }

    pub fn eval(&self, scopes: &mut Scopes, library: &mut Library) -> Result<()> {
        scopes.local.push(StackFrame::default());
        let result = self.import(scopes, library);
        scopes.local.pop();
        result?;
        Ok(())
    }
}
