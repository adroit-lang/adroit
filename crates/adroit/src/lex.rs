use std::{fmt, ops::Range};

use enumset::EnumSetType;
use index_vec::{IndexVec, define_index_type};
use logos::Logos;

define_index_type! {
    pub struct ByteIndex = u32;
    IMPL_RAW_CONVERSIONS = true;
}

define_index_type! {
    pub struct ByteLen = u16;
    IMPL_RAW_CONVERSIONS = true;
}

#[derive(Debug, EnumSetType, Logos)]
#[logos(skip r"\s+")]
pub enum TokenKind {
    Eof,

    #[regex("#[^\n]*")]
    Comment,

    #[regex(r"[A-Z_a-z]\w*")]
    Ident,

    #[regex(r"\d+")]
    Int,

    #[regex(r"\d+\.\d+")]
    Float,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token(",")]
    Comma,

    #[token(".")]
    Dot,

    #[token("..")]
    DotDot,

    #[token(":")]
    Colon,

    #[token("=")]
    Equals,

    #[token(";")]
    Semicolon,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Times,

    #[token("/")]
    Divide,

    #[token("+=")]
    PlusEquals,

    #[token("-=")]
    MinusEquals,

    #[token("*=")]
    TimesEquals,

    #[token("/=")]
    DivideEquals,

    #[token("import")]
    Import,

    #[token("func")]
    Func,

    #[token("let")]
    Let,

    #[token("var")]
    Var,

    #[token("for")]
    For,

    #[token("in")]
    In,
}

impl TokenKind {
    pub fn ignore(self) -> bool {
        matches!(self, Self::Comment)
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eof => write!(f, "end of file"),
            Self::Comment => write!(f, "comment"),
            Self::Ident => write!(f, "identifier"),
            Self::Int => write!(f, "integer"),
            Self::Float => write!(f, "number"),
            Self::LParen => write!(f, "`(`"),
            Self::RParen => write!(f, "`)`"),
            Self::LBracket => write!(f, "`[`"),
            Self::RBracket => write!(f, "`]`"),
            Self::LBrace => write!(f, "`{{`"),
            Self::RBrace => write!(f, "`}}`"),
            Self::Comma => write!(f, "`,`"),
            Self::Dot => write!(f, "`.`"),
            Self::DotDot => write!(f, "`..`"),
            Self::Colon => write!(f, "`:`"),
            Self::Equals => write!(f, "`=`"),
            Self::Semicolon => write!(f, "`;`"),
            Self::Plus => write!(f, "`+`"),
            Self::Minus => write!(f, "`-`"),
            Self::Times => write!(f, "`*`"),
            Self::Divide => write!(f, "`/`"),
            Self::PlusEquals => write!(f, "`+=`"),
            Self::MinusEquals => write!(f, "`-=`"),
            Self::TimesEquals => write!(f, "`*=`"),
            Self::DivideEquals => write!(f, "`/=`"),
            Self::Import => write!(f, "`import`"),
            Self::Func => write!(f, "`func`"),
            Self::Let => write!(f, "`let`"),
            Self::Var => write!(f, "`var`"),
            Self::For => write!(f, "`for`"),
            Self::In => write!(f, "`in`"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token {
    pub start: ByteIndex,
    pub len: ByteLen,
    pub kind: TokenKind,
}

impl Token {
    pub fn byte_range(&self) -> Range<usize> {
        let start = self.start.index();
        start..(start + self.len.index())
    }
}

define_index_type! {
    pub struct TokenId = u32;
}

pub type Tokens = IndexVec<TokenId, Token>;

#[derive(Debug)]
pub enum LexError {
    SourceTooLong,
    TokenTooLong { start: ByteIndex, end: ByteIndex },
    InvalidToken { start: ByteIndex, len: ByteLen },
}

impl LexError {
    pub fn byte_range(&self) -> Range<usize> {
        match *self {
            LexError::SourceTooLong => {
                let max = ByteIndex::from(u32::MAX).index();
                max..max
            }
            LexError::TokenTooLong { start, end } => start.index()..end.index(),
            LexError::InvalidToken { start, len } => {
                let start = start.index();
                start..(start + len.index())
            }
        }
    }

    pub fn message(&self) -> &str {
        match self {
            LexError::SourceTooLong => "file size exceeds 4 GiB limit",
            LexError::TokenTooLong { .. } => "token size exceeds 64 KiB limit",
            LexError::InvalidToken { .. } => "invalid token",
        }
    }
}

pub fn lex(source: &str) -> Result<Tokens, LexError> {
    let eof = match u32::try_from(source.len()) {
        Ok(len) => Token {
            start: ByteIndex::from(len),
            len: ByteLen::from(0u16),
            kind: TokenKind::Eof,
        },
        Err(_) => return Err(LexError::SourceTooLong),
    };
    let mut tokens = IndexVec::new();
    for (result, range) in TokenKind::lexer(source).spanned() {
        let start = ByteIndex::from_usize(range.start);
        let end = ByteIndex::from_usize(range.end);
        let len = ByteLen::from(
            u16::try_from(u32::from(end) - u32::from(start))
                .map_err(|_| LexError::TokenTooLong { start, end })?,
        );
        let kind = result.map_err(|_| LexError::InvalidToken { start, len })?;
        tokens.push(Token { start, len, kind });
    }
    tokens.push(eof);
    Ok(tokens)
}
