use logos::Logos;

pub mod csharp_gen;
pub mod fmt;
pub mod monomorphize;
pub mod release_gen;
pub mod units;
pub use fmt::{
    format_expr, format_program, format_program_with_dialect, format_program_with_options,
    format_source,
};

// AST

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Rust,
    Odin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraceStyle {
    Kr,
    Allman,
}

pub fn detect_format_options_for_path(file_path: &std::path::Path) -> (Dialect, BraceStyle) {
    let mut dir = if file_path.is_dir() {
        Some(file_path)
    } else {
        file_path.parent()
    };
    while let Some(d) = dir {
        let cfg = d.join("striv.toml");
        if let Ok(txt) = std::fs::read_to_string(&cfg) {
            let mut dialect = Dialect::Rust;
            let mut braces = BraceStyle::Kr;
            for line in txt.lines() {
                let clean = line.trim().replace(' ', "");
                if clean.starts_with("syntax=\"odin\"") || clean.starts_with("syntax='odin'") {
                    dialect = Dialect::Odin;
                }
                if clean.starts_with("braces=\"allman\"") || clean.starts_with("braces='allman'") {
                    braces = BraceStyle::Allman;
                }
            }
            return (dialect, braces);
        }
        dir = d.parent();
    }
    (Dialect::Rust, BraceStyle::Kr)
}

pub fn detect_dialect_for_path(file_path: &std::path::Path) -> Dialect {
    detect_format_options_for_path(file_path).0
}

#[derive(Debug, Clone)]
pub enum TweakVal {
    Int(i64),
    Str(String),
    Vec(Vec<i64>),
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub field_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub is_open: bool,
    pub is_extend: bool,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub enum MatchPat {
    Wildcard,
    Expr(Expr),
    EnumDestructure {
        enum_name: String,
        variant: String,
        bindings: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum UnitExpr {
    Name(String),
    Mul(Box<UnitExpr>, Box<UnitExpr>),
    Div(Box<UnitExpr>, Box<UnitExpr>),
}

#[derive(Debug, Clone)]
pub struct UnitDef {
    pub name: String,
    pub formula: Option<UnitExpr>,
}

#[derive(Debug, Clone)]
pub struct ExternFn {
    pub lib: String,
    pub name: String,
    pub params: Vec<(String, String)>,
    pub ret_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<String>,
    pub tweaks: Vec<(String, TweakVal)>,
    pub consts: Vec<(String, Expr)>,
    pub units: Vec<UnitDef>,
    pub rust_blocks: Vec<String>,
    pub extern_fns: Vec<ExternFn>,
    pub enums: Vec<EnumDef>,
    pub structs: Vec<StructDef>,
    pub functions: Vec<Function>,
    pub tests: Vec<(String, Vec<Stmt>)>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<(String, String)>, // (field_name, type_name)
    pub sync_mask: u64,
    pub default_fields: Vec<(String, Expr)>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub is_rpc: bool,
    pub rust_body: Option<String>,
    pub type_params: Vec<String>,
    pub params: Vec<(String, String, bool)>,
    pub ret_type: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Var(String),
    Field { var: String, field: String },
    Index { var: String, index: Expr },
}

#[derive(Debug, Clone)]
pub struct SelectRecvArm {
    pub bind_var: String,
    pub channel_var: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        ty_hint: Option<String>,
        is_mut: bool,
        value: Expr,
    },
    Declare {
        name: String,
        ty: String,
        is_mut: bool,
    },
    ForEach {
        var: String,
        is_mut: bool,
        is_parallel: bool,
        collection: String,
        body: Vec<Stmt>,
    },
    Concurrent(Vec<Stmt>),
    Spawn {
        name: String,
        args: Vec<Expr>,
    },
    Shared {
        name: String,
        value: Expr,
    },
    Lock {
        var: String,
        body: Vec<Stmt>,
    },
    Select {
        arms: Vec<SelectRecvArm>,
        timeout: Option<(Expr, Vec<Stmt>)>,
    },
    Defer(Box<Stmt>),
    Block(Vec<Stmt>),
    Yield,
    Break,
    Continue,
    Query {
        entity_var: Option<String>,
        terms: Vec<(String, String, bool)>,
        without: Vec<String>,
        ecs_var: String,
        is_parallel: bool,
        body: Vec<Stmt>,
    },
    BlockCall {
        name: String,
        args: Vec<Expr>,
        body: Vec<Stmt>,
    },
    LetTuple {
        names: Vec<(String, bool)>,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    FieldAssign {
        var: String,
        field: String,
        value: Expr,
    },
    IndexAssign {
        var: String,
        index: Expr,
        value: Expr,
    },
    Expr(Expr), // e.g. p.hp = 80
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Vec<Stmt>,
    },
    Print(Expr),
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Match {
        target: Expr,
        arms: Vec<(MatchPat, Vec<Stmt>)>,
    },
    ForRange {
        var: String,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
    },
    CompoundAssign {
        target: AssignTarget,
        op: Op,
        value: Expr,
    },
    Return(Expr),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: String,
        method: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Hex(i64),
    Null,
    Float(f64),
    UnitFloat(f64, String),
    PathConst {
        ty: String,
        member: String,
    },
    StrInterp(Vec<StringPart>),
    Ok(Box<Expr>),
    Err(Box<Expr>),
    Some(Box<Expr>),
    None,
    Try(Box<Expr>),
    Tuple(Vec<Expr>),
    TupleIndex {
        tuple: Box<Expr>,
        idx: usize,
    },
    VecLit(Vec<Expr>),
    Index {
        var: String,
        index: Box<Expr>,
    },
    Var(String),
    BinOp {
        left: Box<Expr>,
        op: Op,
        right: Box<Expr>,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    }, // e.g. Player { hp: 100, x: 10 }
    FieldAccess {
        var: String,
        field: String,
    }, // e.g. p.hp
    Call {
        name: String,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: String,
        method: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Eq,
    NotEq,
    And,
    Or,
    BitOr,
    BitAnd,
    BitXor,
    Shl,
    Shr,
}

// Lexer
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
pub enum Token {
    #[token("fn")]
    Fn,
    #[token("extern")]
    Extern,
    #[token("foreign")]
    Foreign,
    #[token("const")]
    Const,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("null")]
    Null,
    #[token("enum")]
    Enum,
    #[token("match")]
    Match,
    #[token("_", priority = 3)]
    Underscore,
    #[token("use")]
    Use,
    #[token("import")]
    Import,
    #[token("tweak")]
    Tweak,
    #[token("struct")]
    Struct,
    #[token("impl")]
    Impl,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("return")]
    Return,
    #[token("print")]
    Print,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("concurrent")]
    Concurrent,
    #[token("spawn")]
    Spawn,
    #[token("shared")]
    Shared,
    #[token("lock")]
    Lock,
    #[token("parallel")]
    Parallel,
    #[token("select")]
    Select,
    #[token("timeout")]
    Timeout,
    #[token("defer")]
    Defer,
    #[token("yield")]
    Yield,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("test")]
    Test,
    #[token("sync")]
    Sync,
    #[token("rpc")]
    Rpc,
    #[token("open")]
    Open,
    #[token("extend")]
    Extend,
    #[token("unit")]
    Unit,
    #[token("rust")]
    Rust,
    #[token("query")]
    Query,
    #[token("proc")]
    Proc,
    #[token(":=")]
    ColonAssign,
    #[regex(r"[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9][0-9_]*)?_[a-zA-Z][a-zA-Z0-9]*", |lex| {
        let s = lex.slice();
        let unit_idx = s.rfind('_').unwrap();
        let num_s = s[..unit_idx].replace('_', "");
        let unit_s = &s[unit_idx + 1..];
        let val: f64 = num_s.parse().unwrap();
        (val, unit_s.to_string())
    }, priority = 5)]
    UnitFloat((f64, String)),
    #[token("Ok")]
    OkKw,
    #[token("Err")]
    ErrKw,
    #[token("Some")]
    SomeKw,
    #[token("None")]
    NoneKw,
    #[token("?")]
    Question,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("::")]
    ColonColon,
    #[token(":")]
    Colon,
    #[token("..")]
    DotDot,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("->")]
    Arrow,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("!")]
    Bang,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("|")]
    Pipe,
    #[token("&")]
    Amp,
    #[token("^")]
    Caret,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("=>")]
    FatArrow,
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,

    #[regex(r"[0-9][0-9_]*(\.[0-9][0-9_]*)?[eE][+-]?[0-9][0-9_]*|[0-9][0-9_]*\.[0-9][0-9_]*", |lex| {
        lex.slice().replace('_', "").parse::<f64>().ok()
    }, priority = 4)]
    Float(f64),
    #[regex("0x[0-9a-fA-F]+", |lex| i64::from_str_radix(&lex.slice()[2..], 16).ok())]
    Hex(i64),
    #[regex(r"[0-9][0-9_]*", |lex| lex.slice().replace('_', "").parse::<i64>().ok(), priority = 3)]
    Int(i64),

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len() - 1].to_string())
    })]
    Str(String),
}

