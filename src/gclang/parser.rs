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
    #[skip(r"\s+|//.+\n")]
    _Skip,
    #[regex(r"global|let|fn|if|else|true|false|unit|return|table|any|with|ctl|effect|resume")]
    Keyword(Keyword),
    #[regex(r"int|bool|String|Array|Table|Any")]
    Type(Type),
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
    Fn,
    If,
    Else,
    True,
    False,
    Unit,
    Return,
    Table,
    Any,
    With,
    Ctl,
    Effect,
    Resume,
}

impl FromStr for Keyword {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "global" => Ok(Self::Global),
            "let" => Ok(Self::Let),
            "fn" => Ok(Self::Fn),
            "if" => Ok(Self::If),
            "else" => Ok(Self::Else),
            "true" => Ok(Self::True),
            "false" => Ok(Self::False),
            "unit" => Ok(Self::Unit),
            "return" => Ok(Self::Return),
            "table" => Ok(Self::Table),
            "any" => Ok(Self::Any),
            "with" => Ok(Self::With),
            "ctl" => Ok(Self::Ctl),
            "effect" => Ok(Self::Effect),
            "resume" => Ok(Self::Resume),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Let => write!(f, "let"),
            Self::Fn => write!(f, "fn"),
            Self::If => write!(f, "if"),
            Self::Else => write!(f, "else"),
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::Unit => write!(f, "unit"),
            Self::Return => write!(f, "return"),
            Self::Table => write!(f, "table"),
            Self::Any => write!(f, "any"),
            Self::With => write!(f, "with"),
            Self::Ctl => write!(f, "ctl"),
            Self::Effect => write!(f, "effect"),
            Self::Resume => write!(f, "resume"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Type {
    Int,
    Bool,
    String,
    Array,
    Table,
    Any,
}

impl FromStr for Type {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "int" => Ok(Self::Int),
            "bool" => Ok(Self::Bool),
            "String" => Ok(Self::String),
            "Array" => Ok(Self::Array),
            "Table" => Ok(Self::Table),
            "Any" => Ok(Self::Any),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Int => write!(f, "int"),
            Self::Bool => write!(f, "bool"),
            Self::String => write!(f, "String"),
            Self::Array => write!(f, "Array"),
            Self::Table => write!(f, "Table"),
            Self::Any => write!(f, "Any"),
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
  #[derive(Clone, Debug, PartialEq)]
  macro Token<TokenKind> {
    [ident] => { kind: TokenKind::Ident(_), prompt: "identifier" },
    [global] => { kind: TokenKind::Keyword(Keyword::Global) },
    [let] => { kind: TokenKind::Keyword(Keyword::Let) },
    [fn] => { kind: TokenKind::Keyword(Keyword::Fn) },
    [if] => { kind: TokenKind::Keyword(Keyword::If) },
    [else] => { kind: TokenKind::Keyword(Keyword::Else) },
    [true] => { kind: TokenKind::Keyword(Keyword::True) },
    [false] => { kind: TokenKind::Keyword(Keyword::False) },
    [unit] => { kind: TokenKind::Keyword(Keyword::Unit) },
    [return] => { kind: TokenKind::Keyword(Keyword::Return) },
    [resume] => { kind: TokenKind::Keyword(Keyword::Resume) },
    [table] => { kind: TokenKind::Keyword(Keyword::Table) },
    [any] => { kind: TokenKind::Keyword(Keyword::Any) },
    [with] => { kind: TokenKind::Keyword(Keyword::With) },
    [ctl] => { kind: TokenKind::Keyword(Keyword::Ctl) },
    [effect] => { kind: TokenKind::Keyword(Keyword::Effect) },
    [type] => { kind: TokenKind::Type(_), prompt: "type" },
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
    [:] => { kind: TokenKind::Other(':') },
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

impl Token![type] {
    pub(super) fn inner(&self) -> &Type {
        match &self.0.kind {
            TokenKind::Type(value) => value,
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
#[derive(Parse, Clone, Debug)]
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
    FnDecl(Box<FnDecl>),
    EffectDecl(EffectDecl),
    If(Box<IfStatement>),
    Return(Token![return], Expression, Token![;]),
    Resume(Token![resume], Expression, Token![;]),
    Expression(ExpressionStatement),
    End(Token![eof]),
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct FnDecl {
    _fn: Token![fn],
    pub(super) name: Token![ident],
    pub(super) block: FnBlock,
}

pub(super) type EffectTag = Token![ident];

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct FnBlock {
    pub(super) signature: FnSignature,
    pub(super) effects: SepSeq<EffectTag, Token![,]>,
    pub(super) expression: Expression,
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct FnSignature {
    lpr: Token![lpr],
    pub(super) args: SepSeq<ArgDef, Token![,]>,
    rpr: Token![rpr],
}

impl Spanned for FnBlock {
    fn span(&self) -> laps::span::Span {
        self.signature
            .lpr
            .span()
            .into_end_updated(self.signature.rpr.span())
    }
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct ArgDef {
    pub(super) name: Token![ident],
    _colon: Token![:],
    pub(super) arg_type: Token![type],
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct EffectDecl {
    _effect: Token![effect],
    pub(super) name: Token![ident],
    _lbk: Token![lbk],
    pub(super) handlers: Option<NonEmptySeq<EffectHandlerDecl>>,
    _rbk: Token![rbk],
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct EffectHandlerDecl {
    _ctl: Token![ctl],
    pub(super) name: Token![ident],
    pub(super) signature: FnSignature,
    _semi: Token![;],
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct IfStatement {
    _if: Token![if],
    pub(super) condition: Expression,
    pub(super) statement: Statement,
    pub(super) else_statement: Option<ElseStatement>,
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct ElseStatement {
    _else: Token![else],
    pub(super) statement: Statement,
}

#[derive(Clone, Debug)]
pub(super) enum ExpressionStatement {
    Expression(Box<Expression>, Option<Token![;]>),
    Assign(Box<Assign>),
}

impl<TS> Parse<TS> for ExpressionStatement
where
    TS: TokenStream<Token = Token>,
{
    fn parse(tokens: &mut TS) -> Result<Self> {
        let expression = tokens.parse()?;
        Ok(if <Token![=]>::maybe(tokens)? {
            Self::Assign(Box::new(Assign {
                lval: expression,
                _assign: tokens.parse()?,
                rval: tokens.parse()?,
                _semi: tokens.parse()?,
            }))
        } else {
            let is_block = if let NonEmptySepList::One(NonEmptySepList::One(
                NonEmptySepList::One(NonEmptySepList::One(NonEmptySepList::One(
                    NonEmptySepList::One(UnaryExpression::Primary(expression)),
                ))),
            )) = &expression
            {
                matches!(expression.as_ref(), PrimaryExpression::Block(_))
            } else {
                false
            };
            Self::Expression(
                Box::new(expression),
                if is_block {
                    None
                } else {
                    Some(tokens.parse()?)
                },
            )
        })
    }

    fn maybe(tokens: &mut TS) -> Result<bool> {
        Ok(<Token![;]>::maybe(tokens)? || Expression::maybe(tokens)?)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Assign {
    pub(super) lval: Expression,
    _assign: Token![=],
    pub(super) rval: Expression,
    _semi: Token![;],
}

// * ------------------------------------------------------------------------------- Expressions ------------------------------------------------------------------------------ * //
pub(super) type Expression = NonEmptySepList<AndExpression, Token![||]>;
pub(super) type AndExpression = NonEmptySepList<EqExpression, Token![&&]>;
pub(super) type EqExpression = NonEmptySepList<RelExpression, EqOps>;

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) enum EqOps {
    Eq(Token![==]),
    Ne(Token![!=]),
}

pub(super) type RelExpression = NonEmptySepList<AddExpression, RelOps>;

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) enum RelOps {
    Lt(Token![<]),
    Gt(Token![>]),
    Le(Token![<=]),
    Ge(Token![>=]),
}

pub(super) type AddExpression = NonEmptySepList<MulExpression, AddOps>;

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) enum AddOps {
    Add(Token![+]),
    Sub(Token![-]),
}

pub(super) type MulExpression = NonEmptySepList<UnaryExpression, MulOps>;

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) enum MulOps {
    Mul(Token![*]),
    Div(Token![/]),
    Mod(Token![%]),
}

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
pub(super) enum UnaryExpression {
    Unary(UnaryOps, Box<Self>),
    Primary(Box<PrimaryExpression>),
}

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
pub(super) enum UnaryOps {
    Pos(Token![+]),
    Neg(Token![-]),
    Not(Token![!]),
}

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
pub(super) enum PrimaryExpression {
    Parens(ParenExpression),
    Block(BlockExpression),
    FuncCall(FunctionCall),
    Access(Access),
    LInt(Token![lint]),
    LString(Token![lstring]),
    LBoolTrue(Token![true]),
    LBoolFalse(Token![false]),
    LUnit(Token![unit]),
    Array(Array),
    Table(Token![table], Table),
    Lambda(Token![fn], FnBlock),
    Any(Token![any]),
}

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
pub(super) struct ParenExpression {
    _lpr: Token![lpr],
    pub(super) exp: Expression,
    _rpr: Token![rpr],
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct BlockExpression {
    lbk: Token![lbk],
    pub(super) with_handlers: Option<NonEmptySeq<WithHandlers>>,
    pub(super) statements: Option<NonEmptySeq<Statement>>,
    rbk: Token![rbk],
}

impl Spanned for BlockExpression {
    fn span(&self) -> laps::span::Span {
        self.lbk.span().into_end_updated(self.rbk.span())
    }
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct WithHandlers {
    _with: Token![with],
    pub(super) effect: Token![ident],
    _lbk: Token![lbk],
    pub(super) handlers: NonEmptySeq<EffectHandler>,
    _rbk: Token![rbk],
}

#[derive(Parse, Clone, Debug)]
#[token(Token)]
pub(super) struct EffectHandler(
    pub(super) Token![ctl],
    pub(super) Token![ident],
    pub(super) FnBlock,
);

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
pub(super) struct Array {
    _lbc: Token![lbc],
    pub(super) values: SepSeq<Expression, Token![,]>,
    _rbc: Token![rbc],
}

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
pub(super) struct Table {
    _lbk: Token![lbk],
    pub(super) values: Option<NonEmptySeq<TableEntry>>,
    _rbk: Token![rbk],
}

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
pub(super) enum TableEntry {
    Property(Token![ident], Token![=], Expression, Token![;]),
    Indexed(Expression, Token![=], Expression, Token![;]),
}

#[derive(Parse, Clone, Spanned, Debug)]
#[token(Token)]
#[starts_with(Token![ident], Token![lpr])]
pub(super) struct FunctionCall {
    pub(super) name: Token![ident],
    _lpr: Token![lpr],
    pub(super) args: SepSeq<Expression, Token![,]>,
    _rpr: Token![rpr],
}

#[derive(Parse, Clone, Debug)]
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

#[derive(Parse, Clone, Spanned, Debug)]
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
    pub fn parse(source: &str) -> anyhow::Result<Self> {
        let reader = Reader::from(source);
        let lexer = TokenKind::lexer(reader);
        let mut tokens = TokenBuffer::new(lexer);
        let mut statements = Vec::new();
        loop {
            match tokens
                .parse::<Statement>()
                .map_err(|_| anyhow::anyhow!("Compilation error!"))?
            {
                Statement::End(_) => break,
                statement => statements.push(statement),
            }
        }
        Ok(Self { statements })
    }
}
