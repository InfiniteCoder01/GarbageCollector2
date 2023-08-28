use laps::ast::{NonEmptySepList, NonEmptySeq, SepSeq};
use laps::lexer::{int_literal, str_literal};
use laps::prelude::*;
use laps::reader::Reader;
use laps::return_error;
use laps::span::Result;
use laps::token::TokenBuffer;
use std::{fmt, str::FromStr};

// * ---------------------------------------------------------------------------------- Lexer --------------------------------------------------------------------------------- * //
#[token_kind]
#[derive(Debug, Tokenize)]
enum TokenKind {
    #[skip(r"\s+")]
    _Skip,
    #[regex(r"global|let")]
    Keyword(Keyword),
    #[regex(r"[_a-zA-Z][_a-zA-Z0-9]*")]
    Ident(String),
    #[regex(r"[0-9]|[1-9][0-9]+|0x[0-9a-fA-F]+", int_literal)]
    Int(u64),
    #[regex(r#""([^"\\]|\\[\s\S])*""#)]
    String(StringLiteral),
    #[regex(r"\+|-|\*|/|%|<|>|<=|>=|==|!=|&&|\|\||!|=")]
    Operator(Operator),
    #[regex(r".")]
    Other(char),
    #[eof]
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Keyword {
    Global,
    Let,
}

impl FromStr for Keyword {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "global" => Ok(Keyword::Global),
            "let" => Ok(Keyword::Let),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Let => write!(f, "let"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    And,
    Or,
    Not,
    Assign,
}

impl FromStr for Operator {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "+" => Ok(Self::Add),
            "-" => Ok(Self::Sub),
            "*" => Ok(Self::Mul),
            "/" => Ok(Self::Div),
            "%" => Ok(Self::Mod),
            "<" => Ok(Self::Lt),
            ">" => Ok(Self::Gt),
            "<=" => Ok(Self::Le),
            ">=" => Ok(Self::Ge),
            "==" => Ok(Self::Eq),
            "!=" => Ok(Self::Ne),
            "&&" => Ok(Self::And),
            "||" => Ok(Self::Or),
            "!" => Ok(Self::Not),
            "=" => Ok(Self::Assign),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Mod => write!(f, "%"),
            Self::Lt => write!(f, "<"),
            Self::Gt => write!(f, ">"),
            Self::Le => write!(f, "<="),
            Self::Ge => write!(f, ">="),
            Self::Eq => write!(f, "=="),
            Self::Ne => write!(f, "!="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::Not => write!(f, "!"),
            Self::Assign => write!(f, "="),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct StringLiteral(String);
impl FromStr for StringLiteral {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(str_literal(s).ok_or(())?))
    }
}

impl fmt::Display for StringLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

type Token = laps::token::Token<TokenKind>;

token_ast! {
  #[derive(Debug, PartialEq)]
  macro Token<TokenKind> {
    [ident] => { kind: TokenKind::Ident(_), prompt: "identifier" },
    [let] => { kind: TokenKind::Keyword(Keyword::Let) },
    [global] => { kind: TokenKind::Keyword(Keyword::Global) },
    [lint] => { kind: TokenKind::Int(_), prompt: "integer literal" },
    [lstring] => { kind: TokenKind::String(_), prompt: "string literal" },
    [+] => { kind: TokenKind::Operator(Operator::Add) },
    [-] => { kind: TokenKind::Operator(Operator::Sub) },
    [*] => { kind: TokenKind::Operator(Operator::Mul) },
    [/] => { kind: TokenKind::Operator(Operator::Div) },
    [%] => { kind: TokenKind::Operator(Operator::Mod) },
    [<] => { kind: TokenKind::Operator(Operator::Lt) },
    [>] => { kind: TokenKind::Operator(Operator::Gt) },
    [<=] => { kind: TokenKind::Operator(Operator::Le) },
    [>=] => { kind: TokenKind::Operator(Operator::Ge) },
    [==] => { kind: TokenKind::Operator(Operator::Eq) },
    [!=] => { kind: TokenKind::Operator(Operator::Ne) },
    [&&] => { kind: TokenKind::Operator(Operator::And) },
    [||] => { kind: TokenKind::Operator(Operator::Or) },
    [!] => { kind: TokenKind::Operator(Operator::Not) },
    [=] => { kind: TokenKind::Operator(Operator::Assign) },
    [,] => { kind: TokenKind::Other(',') },
    [;] => { kind: TokenKind::Other(';') },
    [lpr] => { kind: TokenKind::Other('(') },
    [rpr] => { kind: TokenKind::Other(')') },
    [lbk] => { kind: TokenKind::Other('{') },
    [rbk] => { kind: TokenKind::Other('}') },
    [lbc] => { kind: TokenKind::Other('[') },
    [rbc] => { kind: TokenKind::Other(']') },
    [eof] => { kind: TokenKind::Eof },
  }
}

impl Token![ident] {
    pub(super) fn ident(&self) -> &str {
        match &self.0.kind {
            TokenKind::Ident(value) => value,
            _ => unreachable!(),
        }
    }
}

impl Token![lint] {
    pub(super) fn inner(&self) -> u64 {
        match self.0.kind {
            TokenKind::Int(value) => value,
            _ => unreachable!(),
        }
    }
}

impl Token![lstring] {
    pub(super) fn inner(&self) -> &str {
        match &self.0.kind {
            TokenKind::String(value) => &value.0,
            _ => unreachable!(),
        }
    }
}

// * ------------------------------------------------------------------------------- Statements ------------------------------------------------------------------------------- * //
#[derive(Parse, Debug)]
#[token(Token)]
pub(super) enum Statement {
    GlobalDecl(
        Token![global],
        Token![ident],
        Token![=],
        Expression,
        Token![;],
    ),
    LocalDecl(Token![let], Token![ident], Token![=], Expression, Token![;]),
    Expression(ExpressionStatement),
    End(Token![eof]),
}

#[derive(Debug)]
pub(super) enum ExpressionStatement {
    Expression(Box<Expression>, Token![;]),
    Assign(Box<Assign>),
}

impl<TS> Parse<TS> for ExpressionStatement
where
    TS: TokenStream<Token = Token>,
{
    fn parse(tokens: &mut TS) -> Result<Self> {
        let exp = tokens.parse()?;
        Ok(if <Token![=]>::maybe(tokens)? {
            Self::Assign(Box::new(Assign {
                lval: exp,
                _assign: tokens.parse()?,
                rval: tokens.parse()?,
                _semi: tokens.parse()?,
            }))
        } else {
            Self::Expression(Box::new(exp), tokens.parse()?)
        })
    }

    fn maybe(tokens: &mut TS) -> Result<bool> {
        Ok(<Token![;]>::maybe(tokens)? || Expression::maybe(tokens)?)
    }
}

#[derive(Debug)]
pub(super) struct Assign {
    pub(super) lval: Expression,
    _assign: Token![=],
    pub(super) rval: Expression,
    _semi: Token![;],
}

// * ------------------------------------------------------------------------------- Expressions ------------------------------------------------------------------------------ * //
pub(super) type Expression = NonEmptySepList<AndExp, Token![||]>;
pub(super) type AndExp = NonEmptySepList<EqExp, Token![&&]>;
pub(super) type EqExp = NonEmptySepList<RelExp, EqOps>;

#[derive(Parse, Debug)]
#[token(Token)]
pub(super) enum EqOps {
    Eq(Token![==]),
    Ne(Token![!=]),
}

pub(super) type RelExp = NonEmptySepList<AddExp, RelOps>;

#[derive(Parse, Debug)]
#[token(Token)]
pub(super) enum RelOps {
    Lt(Token![<]),
    Gt(Token![>]),
    Le(Token![<=]),
    Ge(Token![>=]),
}

pub(super) type AddExp = NonEmptySepList<MulExp, AddOps>;

#[derive(Parse, Debug)]
#[token(Token)]
pub(super) enum AddOps {
    Add(Token![+]),
    Sub(Token![-]),
}

pub(super) type MulExp = NonEmptySepList<UnaryExp, MulOps>;

#[derive(Parse, Debug)]
#[token(Token)]
pub(super) enum MulOps {
    Mul(Token![*]),
    Div(Token![/]),
    Mod(Token![%]),
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
pub(super) enum UnaryExp {
    Unary(UnaryOps, Box<Self>),
    Primary(Box<PrimaryExp>),
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
pub(super) enum UnaryOps {
    Pos(Token![+]),
    Neg(Token![-]),
    Not(Token![!]),
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
pub(super) enum PrimaryExp {
    ParenExp(ParenExp),
    FuncCall(FunctionCall),
    Access(Access),
    LInt(Token![lint]),
    LString(Token![lstring]),
    Table(Table),
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
pub(super) struct ParenExp {
    _lpr: Token![lpr],
    pub(super) exp: Expression,
    _rpr: Token![rpr],
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
pub(super) struct Table {
    _lbk: Token![lbk],
    pub(super) values: Option<NonEmptySeq<TableEntry>>,
    _rbk: Token![rbk],
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
pub(super) enum TableEntry {
    Property(Token![ident], Token![=], Expression, Token![;]),
    Indexed(Expression, Token![=], Expression, Token![;]),
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
#[starts_with(Token![ident], Token![lpr])]
pub(super) struct FunctionCall {
    pub(super) name: Token![ident],
    _lpr: Token![lpr],
    pub(super) args: SepSeq<Expression, Token![,]>,
    _rpr: Token![rpr],
}

#[derive(Parse, Debug)]
#[token(Token)]
pub(super) struct Access {
    pub(super) ident: Token![ident],
    pub(super) index: Option<Index>,
}

impl Spanned for Access {
    fn span(&self) -> laps::span::Span {
        match &self.index {
            Some(dim) => self.ident.span().into_end_updated(dim.span()),
            None => self.ident.span(),
        }
    }
}

#[derive(Parse, Spanned, Debug)]
#[token(Token)]
pub(super) struct Index {
    _lbc: Token![lbc],
    pub(super) index: Expression,
    _rbc: Token![rbc],
}

// * --------------------------------------------------------------------------------- Program -------------------------------------------------------------------------------- * //
pub struct Program {
    pub(super) statements: Vec<Statement>,
}

impl Program {
    pub fn parse(source: &str) -> Self {
        let reader = Reader::from(source);
        let lexer = TokenKind::lexer(reader);
        let mut tokens = TokenBuffer::new(lexer);
        let mut statements = Vec::new();
        loop {
            match tokens.parse::<Statement>().unwrap() {
                Statement::End(_) => break,
                statement => statements.push(statement),
            }
        }
        Self { statements }
    }
}