impl Token {
    pub fn display_name(&self) -> String {
        match self {
            Token::Fn => "`fn`".to_string(),
            Token::Extern => "`extern`".to_string(),
            Token::Foreign => "`foreign`".to_string(),
            Token::Const => "`const`".to_string(),
            Token::True => "`true`".to_string(),
            Token::False => "`false`".to_string(),
            Token::Null => "`null`".to_string(),
            Token::Enum => "`enum`".to_string(),
            Token::Match => "`match`".to_string(),
            Token::Underscore => "`_`".to_string(),
            Token::FatArrow => "`=>`".to_string(),
            Token::Use => "`use`".to_string(),
            Token::Import => "`import`".to_string(),
            Token::Tweak => "`tweak`".to_string(),
            Token::Let => "`let`".to_string(),
            Token::Mut => "`mut`".to_string(),
            Token::Struct => "`struct`".to_string(),
            Token::Impl => "`impl`".to_string(),
            Token::If => "`if`".to_string(),
            Token::Else => "`else`".to_string(),
            Token::While => "`while`".to_string(),
            Token::For => "`for`".to_string(),
            Token::In => "`in`".to_string(),
            Token::Concurrent => "`concurrent`".to_string(),
            Token::Spawn => "`spawn`".to_string(),
            Token::Shared => "`shared`".to_string(),
            Token::Lock => "`lock`".to_string(),
            Token::Parallel => "`parallel`".to_string(),
            Token::Select => "`select`".to_string(),
            Token::Timeout => "`timeout`".to_string(),
            Token::Defer => "`defer`".to_string(),
            Token::Yield => "`yield`".to_string(),
            Token::Break => "`break`".to_string(),
            Token::Continue => "`continue`".to_string(),
            Token::Test => "`test`".to_string(),
            Token::Sync => "`sync`".to_string(),
            Token::Rpc => "`rpc`".to_string(),
            Token::Open => "`open`".to_string(),
            Token::Extend => "`extend`".to_string(),
            Token::Unit => "`unit`".to_string(),
            Token::Rust => "`rust`".to_string(),
            Token::Query => "`query`".to_string(),
            Token::Proc => "`proc`".to_string(),
            Token::ColonAssign => "`:=`".to_string(),
            Token::UnitFloat((v, u)) => format!("{}_{}", v, u),
            Token::OkKw => "`Ok`".to_string(),
            Token::ErrKw => "`Err`".to_string(),
            Token::SomeKw => "`Some`".to_string(),
            Token::NoneKw => "`None`".to_string(),
            Token::Question => "`?`".to_string(),
            Token::Return => "`return`".to_string(),
            Token::Print => "`print`".to_string(),
            Token::LParen => "`(`".to_string(),
            Token::RParen => "`)`".to_string(),
            Token::LBrace => "`{`".to_string(),
            Token::RBrace => "`}`".to_string(),
            Token::LBracket => "`[`".to_string(),
            Token::RBracket => "`]`".to_string(),
            Token::ColonColon => "`::`".to_string(),
            Token::Colon => "`:`".to_string(),
            Token::Comma => "`,`".to_string(),
            Token::DotDot => "`..`".to_string(),
            Token::Dot => "`.`".to_string(),
            Token::Arrow => "`->`".to_string(),
            Token::PlusEq => "`+=`".to_string(),
            Token::MinusEq => "`-=`".to_string(),
            Token::Assign => "`=`".to_string(),
            Token::EqEq => "`==`".to_string(),
            Token::NotEq => "`!=`".to_string(),
            Token::Bang => "`!`".to_string(),
            Token::LtEq => "`<=`".to_string(),
            Token::GtEq => "`>=`".to_string(),
            Token::AndAnd => "`&&`".to_string(),
            Token::OrOr => "`||`".to_string(),
            Token::Pipe => "`|`".to_string(),
            Token::Amp => "`&`".to_string(),
            Token::Caret => "`^`".to_string(),
            Token::Shl => "`<<`".to_string(),
            Token::Shr => "`>>`".to_string(),
            Token::Plus => "`+`".to_string(),
            Token::Minus => "`-`".to_string(),
            Token::Star => "`*`".to_string(),
            Token::Slash => "`/`".to_string(),
            Token::Lt => "`<`".to_string(),
            Token::Gt => "`>`".to_string(),
            Token::Float(f) => format!("number `{}`", f),
            Token::Hex(h) => format!("`0x{:X}`", h),
            Token::Int(n) => format!("number `{}`", n),
            Token::Ident(s) => format!("`{}`", s),
            Token::Str(s) => format!("\"{}\"", s),
        }
    }
}

// Parser

pub struct ParseError {
    pub message: String,
    pub span: std::ops::Range<usize>,
}

pub struct Parser {
    tokens: Vec<(Token, std::ops::Range<usize>)>,
    src: String,
    src_len: usize,
    pos: usize,
    pub dialect: Dialect,
}

impl Parser {
    pub fn new(src: &str) -> Self {
        Self::new_with_dialect(src, Dialect::Rust)
    }

