use super::*;
use anyhow::{anyhow, bail, Context, Result};
use enum_lexer::enum_lexer;

// * ------------------------------------- Lexer ------------------------------------ * //
enum_lexer! {
    #[derive(Clone, Debug, PartialEq)]
    enum lexer {
        COMMENTS: {
            r"//.*?\n" => !,
            r"/\*.*?\*/" => !,
        }
        Ident(String): {
            r"[A-Za-z_][A-Za-z_0-9]*" => Ident(text),
        }
        LInt(i32): {
            r"-?[0-9]+" => LInt(text.parse()?),
        }
        LString(String): {
            r#"".*?""# => LString(text[1..text.len() - 1].replace("\\n", "\n").replace("\\\"", "\"")),
            r#""\\\"""# => LString(String::from("\"")),
        }
        LBool(bool): {
            r"true" => LBool(true),
            r"false" => LBool(false),

        }
        Op(String): {
            r"\+" => Op(String::from("+")),
            r"\-" => Op(String::from("-")),
            r"\*" => Op(String::from("*")),
            r"\/" => Op(String::from("/")),
            r"%" => Op(String::from("%")),
            r"=" => Op(String::from("=")),
            r"==" => Op(String::from("==")),
        }
        Semicolon: r";",
        Fn: r"fn",
        Let: r"let",
        Global: r"global",
        Parentheses(Vec<Token>) : {
            r"\(" => {
                Parentheses(read_group()?)
            }
            r"\)" => { panic!("error") }
        }
    }
}

// * ------------------------------------ Parser ------------------------------------ * //
use lexer::{Token, TokenInner};
use std::iter::Peekable;

fn next_token(
    tokens: &mut Peekable<impl Iterator<Item = lexer::Result<Token>>>,
) -> Result<Option<TokenInner>> {
    Ok(if let Some(token) = tokens.next() {
        Some(token.map_err(|err| anyhow::anyhow!(err.to_string()))?.inner)
    } else {
        None
    })
}

fn peek_token(
    tokens: &mut Peekable<impl Iterator<Item = lexer::Result<Token>>>,
) -> Result<Option<TokenInner>> {
    Ok(if let Some(token) = tokens.peek() {
        Some(
            token
                .as_ref()
                .map_err(|err| anyhow::anyhow!(err.to_string()))?
                .inner
                .clone(),
        )
    } else {
        None
    })
}

fn to_stream(tokens: Vec<Token>) -> Peekable<impl Iterator<Item = lexer::Result<Token>>> {
    tokens.into_iter().map(Ok).peekable()
}

impl Scope {
    fn parse(
        tokens: &mut Peekable<impl Iterator<Item = lexer::Result<lexer::Token>>>,
    ) -> Result<Self> {
        let mut scope = Self::default();
        while let Some(token) = peek_token(tokens)? {
            scope.statements.push(match token {
                TokenInner::Global => {
                    next_token(tokens)?;
                    if let Some(TokenInner::Ident(name)) = next_token(tokens)? {
                        if next_token(tokens)? == Some(TokenInner::Op(String::from("="))) {
                            Statement::GlobalVariable(name, Expression::parse(tokens)?)
                        } else {
                            bail!("You have to assign a value to a global variable!");
                        }
                    } else {
                        bail!("Expected global variable name!");
                    }
                }
                _ => Statement::Expression(Expression::parse(tokens)?),
            });
            if next_token(tokens)? != Some(TokenInner::Semicolon) {
                bail!("We love semicolons!");
            }
        }
        Ok(scope)
    }
}

impl Expression {
    fn parse(
        tokens: &mut Peekable<impl Iterator<Item = lexer::Result<lexer::Token>>>,
    ) -> Result<Self> {
        let mut lhs = Self::comparsion(tokens)?;
        if peek_token(tokens)? == Some(TokenInner::Op(String::from("="))) {
            next_token(tokens)?;
            if let Expression::Variable(name) = lhs {
                lhs = Expression::Assignment(name, Box::new(Expression::parse(tokens)?));
            } else {
                bail!("You can assign only to a variable!");
            }
        }
        Ok(lhs)
    }

    fn comparsion(
        tokens: &mut Peekable<impl Iterator<Item = lexer::Result<lexer::Token>>>,
    ) -> Result<Self> {
        let mut lhs = Self::binary(tokens)?;
        if let Some(TokenInner::Op(op)) = peek_token(tokens)? {
            match op.as_str() {
                ">" | "<" | ">=" | "<=" | "==" | "!=" => {
                    next_token(tokens)?;
                    lhs = Expression::Binary(Box::new(lhs), op, Box::new(Self::mult(tokens)?))
                }
                _ => (),
            }
        }
        Ok(lhs)
    }

    fn binary(
        tokens: &mut Peekable<impl Iterator<Item = lexer::Result<lexer::Token>>>,
    ) -> Result<Self> {
        let mut lhs = Self::mult(tokens)?;
        while let Some(TokenInner::Op(op)) = peek_token(tokens)? {
            match op.as_str() {
                "+" | "-" => {
                    next_token(tokens)?;
                    lhs = Expression::Binary(Box::new(lhs), op, Box::new(Self::mult(tokens)?))
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn mult(
        tokens: &mut Peekable<impl Iterator<Item = lexer::Result<lexer::Token>>>,
    ) -> Result<Self> {
        let mut lhs = Self::unary(tokens)?;
        while let Some(TokenInner::Op(op)) = peek_token(tokens)? {
            match op.as_str() {
                "*" | "/" | "%" => {
                    next_token(tokens)?;
                    lhs = Expression::Binary(Box::new(lhs), op, Box::new(Self::unary(tokens)?))
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn unary(
        tokens: &mut Peekable<impl Iterator<Item = lexer::Result<lexer::Token>>>,
    ) -> Result<Self> {
        Ok(match next_token(tokens)?.context("Expected expression!")? {
            lexer::TokenInner::Ident(name) => {
                let mut arguments = Vec::new();
                if let Some(TokenInner::Parentheses(args)) = peek_token(tokens)? {
                    {
                        let tokens = &mut to_stream(args);
                        while peek_token(tokens)?.is_some() {
                            arguments.push(Expression::parse(tokens)?);
                        }
                    }
                    next_token(tokens)?;
                    Expression::FunctionCall { name, arguments }
                } else {
                    Expression::Variable(name)
                }
            }

            TokenInner::LInt(value) => Self::Constant(Value::Int(value)),
            TokenInner::LString(value) => Self::Constant(Value::String(value)),
            TokenInner::LBool(value) => Self::Constant(Value::Bool(value)),
            token => bail!("Expected expression, got '{:?}'!", token),
        })
    }
}

impl Program {
    pub fn parse(src: &str) -> Result<Self> {
        let tokens = lexer::parse_str(src).map_err(|err| anyhow!(err.to_string()))?;
        Ok(Self {
            scope: Scope::parse(&mut tokens.peekable())?,
        })
    }
}
