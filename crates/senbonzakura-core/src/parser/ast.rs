use crate::source::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetBinding),
    Assign(Assignment),
    FuncDef(FuncDef),
    DataDecl(DataDecl),
    If(IfStmt),
    Return(ReturnStmt),
    Expr(ExprStmt),
    TypeclassDecl(TypeclassDecl),
    Impl(ImplBlock),
    For(ForStmt),
    While(WhileStmt),
    Import(ImportStmt),
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Expr,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub target: Expr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeParamDef {
    pub name: String,
    pub constraints: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct DataDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<FuncDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub type_annotation: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_body: Vec<Statement>,
    pub elif_clauses: Vec<(Expr, Vec<Statement>)>,
    pub else_body: Option<Vec<Statement>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeclassDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub methods: Vec<MethodSig>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub kind: ImplKind,
    pub methods: Vec<FuncDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ImplKind {
    Inherent { target_type: String },
    Instance { typeclass_name: String, type_args: Vec<TypeExpr> },
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub var_name: String,
    pub iterable: Expr,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportStmt {
    pub kind: ImportKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Simple {
        module_path: Vec<String>,
        alias: Option<String>,
    },
    From {
        module_path: Vec<String>,
        names: Vec<ImportName>,
    },
}

#[derive(Debug, Clone)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLiteral(i64, Span),
    FloatLiteral(f64, Span),
    StringLiteral(String, Span),
    BoolLiteral(bool, Span),
    NoneLiteral(Span),
    Name(String, Span),
    BinOp(Box<BinOpExpr>),
    UnaryOp(Box<UnaryOpExpr>),
    Call(Box<CallExpr>),
    Attr(Box<AttrExpr>),
    ListLiteral(Vec<Expr>, Span),
    Index(Box<IndexExpr>),
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub object: Expr,
    pub index: Expr,
    pub span: Span,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLiteral(_, s)
            | Expr::FloatLiteral(_, s)
            | Expr::StringLiteral(_, s)
            | Expr::BoolLiteral(_, s)
            | Expr::NoneLiteral(s)
            | Expr::Name(_, s)
            | Expr::ListLiteral(_, s) => *s,
            Expr::BinOp(e) => e.span,
            Expr::UnaryOp(e) => e.span,
            Expr::Call(e) => e.span,
            Expr::Attr(e) => e.span,
            Expr::Index(e) => e.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinOpExpr {
    pub left: Expr,
    pub op: BinOp,
    pub right: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct UnaryOpExpr {
    pub op: UnaryOp,
    pub operand: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Expr,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AttrExpr {
    pub object: Expr,
    pub attr: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String, Span),
    Parameterized(String, Vec<TypeExpr>, Span),
    Function(Vec<TypeExpr>, Box<TypeExpr>, Span),
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named(_, s) | TypeExpr::Parameterized(_, _, s) | TypeExpr::Function(_, _, s) => *s,
        }
    }
}