    pub fn new_with_dialect(src: &str, dialect: Dialect) -> Self {
        let tokens = Token::lexer(src)
            .spanned()
            .filter_map(|(res, span)| res.ok().map(|tok| (tok, span)))
            .collect();
        Self {
            tokens,
            src: src.to_string(),
            src_len: src.len(),
            pos: 0,
            dialect,
        }
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset).map(|(t, _)| t)
    }

    fn current_span(&self) -> std::ops::Range<usize> {
        self.tokens
            .get(self.pos)
            .map(|(_, s)| s.clone())
            .unwrap_or(self.src_len..self.src_len)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).map(|(t, _)| t.clone());
        self.pos += 1;
        t
    }

    #[allow(dead_code)]
    fn error(&self, msg: impl Into<String>) -> ! {
        let s = self.current_span();
        panic!("SPAN:{}..{}:{}", s.start, s.end, msg.into());
    }

    fn expect(&mut self, expected: Token) {
        let span = self.current_span();
        let got = self.next();
        if got.as_ref() != Some(&expected) {
            let got_str = got
                .map(|t| t.display_name())
                .unwrap_or_else(|| "end of file".to_string());
            panic!(
                "SPAN:{}..{}:Expected {}, but found {}",
                span.start,
                span.end,
                expected.display_name(),
                got_str
            );
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut imports = Vec::new();
        let mut tweaks = Vec::new();
        let mut consts = Vec::new();
        let mut units = Vec::new();
        let mut rust_blocks = Vec::new();
        let mut extern_fns = Vec::new();
        let mut enums = Vec::new();
        let mut structs = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        while let Some(tok) = self.peek() {
            match tok {
                Token::Use => {
                    self.next(); // consume `use`
                    let Some(Token::Ident(mod_name)) = self.next() else {
                        self.error("Expected module name after `use` (e.g. `use player`)");
                    };
                    let mut mod_path = mod_name;
                    while self.peek() == Some(&Token::ColonColon) {
                        self.next(); // `::`
                        if self.peek() == Some(&Token::LBrace) {
                            self.next(); // `{`
                            while self.peek().is_some() && self.peek() != Some(&Token::RBrace) {
                                self.next();
                            }
                            self.expect(Token::RBrace);
                            break;
                        } else if let Some(Token::Ident(seg)) = self.peek().cloned() {
                            self.next();
                            mod_path.push('/');
                            mod_path.push_str(&seg);
                        } else {
                            self.error("Expected segment after `::` in `use`");
                        }
                    }
                    imports.push(mod_path);
                }
                Token::Import => {
                    self.next(); // consume `import`
                    match self.next() {
                        Some(Token::Str(s)) => imports.push(s.replace("::", "/")),
                        Some(Token::Ident(id)) => imports.push(id),
                        _ => self.error(
                            "Expected path string after `import` (e.g. `import \"physics/orbit\"`)",
                        ),
                    }
                }
                Token::Tweak => {
                    self.next(); // consume `tweak`
                    let Some(Token::Ident(t_name)) = self.next() else {
                        self.error("Expected name after `tweak`");
                    };
                    self.expect(Token::Assign);
                    let val = match self.next() {
                        Some(Token::Int(v)) => TweakVal::Int(v),
                        Some(Token::Str(s)) => TweakVal::Str(s),
                        Some(Token::LBracket) => {
                            let mut items = Vec::new();
                            while self.peek() != Some(&Token::RBracket) {
                                let Some(Token::Int(n)) = self.next() else {
                                    self.error("Expected integer in `tweak` list");
                                };
                                items.push(n);
                                if self.peek() == Some(&Token::Comma) {
                                    self.next();
                                }
                            }
                            self.expect(Token::RBracket);
                            TweakVal::Vec(items)
                        }
                        _ => self.error("Expected integer, string, or list after `=` in `tweak`"),
                    };
                    tweaks.push((t_name, val));
                }
                Token::Enum => enums.push(self.parse_enum()),
                Token::Open => {
                    self.next();
                    let mut edef = self.parse_enum();
                    edef.is_open = true;
                    enums.push(edef);
                }
                Token::Extend => {
                    self.next();
                    let mut edef = self.parse_enum();
                    edef.is_open = true;
                    edef.is_extend = true;
                    enums.push(edef);
                }
                Token::Struct => structs.push(self.parse_struct()),
                Token::Impl => {
                    self.next(); // consume `impl`
                    let Some(Token::Ident(struct_name)) = self.next() else {
                        panic!("Struct name expected after `impl`");
                    };
                    self.expect(Token::LBrace);
                    while self.peek() != Some(&Token::RBrace) {
                        functions.push(self.parse_method(&struct_name));
                    }
                    self.expect(Token::RBrace);
                }
                Token::Fn => functions.push(self.parse_function()),
                Token::Rust => {
                    self.next();
                    if self.peek() == Some(&Token::Fn) {
                        let mut f = self.parse_function_sig_only();
                        f.rust_body = Some(self.parse_raw_brace_block());
                        functions.push(f);
                    } else if self.peek() == Some(&Token::Proc) {
                        self.next();
                        let Some(Token::Ident(name)) = self.next() else {
                            panic!("Function Name expected")
                        };
                        let mut f = self.parse_function_sig_rest(name);
                        f.rust_body = Some(self.parse_raw_brace_block());
                        functions.push(f);
                    } else {
                        rust_blocks.push(self.parse_raw_brace_block());
                    }
                }
                Token::Rpc => {
                    self.next();
                    let mut f = self.parse_function();
                    f.is_rpc = true;
                    functions.push(f);
                }
                Token::Test => {
                    self.next();
                    let Some(Token::Str(t_name)) = self.next() else {
                        self.error("Expected test name string after `test`");
                    };
                    let body = self.parse_block();
                    tests.push((t_name, body));
                }
                Token::Const => {
                    self.next(); // consume `const`
                    let Some(Token::Ident(c_name)) = self.next() else {
                        self.error("Expected name after `const`");
                    };
                    if self.peek() == Some(&Token::Colon) {
                        self.next();
                        let _ty = self.parse_type_name();
                    }
                    self.expect(Token::Assign);
                    let expr = self.parse_expr();
                    consts.push((c_name, expr));
                }
                Token::Ident(_)
                    if self.dialect == Dialect::Odin
                        && self.peek_at(1) == Some(&Token::ColonColon) =>
                {
                    let Some(Token::Ident(decl_name)) = self.next() else {
                        unreachable!()
                    };
                    self.expect(Token::ColonColon);
                    match self.peek() {
                        Some(Token::Struct) => {
                            self.next();
                            structs.push(self.parse_struct_rest(decl_name));
                        }
                        Some(Token::Open) => {
                            self.next();
                            self.expect(Token::Enum);
                            let mut e = self.parse_enum_rest(decl_name);
                            e.is_open = true;
                            enums.push(e);
                        }
                        Some(Token::Extend) => {
                            self.next();
                            self.expect(Token::Enum);
                            let mut e = self.parse_enum_rest(decl_name);
                            e.is_open = true;
                            e.is_extend = true;
                            enums.push(e);
                        }
                        Some(Token::Enum) => {
                            self.next();
                            enums.push(self.parse_enum_rest(decl_name));
                        }
                        Some(Token::Rpc) => {
                            self.next();
                            self.expect(Token::Proc);
                            let mut f = self.parse_proc_rest(decl_name);
                            f.is_rpc = true;
                            functions.push(f);
                        }
                        Some(Token::Proc) => {
                            self.next();
                            functions.push(self.parse_proc_rest(decl_name));
                        }
                        _ => {
                            let val = self.parse_expr();
                            consts.push((decl_name, val));
                        }
                    }
                }
                Token::Unit => {
                    self.next();
                    let Some(Token::Ident(u_name)) = self.next() else {
                        self.error("Expected unit name after `unit`");
                    };
                    let formula = if self.peek() == Some(&Token::Assign) {
                        self.next();
                        Some(self.parse_unit_expr())
                    } else {
                        None
                    };
                    units.push(UnitDef {
                        name: u_name,
                        formula,
                    });
                }
                Token::Extern | Token::Foreign => {
                    self.next();
                    if let Some(Token::Ident(kw)) = self.peek() {
                        if kw == "import" {
                            self.next();
                        }
                    }
                    let Some(Token::Str(lib_name)) = self.next() else {
                        self.error("Expected library path string after `extern`/`foreign`");
                    };
                    if self.peek() == Some(&Token::LBrace) {
                        self.next();
                        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
                            extern_fns.push(self.parse_extern_fn_decl(&lib_name));
                        }
                        self.expect(Token::RBrace);
                    } else {
                        extern_fns.push(self.parse_extern_fn_decl(&lib_name));
                    }
                }
                t => panic!(
                    "Expected `tweak`, `const`, `struct`, `impl`, or `fn`, found: {:?}",
                    t
                ),
            }
        }
        Program {
            imports,
            tweaks,
            consts,
            units,
            rust_blocks,
            extern_fns,
            enums,
            structs,
            functions,
            tests,
        }
    }

    fn parse_extern_fn_decl(&mut self, lib: &str) -> ExternFn {
        let name = if self.peek() == Some(&Token::Fn) {
            self.next();
            let Some(Token::Ident(n)) = self.next() else {
                self.error("Expected function name after `fn`");
            };
            n
        } else {
            let Some(Token::Ident(n)) = self.next() else {
                self.error("Expected extern function name");
            };
            self.expect(Token::ColonColon);
            self.expect(Token::Proc);
            n
        };
        self.expect(Token::LParen);
        let mut params = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            let Some(Token::Ident(p_name)) = self.next() else {
                self.error("Expected parameter name");
            };
            self.expect(Token::Colon);
            let ty_name = self.parse_type_name();
            params.push((p_name, ty_name));
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RParen);
        let ret_type = if self.peek() == Some(&Token::Arrow) {
            self.next();
            Some(self.parse_type_name())
        } else {
            None
        };
        ExternFn {
            lib: lib.to_string(),
            name,
            params,
            ret_type,
        }
    }

    fn parse_unit_expr(&mut self) -> UnitExpr {
        let mut left = self.parse_unit_atom();
        while matches!(self.peek(), Some(Token::Star) | Some(Token::Slash)) {
            let is_mul = self.next() == Some(Token::Star);
            let right = self.parse_unit_atom();
            left = if is_mul {
                UnitExpr::Mul(Box::new(left), Box::new(right))
            } else {
                UnitExpr::Div(Box::new(left), Box::new(right))
            };
        }
        left
    }

    fn parse_unit_atom(&mut self) -> UnitExpr {
        if self.peek() == Some(&Token::LParen) {
            self.next();
            let inner = self.parse_unit_expr();
            self.expect(Token::RParen);
            return inner;
        }
        let Some(Token::Ident(name)) = self.next() else {
            self.error("Expected unit name or '(' in unit formula");
        };
        UnitExpr::Name(name)
    }

    fn parse_query_stmt(&mut self, is_parallel: bool) -> Stmt {
        self.expect(Token::Query);
        self.expect(Token::LParen);
        let mut entity_var = None;
        let mut terms = Vec::new();
        let mut without = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            if self.peek() == Some(&Token::Bang) {
                self.next();
                let excluded_ty = self.parse_type_name();
                without.push(excluded_ty);
            } else {
                let is_mut = if self.peek() == Some(&Token::Mut) {
                    self.next();
                    true
                } else {
                    false
                };
                let Some(Token::Ident(v_name)) = self.next() else {
                    self.error("Expected variable name in `query`");
                };
                self.expect(Token::Colon);
                let ty = self.parse_type_name();
                if ty == "entity" {
                    entity_var = Some(v_name);
                } else {
                    terms.push((v_name, ty, is_mut));
                }
            }
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RParen);
        self.expect(Token::In);
        let Some(Token::Ident(ecs_var)) = self.next() else {
            self.error("Expected ECS variable after `in`");
        };
        let body = self.parse_block();
        Stmt::Query {
            entity_var,
            terms,
            without,
            ecs_var,
            is_parallel,
            body,
        }
    }

    fn parse_raw_brace_block(&mut self) -> String {
        self.expect(Token::LBrace);
        let open_end = self.tokens[self.pos - 1].1.end;
        let mut depth = 1usize;
        while self.pos < self.tokens.len() && depth > 0 {
            match self.tokens[self.pos].0 {
                Token::LBrace => depth += 1,
                Token::RBrace => depth -= 1,
                _ => {}
            }
            self.pos += 1;
        }
        if depth != 0 {
            self.error("Unclosed `{` in `rust` block");
        }
        let close_start = self.tokens[self.pos - 1].1.start;
        self.src[open_end..close_start].to_string()
    }

    fn parse_enum(&mut self) -> EnumDef {
        self.expect(Token::Enum);
        let Some(Token::Ident(name)) = self.next() else {
            self.error("Expected enum name after `enum`");
        };
        self.parse_enum_rest(name)
    }

    fn parse_enum_rest(&mut self, name: String) -> EnumDef {
        self.expect(Token::LBrace);
        let mut variants = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let Some(Token::Ident(v_name)) = self.next() else {
                self.error("Expected variant name in `enum`");
            };
            let mut field_types = Vec::new();
            if self.peek() == Some(&Token::LParen) {
                self.next(); // `(`
                while self.peek() != Some(&Token::RParen) {
                    field_types.push(self.parse_type_name());
                    if self.peek() == Some(&Token::Comma) {
                        self.next();
                    }
                }
                self.expect(Token::RParen);
            }
            variants.push(EnumVariant {
                name: v_name,
                field_types,
            });
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RBrace);
        EnumDef {
            name,
            is_open: false,
            is_extend: false,
            variants,
        }
    }

    fn parse_struct(&mut self) -> StructDef {
        self.expect(Token::Struct);
        let Some(Token::Ident(name)) = self.next() else {
            panic!("Struct name expected")
        };
        self.parse_struct_rest(name)
    }

    fn parse_struct_rest(&mut self, name: String) -> StructDef {
        let type_params = self.parse_type_params();
        self.expect(Token::LBrace);
        let mut fields = Vec::new();
        let mut sync_mask = 0u64;
        let mut default_fields = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let is_sync = if self.peek() == Some(&Token::Sync) {
                self.next();
                true
            } else {
                false
            };
            let Some(Token::Ident(f_name)) = self.next() else {
                self.error("Expected field name")
            };
            self.expect(Token::Colon);
            let Some(Token::Ident(f_ty)) = self.next() else {
                self.error("Expected field type")
            };
            if is_sync && fields.len() < 64 {
                sync_mask |= 1u64 << fields.len();
            }
            if self.peek() == Some(&Token::Assign) {
                self.next();
                default_fields.push((f_name.clone(), self.parse_expr()));
            }
            fields.push((f_name, f_ty));
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RBrace);
        StructDef {
            name,
            type_params,
            fields,
            sync_mask,
            default_fields,
        }
    }

    fn parse_method(&mut self, struct_name: &str) -> Function {
        self.expect(Token::Fn);
        let Some(Token::Ident(m_name)) = self.next() else {
            panic!("Method name expected")
        };
        self.expect(Token::LParen);

        let mut params = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            let is_mut = if self.peek() == Some(&Token::Mut) {
                self.next();
                true
            } else {
                false
            };
            let Some(Token::Ident(p_name)) = self.next() else {
                panic!("Parameter expected")
            };

            if p_name == "self" {
                params.push(("self".to_string(), struct_name.to_string(), is_mut));
            } else {
                self.expect(Token::Colon);
                let Some(Token::Ident(ty_name)) = self.next() else {
                    panic!("Type expected")
                };
                params.push((p_name, ty_name, is_mut));
            }
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RParen);
        let ret_type = if self.peek() == Some(&Token::Arrow) {
            self.next();
            let Some(Token::Ident(rt)) = self.next() else {
                self.error("Expected return type after `->`");
            };
            Some(rt)
        } else {
            None
        };
        Function {
            name: format!("{}__{}", struct_name, m_name),
            is_rpc: false,
        rust_body: None,
            type_params: Vec::new(),
            params,
            ret_type,
            body: self.parse_block(),
        }
    }

    pub fn parse_function(&mut self) -> Function {
        let mut f = self.parse_function_sig_only();
        f.body = self.parse_block();
        f
    }

    fn parse_proc_rest(&mut self, name: String) -> Function {
        let mut f = self.parse_function_sig_rest(name);
        f.body = self.parse_block();
        f
    }

    fn parse_function_sig_only(&mut self) -> Function {
        self.expect(Token::Fn);
        let Some(Token::Ident(name)) = self.next() else {
            panic!("Function Name expected")
        };
        self.parse_function_sig_rest(name)
    }

    fn parse_function_sig_rest(&mut self, name: String) -> Function {
        let type_params = self.parse_type_params();
        self.expect(Token::LParen);

        let mut params = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            let is_shared = if self.peek() == Some(&Token::Shared) {
                self.next();
                true
            } else {
                false
            };
            let is_mut = if !is_shared && self.peek() == Some(&Token::Mut) {
                self.next();
                true
            } else {
                false
            };
            let Some(Token::Ident(p_name)) = self.next() else {
                panic!("Param name expected")
            };
            self.expect(Token::Colon);
            let raw_ty = self.parse_type_name();
            let ty_name = if is_shared {
                format!("shared:{}", raw_ty)
            } else {
                raw_ty
            };
            params.push((p_name, ty_name, is_mut));
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RParen);
        let ret_type = if self.peek() == Some(&Token::Arrow) {
            self.next();
            let Some(Token::Ident(rt)) = self.next() else {
                self.error("Expected return type after `->`");
            };
            Some(rt)
        } else {
            None
        };
        Function {
            name,
            is_rpc: false,
        rust_body: None,
            type_params,
            params,
            ret_type,
            body: Vec::new(),
        }
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        self.expect(Token::LBrace);
        let mut stmts = Vec::new();
        while self.peek().is_some() && self.peek() != Some(&Token::RBrace) {
            stmts.push(self.parse_stmt());
        }
        self.expect(Token::RBrace);
        stmts
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek() {
            Some(Token::Let) => {
                self.next();
                if self.peek() == Some(&Token::LParen) {
                    self.next(); // `(`
                    let mut names = Vec::new();
                    while self.peek() != Some(&Token::RParen) {
                        let is_mut = if self.peek() == Some(&Token::Mut) {
                            self.next();
                            true
                        } else {
                            false
                        };
                        let Some(Token::Ident(n)) = self.next() else {
                            self.error("Expected variable name in tuple");
                        };
                        names.push((n, is_mut));
                        if self.peek() == Some(&Token::Comma) {
                            self.next();
                        }
                    }
                    self.expect(Token::RParen);
                    self.expect(Token::Assign);
                    Stmt::LetTuple {
                        names,
                        value: self.parse_expr(),
                    }
                } else {
                    let is_mut = if self.peek() == Some(&Token::Mut) {
                        self.next();
                        true
                    } else {
                        false
                    };
                    let Some(Token::Ident(name)) = self.next() else {
                        panic!("Name expected")
                    };
                    let ty_hint = if self.peek() == Some(&Token::Colon) {
                        self.next(); // `:`
                        Some(self.parse_type_name())
                    } else {
                        None
                    };
                    self.expect(Token::Assign);
                    Stmt::Let {
                        name,
                        ty_hint,
                        is_mut,
                        value: self.parse_expr(),
                    }
                }
            }
            Some(Token::If) => {
                self.next();
                let cond = self.parse_expr();
                let then_block = self.parse_block();
                let else_block = if self.peek() == Some(&Token::Else) {
                    self.next();
                    self.parse_block()
                } else {
                    Vec::new()
                };
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                }
            }
            Some(Token::Print) => {
                self.next();
                self.expect(Token::LParen);
                let expr = self.parse_expr();
                self.expect(Token::RParen);
                Stmt::Print(expr)
            }
            Some(Token::Break) => {
                self.next();
                Stmt::Break
            }
            Some(Token::Continue) => {
                self.next();
                Stmt::Continue
            }
            Some(Token::Defer) => {
                self.next();
                if self.peek() == Some(&Token::LBrace) {
                    Stmt::Defer(Box::new(Stmt::Block(self.parse_block())))
                } else {
                    Stmt::Defer(Box::new(self.parse_stmt()))
                }
            }
            Some(Token::Yield) => {
                self.next();
                if self.peek() == Some(&Token::LParen) {
                    self.next();
                    self.expect(Token::RParen);
                }
                Stmt::Yield
            }
            Some(Token::Select) => {
                self.next();
                self.expect(Token::LBrace);
                let mut arms = Vec::new();
                let mut timeout = None;
                while self.peek() != Some(&Token::RBrace) {
                    if self.peek() == Some(&Token::Timeout) {
                        self.next();
                        self.expect(Token::LParen);
                        let ms_expr = self.parse_expr();
                        self.expect(Token::RParen);
                        self.expect(Token::FatArrow);
                        let body = self.parse_block();
                        if self.peek() == Some(&Token::Comma) {
                            self.next();
                        }
                        timeout = Some((ms_expr, body));
                    } else {
                        let Some(Token::Ident(bind_var)) = self.next() else {
                            self.error("Expected variable name or `timeout` in `select`");
                        };
                        self.expect(Token::Assign);
                        let Some(Token::Ident(channel_var)) = self.next() else {
                            self.error("Expected channel variable after `=`");
                        };
                        self.expect(Token::Dot);
                        let Some(Token::Ident(m)) = self.next() else {
                            self.error("Expected `.recv()` in `select`");
                        };
                        if m != "recv" {
                            self.error("Expected `.recv()` in `select`");
                        }
                        self.expect(Token::LParen);
                        self.expect(Token::RParen);
                        self.expect(Token::FatArrow);
                        let body = self.parse_block();
                        if self.peek() == Some(&Token::Comma) {
                            self.next();
                        }
                        arms.push(SelectRecvArm {
                            bind_var,
                            channel_var,
                            body,
                        });
                    }
                }
                self.expect(Token::RBrace);
                Stmt::Select { arms, timeout }
            }
            Some(Token::Shared) => {
                self.next();
                let Some(Token::Ident(name)) = self.next() else {
                    self.error("Expected variable name after `shared`");
                };
                self.expect(Token::Assign);
                Stmt::Shared {
                    name,
                    value: self.parse_expr(),
                }
            }
            Some(Token::Lock) => {
                self.next();
                let Some(Token::Ident(var)) = self.next() else {
                    self.error("Expected variable name after `lock`");
                };
                Stmt::Lock {
                    var,
                    body: self.parse_block(),
                }
            }
            Some(Token::Concurrent) => {
                self.next();
                Stmt::Concurrent(self.parse_block())
            }
            Some(Token::Query) => self.parse_query_stmt(false),
            Some(Token::Spawn) => {
                self.next();
                let Some(Token::Ident(name)) = self.next() else {
                    self.error("Expected function name after `spawn`");
                };
                Stmt::Spawn {
                    name,
                    args: self.parse_args(),
                }
            }
            Some(Token::Parallel) => {
                self.next();
                if self.peek() == Some(&Token::Query) {
                    return self.parse_query_stmt(true);
                }
                if self.peek() != Some(&Token::For) {
                    self.error("Expected `for` after `parallel`");
                }
                let mut stmt = self.parse_stmt();
                if let Stmt::ForEach { is_parallel, .. } = &mut stmt {
                    *is_parallel = true;
                    stmt
                } else {
                    self.error("`parallel` is currently only supported on `for ... in list` loops")
                }
            }
            Some(Token::For) => {
                self.next(); // consume `for`
                let is_mut = if self.peek() == Some(&Token::Mut) {
                    self.next();
                    true
                } else {
                    false
                };
                let Some(Token::Ident(var)) = self.next() else {
                    self.error("Expected loop variable after `for` (e.g. `for i in 0..10`)");
                };
                self.expect(Token::In);
                let first = self.parse_expr();
                if self.peek() == Some(&Token::DotDot) {
                    self.next(); // `..`
                    let end = self.parse_expr();
                    let body = self.parse_block();
                    Stmt::ForRange {
                        var,
                        start: first,
                        end,
                        body,
                    }
                } else if let Expr::Var(coll_name) = first {
                    let body = self.parse_block();
                    Stmt::ForEach {
                        var,
                        is_mut,
                        is_parallel: false,
                        collection: coll_name,
                        body,
                    }
                } else {
                    self.error("Expected `start..end` or a list variable after `in`");
                }
            }
            Some(Token::Match) => {
                self.next(); // consume `match`
                let target = self.parse_expr();
                self.expect(Token::LBrace);
                let mut arms = Vec::new();
                while self.peek() != Some(&Token::RBrace) {
                    let pat = if self.peek() == Some(&Token::Underscore) {
                        self.next();
                        MatchPat::Wildcard
                    } else {
                        let e = self.parse_expr();
                        if let Expr::Call { name, args } = &e {
                            if let Some((en, vn)) = name.split_once("__") {
                                let bindings: Vec<String> = args
                                    .iter()
                                    .map(|a| match a {
                                        Expr::Var(b) => b.clone(),
                                        _ => self.error("Variable name expected in enum pattern"),
                                    })
                                    .collect();
                                MatchPat::EnumDestructure {
                                    enum_name: en.to_string(),
                                    variant: vn.to_string(),
                                    bindings,
                                }
                            } else {
                                MatchPat::Expr(e)
                            }
                        } else {
                            MatchPat::Expr(e)
                        }
                    };
                    self.expect(Token::FatArrow);
                    let body = self.parse_block();
                    if self.peek() == Some(&Token::Comma) {
                        self.next();
                    }
                    arms.push((pat, body));
                }
                self.expect(Token::RBrace);
                Stmt::Match { target, arms }
            }
            Some(Token::While) => {
                self.next();
                let cond = self.parse_expr();
                let body = self.parse_block();
                Stmt::While { cond, body }
            }
            Some(Token::Return) => {
                self.next();
                Stmt::Return(self.parse_expr())
            }
            Some(Token::Mut) if self.dialect == Dialect::Odin => {
                self.next();
                let Some(Token::Ident(name)) = self.next() else {
                    self.error("Expected variable name after `mut`");
                };
                let ty_hint = if self.peek() == Some(&Token::Colon) {
                    self.next();
                    let t = self.parse_type_name();
                    self.expect(Token::Assign);
                    Some(t)
                } else {
                    self.expect(Token::ColonAssign);
                    None
                };
                Stmt::Let {
                    name,
                    ty_hint,
                    is_mut: true,
                    value: self.parse_expr(),
                }
            }
            Some(Token::Ident(_)) => {
                let ident_span = self.current_span();
                let Some(Token::Ident(name)) = self.next() else {
                    unreachable!()
                };
                if self.dialect == Dialect::Odin && self.peek() == Some(&Token::ColonAssign) {
                    self.next();
                    return Stmt::Let {
                        name,
                        ty_hint: None,
                        is_mut: true,
                        value: self.parse_expr(),
                    };
                }
                if self.dialect == Dialect::Odin && self.peek() == Some(&Token::Colon) {
                    self.next();
                    let ty = self.parse_type_name();
                    if self.peek() == Some(&Token::Assign) {
                        self.next();
                        return Stmt::Let {
                            name,
                            ty_hint: Some(ty),
                            is_mut: true,
                            value: self.parse_expr(),
                        };
                    }
                    return Stmt::Declare {
                        name,
                        ty,
                        is_mut: true,
                    };
                }
                if self.peek() == Some(&Token::ColonColon) {
                    self.next(); // consume `::`
                    let Some(Token::Ident(member)) = self.next() else {
                        self.error("Expected name after `::`");
                    };
                    Stmt::Call {
                        name: format!("{}__{}", name, member),
                        args: self.parse_args(),
                    }
                } else if self.peek() == Some(&Token::LParen) {
                    let args = self.parse_args();
                    if self.peek() == Some(&Token::LBrace) {
                        let body = self.parse_block();
                        Stmt::BlockCall { name, args, body }
                    } else {
                        Stmt::Call { name, args }
                    }
                } else if self.peek() == Some(&Token::LBrace)
                    && name
                        .chars()
                        .next()
                        .map(|c| c.is_lowercase())
                        .unwrap_or(false)
                {
                    let body = self.parse_block();
                    Stmt::BlockCall {
                        name,
                        args: Vec::new(),
                        body,
                    }
                } else if self.peek() == Some(&Token::Dot) {
                    self.next(); // consume `.`
                    let Some(Token::Ident(member)) = self.next() else {
                        self.error("Expected field or method name after `.`");
                    };
                    if self.peek() == Some(&Token::ColonColon) {
                        self.next(); // consume `::`
                        let Some(Token::Ident(member)) = self.next() else {
                            self.error("Expected name after `::`");
                        };
                        Stmt::Call {
                            name: format!("{}__{}", name, member),
                            args: self.parse_args(),
                        }
                    } else if self.peek() == Some(&Token::LParen) {
                        let args = self.parse_args();
                        if self.peek() == Some(&Token::Question) {
                            self.next();
                            Stmt::Expr(Expr::Try(Box::new(Expr::MethodCall {
                                receiver: name,
                                method: member,
                                args,
                            })))
                        } else {
                            Stmt::MethodCall {
                                receiver: name,
                                method: member,
                                args,
                            }
                        }
                    } else {
                        match self.peek() {
                            Some(Token::PlusEq) => {
                                self.next();
                                Stmt::CompoundAssign {
                                    target: AssignTarget::Field {
                                        var: name,
                                        field: member,
                                    },
                                    op: Op::Add,
                                    value: self.parse_expr(),
                                }
                            }
                            Some(Token::MinusEq) => {
                                self.next();
                                Stmt::CompoundAssign {
                                    target: AssignTarget::Field {
                                        var: name,
                                        field: member,
                                    },
                                    op: Op::Sub,
                                    value: self.parse_expr(),
                                }
                            }
                            _ => {
                                self.expect(Token::Assign);
                                Stmt::FieldAssign {
                                    var: name,
                                    field: member,
                                    value: self.parse_expr(),
                                }
                            }
                        }
                    }
                } else if self.peek() == Some(&Token::LBracket) {
                    self.next(); // `[`
                    let idx = self.parse_expr();
                    self.expect(Token::RBracket);
                    match self.peek() {
                        Some(Token::PlusEq) => {
                            self.next();
                            Stmt::CompoundAssign {
                                target: AssignTarget::Index {
                                    var: name,
                                    index: idx,
                                },
                                op: Op::Add,
                                value: self.parse_expr(),
                            }
                        }
                        Some(Token::MinusEq) => {
                            self.next();
                            Stmt::CompoundAssign {
                                target: AssignTarget::Index {
                                    var: name,
                                    index: idx,
                                },
                                op: Op::Sub,
                                value: self.parse_expr(),
                            }
                        }
                        _ => {
                            self.expect(Token::Assign);
                            Stmt::IndexAssign {
                                var: name,
                                index: idx,
                                value: self.parse_expr(),
                            }
                        }
                    }
                } else if matches!(self.peek(), Some(Token::PlusEq | Token::MinusEq)) {
                    let op = if self.next() == Some(Token::PlusEq) {
                        Op::Add
                    } else {
                        Op::Sub
                    };
                    Stmt::CompoundAssign {
                        target: AssignTarget::Var(name),
                        op,
                        value: self.parse_expr(),
                    }
                } else if self.peek() == Some(&Token::Assign) {
                    self.next(); // `=`
                    Stmt::Assign {
                        name,
                        value: self.parse_expr(),
                    }
                } else {
                    panic!(
                        "SPAN:{}..{}:Unexpected identifier `{}` (missing `=` for assignment or `()` for call?)",
                        ident_span.start, ident_span.end, name
                    );
                }
            }
            t => panic!("Unexpected token: {:?}", t),
        }
    }

    fn peek_op(&mut self) -> Option<(Op, u8)> {
        match self.peek() {
            Some(Token::OrOr) => Some((Op::Or, 1)),
            Some(Token::AndAnd) => Some((Op::And, 2)),
            Some(Token::Pipe) => Some((Op::BitOr, 3)),
            Some(Token::Caret) => Some((Op::BitXor, 4)),
            Some(Token::Amp) => Some((Op::BitAnd, 5)),
            Some(Token::EqEq) => Some((Op::Eq, 6)),
            Some(Token::NotEq) => Some((Op::NotEq, 6)),
            Some(Token::Lt) => Some((Op::Lt, 7)),
            Some(Token::Gt) => Some((Op::Gt, 7)),
            Some(Token::LtEq) => Some((Op::LtEq, 7)),
            Some(Token::GtEq) => Some((Op::GtEq, 7)),
            Some(Token::Shl) => Some((Op::Shl, 8)),
            Some(Token::Shr) => Some((Op::Shr, 8)),
            Some(Token::Plus) => Some((Op::Add, 9)),
            Some(Token::Minus) => Some((Op::Sub, 9)),
            Some(Token::Star) => Some((Op::Mul, 10)),
            Some(Token::Slash) => Some((Op::Div, 10)),
            _ => None,
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_expr_bp(1)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Expr {
        let mut left = self.parse_primary();
        while let Some((op, bp)) = self.peek_op() {
            if bp < min_bp {
                break;
            }
            self.next();
            let right = self.parse_expr_bp(bp + 1);
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        left
    }

    fn parse_type_params(&mut self) -> Vec<String> {
        let mut params = Vec::new();
        if self.peek() == Some(&Token::Lt) {
            self.next(); // `<`
            while self.peek() != Some(&Token::Gt) {
                let Some(Token::Ident(tp)) = self.next() else {
                    self.error("Expected type parameter name in `<...>`");
                };
                params.push(tp);
                if self.peek() == Some(&Token::Comma) {
                    self.next();
                }
            }
            self.expect(Token::Gt);
        }
        params
    }

    fn parse_type_name(&mut self) -> String {
        let Some(Token::Ident(base)) = self.next() else {
            self.error("Expected type name");
        };
        if self.peek() == Some(&Token::Lt) {
            self.next(); // `<`
            let mut args = Vec::new();
            while self.peek() != Some(&Token::Gt) {
                args.push(self.parse_type_name());
                if self.peek() == Some(&Token::Comma) {
                    self.next();
                }
            }
            self.expect(Token::Gt);
            format!("{}<{}>", base, args.join(","))
        } else {
            base
        }
    }

    fn parse_args(&mut self) -> Vec<Expr> {
        self.expect(Token::LParen);
        let mut args = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            args.push(self.parse_expr());
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RParen);
        args
    }

    fn parse_primary(&mut self) -> Expr {
        let base = self.parse_atom();
        if self.peek() == Some(&Token::Question) {
            self.next(); // consume `?`
            Expr::Try(Box::new(base))
        } else {
            base
        }
    }

    fn parse_atom(&mut self) -> Expr {
        match self.next() {
            Some(Token::OkKw) => {
                self.expect(Token::LParen);
                let inner = self.parse_expr();
                self.expect(Token::RParen);
                Expr::Ok(Box::new(inner))
            }
            Some(Token::ErrKw) => {
                self.expect(Token::LParen);
                let inner = self.parse_expr();
                self.expect(Token::RParen);
                Expr::Err(Box::new(inner))
            }
            Some(Token::SomeKw) => {
                self.expect(Token::LParen);
                let inner = self.parse_expr();
                self.expect(Token::RParen);
                Expr::Some(Box::new(inner))
            }
            Some(Token::NoneKw) => Expr::None,
            Some(Token::True) => Expr::Int(1),
            Some(Token::False) => Expr::Int(0),
            Some(Token::Null) => Expr::Null,
            Some(Token::Bang) => {
                let inner = self.parse_primary();
                Expr::BinOp {
                    left: Box::new(inner),
                    op: Op::Eq,
                    right: Box::new(Expr::Int(0)),
                }
            }
            Some(Token::Int(v)) => Expr::Int(v),
            Some(Token::Hex(v)) => Expr::Hex(v),
            Some(Token::Float(v)) => Expr::Float(v),
            Some(Token::UnitFloat((v, u))) => Expr::UnitFloat(v, u),
            Some(Token::Minus) => {
                let inner = self.parse_primary();
                let zero = if matches!(inner, Expr::Float(_)) {
                    Expr::Float(0.0)
                } else {
                    Expr::Int(0)
                };
                Expr::BinOp {
                    left: Box::new(zero),
                    op: Op::Sub,
                    right: Box::new(inner),
                }
            }
            Some(Token::Str(raw)) => {
                let mut parts = Vec::new();
                let mut rest = raw.as_str();
                while let Some(open) = rest.find('{') {
                    if open > 0 {
                        parts.push(StringPart::Literal(rest[..open].to_string()));
                    }
                    let after_open = &rest[open + 1..];
                    let close = after_open.find('}').unwrap_or_else(|| {
                        self.error("Missing closing `}` in string interpolation")
                    });
                    let inner_code = &after_open[..close];
                    let inner_expr = Parser::new(inner_code).parse_expr();
                    parts.push(StringPart::Expr(inner_expr));
                    rest = &after_open[close + 1..];
                }
                if !rest.is_empty() || parts.is_empty() {
                    parts.push(StringPart::Literal(rest.to_string()));
                }
                Expr::StrInterp(parts)
            }
            Some(Token::Ident(n)) => {
                if self.peek() == Some(&Token::ColonColon) {
                    self.next(); // consume `::`
                    let Some(Token::Ident(member)) = self.next() else {
                        self.error("Expected name after `::`");
                    };
                    if self.peek() == Some(&Token::LParen) {
                        Expr::Call {
                            name: format!("{}__{}", n, member),
                            args: self.parse_args(),
                        }
                    } else {
                        Expr::PathConst { ty: n, member }
                    }
                } else if self.peek() == Some(&Token::LParen) {
                    Expr::Call {
                        name: n,
                        args: self.parse_args(),
                    }
                } else if self.peek() == Some(&Token::Dot) {
                    self.next(); // `.`
                    if let Some(Token::Int(idx)) = self.peek().cloned() {
                        self.next();
                        Expr::TupleIndex {
                            tuple: Box::new(Expr::Var(n)),
                            idx: idx as usize,
                        }
                    } else {
                        let member = match self.next() {
                            Some(Token::Ident(m)) => m,
                            Some(Token::Spawn) => "spawn".to_string(),
                            _ => panic!("Member name expected after `.`"),
                        };
                        if self.peek() == Some(&Token::LParen) {
                            Expr::MethodCall {
                                receiver: n,
                                method: member,
                                args: self.parse_args(),
                            }
                        } else {
                            Expr::FieldAccess {
                                var: n,
                                field: member,
                            }
                        }
                    }
                } else if self.peek() == Some(&Token::LBracket) {
                    self.next(); // `[`
                    let idx = self.parse_expr();
                    self.expect(Token::RBracket);
                    Expr::Index {
                        var: n,
                        index: Box::new(idx),
                    }
                } else if self.peek() == Some(&Token::LBrace)
                    && n.chars().next().unwrap().is_uppercase()
                {
                    // Großbuchstabe + `{` -> Struct-Initialisierung wie `Player { hp: 100, x: 10 }`!
                    // (Durch die Großbuchstaben-Regel verwechselt er es nie mit `if p > 0 { ... }`!)
                    self.next(); // `{`
                    let mut fields = Vec::new();
                    while self.peek() != Some(&Token::RBrace) {
                        let Some(Token::Ident(f_name)) = self.next() else {
                            panic!("Field name expected")
                        };
                        self.expect(Token::Colon);
                        let val = self.parse_expr();
                        fields.push((f_name, val));
                        if self.peek() == Some(&Token::Comma) {
                            self.next();
                        }
                    }
                    self.expect(Token::RBrace);
                    Expr::StructInit { name: n, fields }
                } else {
                    Expr::Var(n)
                }
            }
            Some(Token::LBracket) => {
                let mut items = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    items.push(self.parse_expr());
                    if self.peek() == Some(&Token::Comma) {
                        self.next();
                    }
                }
                self.expect(Token::RBracket);
                Expr::VecLit(items)
            }
            Some(Token::LParen) => {
                let first = self.parse_expr();
                if self.peek() == Some(&Token::Comma) {
                    let mut items = vec![first];
                    while self.peek() == Some(&Token::Comma) {
                        self.next(); // `,`
                        if self.peek() == Some(&Token::RParen) {
                            break;
                        }
                        items.push(self.parse_expr());
                    }
                    self.expect(Token::RParen);
                    Expr::Tuple(items)
                } else {
                    self.expect(Token::RParen);
                    first
                }
            }
            t => panic!("Expected expression, found: {:?}", t),
        }
    }
}

