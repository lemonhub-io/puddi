//! Abstract syntax tree.

use crate::diag::Span;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeExpr {
    Path { segs: Vec<Ident>, args: Vec<TypeArg>, span: Span },
    SelfTy(Span),
    Tuple(Vec<TypeExpr>, Span),
    Fn { is_async: bool, params: Vec<TypeExpr>, ret: Option<Box<TypeExpr>>, span: Span },
    Dyn { segs: Vec<Ident>, args: Vec<TypeArg>, span: Span },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Path { span, .. }
            | TypeExpr::SelfTy(span)
            | TypeExpr::Tuple(_, span)
            | TypeExpr::Fn { span, .. }
            | TypeExpr::Dyn { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeArg {
    Type(TypeExpr),
    Assoc(Ident, TypeExpr),
}

#[derive(Clone, Debug)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<TypeExpr>,
}

#[derive(Clone, Debug)]
pub enum AttrArg {
    Ident(Ident),
    Str(String, Span),
}

#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: Ident,
    pub args: Vec<AttrArg>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vis {
    Private,
    Pub,
    Export,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub attrs: Vec<Attribute>,
    pub vis: Vis,
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ItemKind {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    TypeAlias { name: Ident, generics: Vec<GenericParam>, ty: TypeExpr },
    Const { name: Ident, ty: TypeExpr, value: Expr },
    Test { name: String, body: Block },
}

#[derive(Clone, Debug)]
pub struct Import {
    pub path: Vec<Ident>,
    pub names: Option<Vec<Ident>>,
    pub alias: Option<Ident>,
    pub wit: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
}

#[derive(Clone, Debug)]
pub struct FnDecl {
    pub name: Ident,
    pub is_async: bool,
    pub generics: Vec<GenericParam>,
    pub has_self: bool,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Option<Block>,
    pub attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: Ident,
    pub ty: TypeExpr,
    pub mutable: bool,
    pub public: bool,
}

#[derive(Clone, Debug)]
pub struct StructDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub enum VariantKind {
    Unit,
    Tuple(Vec<TypeExpr>),
    Record(Vec<Field>),
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: Ident,
    pub kind: VariantKind,
}

#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub struct TraitDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub supertraits: Vec<TypeExpr>,
    pub assoc_types: Vec<Ident>,
    pub methods: Vec<FnDecl>,
}

#[derive(Clone, Debug)]
pub struct ImplDecl {
    pub generics: Vec<GenericParam>,
    pub trait_ref: Option<TypeExpr>,
    pub self_ty: TypeExpr,
    pub methods: Vec<FnDecl>,
    pub assoc_types: Vec<(Ident, TypeExpr)>,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let { mutable: bool, pat: Pattern, ty: Option<TypeExpr>, init: Expr, else_block: Option<Block>, span: Span },
    Expr(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Shl,
    Shr,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn text(self) -> &'static str {
        use BinOp::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Rem => "%",
            Shl => "<<",
            Shr => ">>",
            BitAnd => "&",
            BitOr => "|",
            BitXor => "^",
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            And => "&&",
            Or => "||",
        }
    }
}

#[derive(Clone, Debug)]
pub enum StrPiece {
    Lit(String),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub enum Cond {
    Expr(Box<Expr>),
    Let(Pattern, Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pat: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(u128),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(Vec<StrPiece>),
    Unit,
    Name(Ident, Vec<TypeExpr>),
    SelfValue,
    SelfType,
    Field(Box<Expr>, Ident, Vec<TypeExpr>),
    TupleField(Box<Expr>, u32),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Try(Box<Expr>),
    Await(Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Assign(Option<BinOp>, Box<Expr>, Box<Expr>),
    Range(Box<Expr>, Box<Expr>, bool),
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Struct { path: Vec<Ident>, args: Vec<TypeExpr>, fields: Vec<(Ident, Option<Expr>)>, base: Option<Box<Expr>> },
    Block(Block),
    If { cond: Cond, then: Block, els: Option<Box<Expr>> },
    Match { scrut: Box<Expr>, arms: Vec<Arm> },
    Loop(Block),
    While(Cond, Block),
    For(Pattern, Box<Expr>, Block),
    Closure { is_async: bool, params: Vec<(Pattern, Option<TypeExpr>)>, ret: Option<TypeExpr>, body: Box<Expr> },
    Return(Option<Box<Expr>>),
    Break(Option<Box<Expr>>),
    Continue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LitPat {
    Int(i128),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub kind: PatKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatKind {
    Wild,
    Bind(Ident),
    Lit(LitPat),
    Range(LitPat, LitPat, bool),
    Tuple(Vec<Pattern>),
    Path(Vec<Ident>),
    TupleStruct(Vec<Ident>, Vec<Pattern>),
    Struct(Vec<Ident>, Vec<(Ident, Option<Pattern>)>, bool),
    Or(Vec<Pattern>),
    List(Vec<Pattern>, Option<Option<Ident>>),
}