pub fn check_diagnostics(src: &str) -> Vec<ParseError> {
    check_diagnostics_with_dialect(src, Dialect::Rust)
}

pub fn check_diagnostics_with_dialect(src: &str, dialect: Dialect) -> Vec<ParseError> {
    let mut errors = Vec::new();

    // 1. Check lexer errors (unknown characters like `@` or `$`)
    for (res, span) in Token::lexer(src).spanned() {
        if res.is_err() {
            errors.push(ParseError {
                message: format!("Unknown character: `{}`", &src[span.clone()]),
                span,
            });
        }
    }
    if !errors.is_empty() {
        return errors;
    }

    // 2. Catch parser errors & extract span
    let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Parser::new_with_dialect(src, dialect).parse_program()
    }));

    match parsed {
        Err(payload) => {
            let msg = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Syntax error".to_string());

            if let Some(rest) = msg.strip_prefix("SPAN:") {
                let parts: Vec<&str> = rest.splitn(2, ':').collect();
                let nums: Vec<&str> = parts[0].split("..").collect();
                if let (Ok(start), Ok(end)) = (nums[0].parse(), nums[1].parse()) {
                    errors.push(ParseError {
                        message: parts.get(1).unwrap_or(&"Syntax error").to_string(),
                        span: start..end,
                    });
                    return errors;
                }
            }

            errors.push(ParseError {
                message: msg,
                span: 0..src.len().min(1),
            });
        }
        Ok(prog) => {
            // 3. Semantic check (e.g. assignment without `mut`)
            check_mutability(src, &prog, &mut errors);
        }
    }

    errors
}

fn check_mutability(src: &str, prog: &Program, errors: &mut Vec<ParseError>) {
    for func in &prog.functions {
        let mut mut_vars = std::collections::HashMap::new();
        for (name, _, is_mut) in &func.params {
            mut_vars.insert(name.clone(), *is_mut);
        }
        check_stmts_mut(src, &func.body, &mut mut_vars, errors);
    }
}

fn check_stmts_mut(
    src: &str,
    stmts: &[Stmt],
    mut_vars: &mut std::collections::HashMap<String, bool>,
    errors: &mut Vec<ParseError>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, is_mut, .. } | Stmt::Declare { name, is_mut, .. } => {
                mut_vars.insert(name.clone(), *is_mut);
            }
            Stmt::Shared { name, .. } => {
                mut_vars.insert(name.clone(), false);
            }
            Stmt::Lock { var, body } => {
                let prev = mut_vars.insert(var.clone(), true);
                check_stmts_mut(src, body, mut_vars, errors);
                if let Some(p) = prev {
                    mut_vars.insert(var.clone(), p);
                } else {
                    mut_vars.remove(var);
                }
            }
            Stmt::Match { target, arms } => {
                let has_wildcard = arms.iter().any(|(p, _)| matches!(p, MatchPat::Wildcard));
                if !has_wildcard {
                    let mut enum_name: Option<String> = None;
                    for (p, _) in arms {
                        match p {
                            MatchPat::EnumDestructure { enum_name: en, .. } => {
                                enum_name = Some(en.clone());
                                break;
                            }
                            MatchPat::Expr(Expr::PathConst { ty, .. }) => {
                                enum_name = Some(ty.clone());
                                break;
                            }
                            _ => {}
                        }
                    }
                    if let Some(en) = enum_name {
                        let covered: std::collections::HashSet<String> = arms
                            .iter()
                            .filter_map(|(p, _)| match p {
                                MatchPat::EnumDestructure { variant, .. } => Some(variant.clone()),
                                MatchPat::Expr(Expr::PathConst { member, .. }) => {
                                    Some(member.clone())
                                }
                                _ => None,
                            })
                            .collect();
                        // Check if any variant is missing via `prog.enums`? We can pass `enums` or check in `check_diagnostics`
                        let _ = (target, en, covered);
                    }
                }
                for (pat, body) in arms {
                    if let MatchPat::EnumDestructure { bindings, .. } = pat {
                        for b in bindings {
                            mut_vars.insert(b.clone(), false);
                        }
                    }
                    check_stmts_mut(src, body, mut_vars, errors);
                }
            }
            Stmt::Select { arms, timeout } => {
                for arm in arms {
                    mut_vars.insert(arm.bind_var.clone(), false);
                    check_stmts_mut(src, &arm.body, mut_vars, errors);
                }
                if let Some((_, t_body)) = timeout {
                    check_stmts_mut(src, t_body, mut_vars, errors);
                }
            }
            Stmt::ForEach {
                var, is_mut, body, ..
            } => {
                mut_vars.insert(var.clone(), *is_mut);
                check_stmts_mut(src, body, mut_vars, errors);
            }
            Stmt::Assign { name, .. } | Stmt::FieldAssign { var: name, .. } => {
                match mut_vars.get(name) {
                    Some(false) => {
                        let pos = src.rfind(name).unwrap_or(0);
                        errors.push(ParseError {
                            message: format!(
                                "Variable `{}` is not `mut` and cannot be reassigned",
                                name
                            ),
                            span: pos..(pos + name.len()),
                        });
                    }
                    None => {
                        let pos = src.rfind(name).unwrap_or(0);
                        errors.push(ParseError {
                            message: format!("Unknown variable `{}`", name),
                            span: pos..(pos + name.len()),
                        });
                    }
                    _ => {}
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                check_stmts_mut(src, then_block, mut_vars, errors);
                check_stmts_mut(src, else_block, mut_vars, errors);
            }
            Stmt::While { body, .. } => {
                check_stmts_mut(src, body, mut_vars, errors);
            }
            Stmt::Match { arms, .. } => {
                for (_, body) in arms {
                    check_stmts_mut(src, body, mut_vars, errors);
                }
            }
            Stmt::ForRange { var, body, .. } => {
                mut_vars.insert(var.clone(), false);
                check_stmts_mut(src, body, mut_vars, errors);
            }
            _ => {}
        }
    }
}

pub fn is_builtin_container_type(ty: &str) -> bool {
    matches!(
        ty,
        "String"
            | "Bytes"
            | "Vec"
            | "Deque"
            | "Map"
            | "StrMap"
            | "BTree"
            | "Set"
            | "Heap"
            | "Grid"
            | "BitSet"
            | "Channel"
            | "Atomic"
            | "Ecs"
    ) || ty.starts_with("Vec<")
}

/// Verbietet `= null` auf eingebauten Heap-Containern (nur externe Handles dürfen `null` sein!)
pub fn check_container_nulls(prog: &Program) -> Result<(), String> {
    for f in &prog.functions {
        let mut container_vars = std::collections::HashSet::new();
        for (p_name, p_ty, _) in &f.params {
            if is_builtin_container_type(p_ty) {
                container_vars.insert(p_name.clone());
            }
        }
        check_nulls_in_block(&f.body, &mut container_vars)?;
    }
    for (_t_name, t_body) in &prog.tests {
        let mut container_vars = std::collections::HashSet::new();
        check_nulls_in_block(t_body, &mut container_vars)?;
    }
    Ok(())
}

fn check_nulls_in_block(
    stmts: &[Stmt],
    containers: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    for s in stmts {
        match s {
            Stmt::Declare { name, ty, .. } => {
                if is_builtin_container_type(ty) {
                    containers.insert(name.clone());
                }
            }
            Stmt::Let {
                name,
                ty_hint,
                value,
                ..
            } => {
                let is_cont = ty_hint.as_deref().is_some_and(is_builtin_container_type)
                    || matches!(value, Expr::StrInterp(_) | Expr::VecLit(_))
                    || matches!(value, Expr::Call { name: cn, .. } if cn.starts_with("Bytes::") || cn.starts_with("Map::") || cn.starts_with("Ecs::") || cn.starts_with("Deque::"));
                if is_cont {
                    if matches!(value, Expr::Null) {
                        return Err(format!(
                            "Null-Fehler: Container '{}' darf nicht mit 'null' initialisiert werden!",
                            name
                        ));
                    }
                    containers.insert(name.clone());
                }
            }
            Stmt::Assign { name, value } => {
                if containers.contains(name) && matches!(value, Expr::Null) {
                    return Err(format!(
                        "Null-Fehler: Eingebauter Container '{}' darf nicht auf 'null' gesetzt werden!",
                        name
                    ));
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                check_nulls_in_block(then_block, &mut containers.clone())?;
                check_nulls_in_block(else_block, &mut containers.clone())?;
            }
            Stmt::While { body, .. }
            | Stmt::ForRange { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Query { body, .. }
            | Stmt::Lock { body, .. }
            | Stmt::Concurrent(body) => {
                check_nulls_in_block(body, &mut containers.clone())?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Prüft für Smart Auto-Move / Auto-Clone, ob `target` in `stmts` noch irgendwo gelesen wird
pub fn stmts_read_var(stmts: &[Stmt], target: &str) -> bool {
    stmts.iter().any(|s| stmt_reads_var(s, target))
}

pub fn stmt_reads_var(stmt: &Stmt, target: &str) -> bool {
    match stmt {
        Stmt::Let { name, value, .. } => name != target && expr_reads_var(value, target),
        Stmt::Assign { value, .. } => expr_reads_var(value, target),
        Stmt::CompoundAssign {
            target: ca_target,
            value,
            ..
        } => {
            matches!(
                ca_target,
                AssignTarget::Var(n)
                    | AssignTarget::Field { var: n, .. }
                    | AssignTarget::Index { var: n, .. }
                if n == target
            ) || expr_reads_var(value, target)
        }
        Stmt::FieldAssign { var, value, .. } => var == target || expr_reads_var(value, target),
        Stmt::Call { args, .. } | Stmt::Spawn { args, .. } => {
            args.iter().any(|a| expr_reads_var(a, target))
        }
        Stmt::MethodCall { receiver, args, .. } => {
            receiver == target || args.iter().any(|a| expr_reads_var(a, target))
        }
        Stmt::Return(e) | Stmt::Print(e) | Stmt::Expr(e) => expr_reads_var(e, target),
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            expr_reads_var(cond, target)
                || stmts_read_var(then_block, target)
                || stmts_read_var(else_block, target)
        }
        Stmt::While { cond, body } => expr_reads_var(cond, target) || stmts_read_var(body, target),
        Stmt::ForRange {
            start, end, body, ..
        } => {
            expr_reads_var(start, target)
                || expr_reads_var(end, target)
                || stmts_read_var(body, target)
        }
        Stmt::ForEach {
            collection, body, ..
        } => collection == target || stmts_read_var(body, target),
        Stmt::Query { ecs_var, body, .. } => ecs_var == target || stmts_read_var(body, target),
        Stmt::Lock { var, body } => var == target || stmts_read_var(body, target),
        Stmt::Match { target: m_expr, arms } => {
            expr_reads_var(m_expr, target) || arms.iter().any(|(_, body)| stmts_read_var(body, target))
        }
        Stmt::Select { arms, timeout } => {
            arms.iter()
                .any(|a| a.channel_var == target || stmts_read_var(&a.body, target))
                || timeout
                    .as_ref()
                    .is_some_and(|(ms, b)| expr_reads_var(ms, target) || stmts_read_var(b, target))
        }
        Stmt::Defer(inner) => stmt_reads_var(inner, target),
        Stmt::Concurrent(b) => stmts_read_var(b, target),
        _ => false,
    }
}

fn expr_reads_var(expr: &Expr, target: &str) -> bool {
    match expr {
        Expr::Var(n) => n == target,
        Expr::BinOp { left, right, .. } => {
            expr_reads_var(left, target) || expr_reads_var(right, target)
        }
        Expr::Try(inner) => expr_reads_var(inner, target),
        Expr::Call { args, .. } | Expr::VecLit(args) | Expr::Tuple(args) => {
            args.iter().any(|a| expr_reads_var(a, target))
        }
        Expr::MethodCall { receiver, args, .. } => {
            receiver == target || args.iter().any(|a| expr_reads_var(a, target))
        }
        Expr::FieldAccess { var, .. } => var == target,
        Expr::Index { var, index } => var == target || expr_reads_var(index, target),
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, e)| expr_reads_var(e, target)),
        _ => false,
    }
}

fn find_project_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut dir = if start.is_dir() {
        Some(start)
    } else {
        start.parent()
    };
    while let Some(d) = dir {
        if d.join("striv.toml").exists() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

pub fn resolve_import_path(
    current_dir: &std::path::Path,
    imp: &str,
) -> Result<std::path::PathBuf, String> {
    let rel = imp.trim_end_matches(".strv").replace("::", "/");
    let mut candidates = vec![
        current_dir.join(format!("{}.strv", rel)),
        current_dir.join(&rel).join("mod.strv"),
        current_dir.join(&rel).join("lib.strv"),
    ];

    if let Some(root) = find_project_root(current_dir) {
        candidates.push(root.join("src").join(format!("{}.strv", rel)));
        candidates.push(root.join("src").join(&rel).join("mod.strv"));

        let mut parts = rel.splitn(2, '/');
        let pkg = parts.next().unwrap_or(&rel);
        let pkg_hyphen = pkg.replace('_', "-");
        for folder in [pkg, pkg_hyphen.as_str()] {
            let pkg_dir = root.join(".striv").join("pkgs").join(folder);
            if let Some(sub) = parts.next() {
                candidates.push(pkg_dir.join("src").join(format!("{}.strv", sub)));
                candidates.push(pkg_dir.join(format!("{}.strv", sub)));
            } else {
                candidates.push(pkg_dir.join("src").join("lib.strv"));
                candidates.push(pkg_dir.join("src").join("main.strv"));
                candidates.push(pkg_dir.join("lib.strv"));
                candidates.push(pkg_dir.join("mod.strv"));
            }
        }
    }

    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }
    Err(format!(
        "Modul '{}' konnte nicht gefunden werden (gesucht ab '{}')",
        imp,
        current_dir.display()
    ))
}

pub fn load_program_with_imports(entry_path: &std::path::Path) -> Result<Program, String> {
    let mut visited = std::collections::HashSet::new();
    let mut combined = Program {
        imports: Vec::new(),
        tweaks: Vec::new(),
        consts: Vec::new(),
        units: Vec::new(),
        rust_blocks: Vec::new(),
        extern_fns: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        tests: Vec::new(),
    };

    load_recursive(entry_path, &mut visited, &mut combined)?;
    check_container_nulls(&combined)?;
    units::check_and_erase_units(&mut combined).map_err(|e| e)?;
    merge_open_enums(&mut combined);
    expand_rpc_functions(&mut combined);
    monomorphize::monomorphize_program(&mut combined);
    Ok(combined)
}

fn load_recursive(
    path: &std::path::Path,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
    out: &mut Program,
) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|_| format!("Module file not found: '{}'", path.display()))?;
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }

    let src = std::fs::read_to_string(&canonical)
        .map_err(|e| format!("Could not read '{}': {}", path.display(), e))?;
    let dialect = detect_dialect_for_path(&canonical);
    let prog = Parser::new_with_dialect(&src, dialect).parse_program();

    let dir = canonical
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("strv");

    for imp in &prog.imports {
        if let Ok(dep_path) = resolve_import_path(dir, imp) {
            load_recursive(&dep_path, visited, out)?;
        } else {
            let dep_path = dir.join(format!("{}.{}", imp, ext));
            if dep_path.exists() {
                load_recursive(&dep_path, visited, out)?;
            }
        }
    }

    out.tweaks.extend(prog.tweaks);
    out.consts.extend(prog.consts);
    out.units.extend(prog.units);
    out.rust_blocks.extend(prog.rust_blocks);
    out.extern_fns.extend(prog.extern_fns);
    out.enums.extend(prog.enums);
    out.structs.extend(prog.structs);
    out.functions.extend(prog.functions);
    out.tests.extend(prog.tests);
    Ok(())
}

pub fn fnv1a_32(name: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for b in name.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

pub fn expand_rpc_functions(prog: &mut Program) {
    let struct_names: std::collections::HashSet<String> =
        prog.structs.iter().map(|s| s.name.clone()).collect();

    let rpc_fns: Vec<Function> = prog.functions.iter().filter(|f| f.is_rpc).cloned().collect();
    if rpc_fns.is_empty() {
        return;
    }

    let mut dispatch_arms: Vec<(Option<(String, bool)>, u32, String, Vec<(String, String)>)> = Vec::new();

    for rf in &rpc_fns {
        let hash = fnv1a_32(&rf.name);
        let (ctx_param, net_params_slice) = match rf.params.first() {
            Some((_, ty, is_mut)) if struct_names.contains(ty) => {
                (Some((ty.clone(), *is_mut)), &rf.params[1..])
            }
            _ => (None, &rf.params[..]),
        };

        let mut pack_body = vec![
            Stmt::Let {
                name: "__pkt".into(),
                ty_hint: Some("Bytes".into()),
                is_mut: true,
                value: Expr::Call { name: "Bytes__new".into(), args: vec![] },
            },
            Stmt::MethodCall {
                receiver: "__pkt".into(),
                method: "write_u32".into(),
                args: vec![Expr::Int(hash as i64)],
            },
        ];

        let mut net_meta = Vec::new();
        for (p_name, p_ty, _) in net_params_slice {
            let write_method = if p_ty == "float" || p_ty == "f32" {
                "write_f64"
            } else {
                "write_i64"
            };
            pack_body.push(Stmt::MethodCall {
                receiver: "__pkt".into(),
                method: write_method.into(),
                args: vec![Expr::Var(p_name.clone())],
            });
            net_meta.push((p_name.clone(), p_ty.clone()));
        }
        pack_body.push(Stmt::Return(Expr::Var("__pkt".into())));

        prog.functions.push(Function {
            name: format!("{}_pack", rf.name),
            is_rpc: false,
        rust_body: None,
            type_params: vec![],
            params: net_params_slice.to_vec(),
            ret_type: Some("Bytes".into()),
            body: pack_body,
        });

        dispatch_arms.push((ctx_param, hash, rf.name.clone(), net_meta));
    }

    let first_ctx = dispatch_arms[0].0.clone();
    let mut disp_params = Vec::new();
    if let Some((ctx_ty, ctx_mut)) = &first_ctx {
        disp_params.push(("__ctx".into(), ctx_ty.clone(), *ctx_mut));
    }
    disp_params.push(("__pkt".into(), "Bytes".into(), false));

    let mut disp_body = vec![
        Stmt::If {
            cond: Expr::BinOp {
                left: Box::new(Expr::MethodCall {
                    receiver: "__pkt".into(),
                    method: "len".into(),
                    args: vec![],
                }),
                op: Op::Lt,
                right: Box::new(Expr::Int(4)),
            },
            then_block: vec![Stmt::Return(Expr::Int(0))],
            else_block: vec![],
        },
        Stmt::Let {
            name: "__rpc_id".into(),
            ty_hint: None,
            is_mut: false,
            value: Expr::MethodCall {
                receiver: "__pkt".into(),
                method: "read_u32".into(),
                args: vec![Expr::Int(0)],
            },
        },
    ];

    for (ctx_info, hash, target_fn, net_meta) in dispatch_arms {
        let needed_bytes = 4 + (net_meta.len() as i64) * 8;
        let mut arm_stmts = vec![Stmt::If {
            cond: Expr::BinOp {
                left: Box::new(Expr::MethodCall {
                    receiver: "__pkt".into(),
                    method: "len".into(),
                    args: vec![],
                }),
                op: Op::Lt,
                right: Box::new(Expr::Int(needed_bytes)),
            },
            then_block: vec![Stmt::Return(Expr::Int(0))],
            else_block: vec![],
        }];

        let mut call_args = Vec::new();
        if ctx_info.is_some() {
            call_args.push(Expr::Var("__ctx".into()));
        }

        let mut offset = 4i64;
        for (i, (_p_name, p_ty)) in net_meta.iter().enumerate() {
            let tmp_name = format!("__rpc_arg_{}", i);
            let read_m = if p_ty == "float" || p_ty == "f32" { "read_f64" } else { "read_i64" };
            arm_stmts.push(Stmt::Let {
                name: tmp_name.clone(),
                ty_hint: None,
                is_mut: false,
                value: Expr::MethodCall {
                    receiver: "__pkt".into(),
                    method: read_m.into(),
                    args: vec![Expr::Int(offset)],
                },
            });
            call_args.push(Expr::Var(tmp_name));
            offset += 8;
        }

        arm_stmts.push(Stmt::Call { name: target_fn, args: call_args });
        arm_stmts.push(Stmt::Return(Expr::Int(1)));

        disp_body.push(Stmt::If {
            cond: Expr::BinOp {
                left: Box::new(Expr::Var("__rpc_id".into())),
                op: Op::Eq,
                right: Box::new(Expr::Int(hash as i64)),
            },
            then_block: arm_stmts,
            else_block: vec![],
        });
    }

    disp_body.push(Stmt::Return(Expr::Int(0)));

    prog.functions.push(Function {
        name: "rpc_dispatch".into(),
        is_rpc: false,
        rust_body: None,
        type_params: vec![],
        params: disp_params,
        ret_type: Some("int".into()),
        body: disp_body,
    });
}

pub fn fnv1a_64(s: &str) -> i64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    ((hash & 0x7FFF_FFFF_FFFF_FFFF).max(1)) as i64
}

pub fn merge_open_enums(prog: &mut Program) {
    let mut extensions = Vec::new();
    let mut base_enums = Vec::new();

    for e in prog.enums.drain(..) {
        if e.is_extend {
            extensions.push(e);
        } else {
            base_enums.push(e);
        }
    }

    for ext in extensions {
        let target = base_enums
            .iter_mut()
            .find(|b| b.name == ext.name)
            .unwrap_or_else(|| {
                panic!(
                    "`extend enum {}` gefunden, aber Basis-Enum fehlt!",
                    ext.name
                )
            });
        assert!(
            target.is_open,
            "Fehler: `enum {}` ist nicht als `open enum` markiert und darf nicht erweitert werden!",
            ext.name
        );
        for v in ext.variants {
            if !target.variants.iter().any(|existing| existing.name == v.name) {
                target.variants.push(v);
            }
        }
    }

    prog.enums = base_enums;
}

#[cfg(test)]
mod dialect_tests {
    use super::*;

    #[test]
    fn test_rust_to_odin_roundtrip() {
        let rust_src = r#"
const LIMIT = 42

enum State {
    Idle,
    Active,
}

struct Ship {
    hp: int,
}

fn calc(a: int, b: int) -> int {
    let mut sum = a + b
    let doubled : int = sum * 2
    return doubled
}
"#;
        let prog_rust = Parser::new(rust_src).parse_program();
        let odin_src = format_program_with_dialect(&prog_rust, Dialect::Odin);
        assert!(odin_src.contains("LIMIT :: 42"));
        assert!(odin_src.contains("Ship :: struct {"));
        assert!(odin_src.contains("calc :: proc(a: int, b: int) -> int {"));
        assert!(odin_src.contains("sum := a + b"));
        assert!(odin_src.contains("doubled : int = sum * 2"));

        let prog_odin = Parser::new_with_dialect(&odin_src, Dialect::Odin).parse_program();
        assert_eq!(prog_odin.consts.len(), 1);
        assert_eq!(prog_odin.enums.len(), 1);
        assert_eq!(prog_odin.structs.len(), 1);
        assert_eq!(prog_odin.functions.len(), 1);

        let allman_src = format_program_with_options(&prog_rust, Dialect::Rust, BraceStyle::Allman);
        assert!(allman_src.contains("fn calc(a: int, b: int) -> int\n{\n"));
        assert!(allman_src.contains("struct Ship\n{\n"));
    }

    #[test]
    fn test_check_container_nulls() {
        let bad_odin = r#"
main :: proc() {
    buf: Bytes
    buf = null
}
"#;
        let bad_prog = Parser::new_with_dialect(bad_odin, Dialect::Odin).parse_program();
        assert!(check_container_nulls(&bad_prog).is_err());

        let good_odin = r#"
main :: proc() {
    tex: TextureHandle
    tex = null
}
"#;
        let good_prog = Parser::new_with_dialect(good_odin, Dialect::Odin).parse_program();
        assert!(check_container_nulls(&good_prog).is_ok());
    }

    #[test]
    fn test_nested_and_package_imports() {
        let tmp = std::env::temp_dir().join("striv_test_nested_imports");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src").join("physics")).unwrap();
        std::fs::create_dir_all(tmp.join(".striv").join("pkgs").join("kepler").join("src")).unwrap();

        std::fs::write(
            tmp.join("striv.toml"),
            "[package]\nname = \"space\"\nsyntax = \"odin\"\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join("src").join("physics").join("orbit.strv"),
            "orbit_step :: proc() -> int { return 42 }\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join(".striv").join("pkgs").join("kepler").join("src").join("lib.strv"),
            "kepler_const :: proc() -> int { return 7 }\n",
        )
        .unwrap();

        let main_odin = tmp.join("src").join("main_odin.strv");
        std::fs::write(
            &main_odin,
            "import \"physics/orbit\"\nimport \"kepler\"\nmain :: proc() -> int { return orbit_step() + kepler_const() }\n",
        )
        .unwrap();
        let prog_odin = load_program_with_imports(&main_odin).unwrap();
        assert!(prog_odin.functions.iter().any(|f| f.name == "orbit_step"));
        assert!(prog_odin.functions.iter().any(|f| f.name == "kepler_const"));

        let main_rust = tmp.join("src").join("main_rust.strv");
        std::fs::write(
            &main_rust,
            "use physics::orbit\nuse kepler\nfn main() -> int { return 0 }\n",
        )
        .unwrap();
        let prog_rust = load_program_with_imports(&main_rust).unwrap();
        assert!(prog_rust.functions.iter().any(|f| f.name == "orbit_step"));
        assert!(prog_rust.functions.iter().any(|f| f.name == "kepler_const"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_fmt_preserves_units_rust_blocks_and_query() {
        let src = r#"
unit m
unit s
unit kg
unit Velocity = m / s
unit Accel = m / (s * s)
unit Force = kg * Accel

rust { pub struct Foo { pub x: i64 } }

fn step(mut ecs: Ecs) {
    query (id: entity, mut orb: Orbit, !Docked) in ecs {
        orb.alt += 1.0
    }
}
"#;
        let prog = Parser::new(src).parse_program();
        let formatted = format_program_with_dialect(&prog, Dialect::Rust);
        assert!(formatted.contains("unit m\n"));
        assert!(formatted.contains("unit Velocity = m / s\n"));
        assert!(formatted.contains("unit Accel = m / (s * s)\n"));
        assert!(formatted.contains("unit Force = kg * Accel\n"));
        assert!(formatted.contains("rust { pub struct Foo { pub x: i64 } }"));
        assert!(formatted.contains("query (id: entity, mut orb: Orbit, !Docked) in ecs {"));
    }
}
