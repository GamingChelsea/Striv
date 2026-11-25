use logos::Logos;

pub mod fmt;
pub mod monomorphize;
pub mod release_gen;
pub use fmt::{format_expr, format_program, format_source};

// AST

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
pub struct Program {
    pub imports: Vec<String>,
    pub tweaks: Vec<(String, TweakVal)>,
    pub consts: Vec<(String, Expr)>,
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
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub is_rpc: bool,
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
    Float(f64),
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
    #[token("const")]
    Const,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("enum")]
    Enum,
    #[token("match")]
    Match,
    #[token("_", priority = 3)]
    Underscore,
    #[token("use")]
    Use,
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

    #[regex("[0-9]+\\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),
    #[regex("0x[0-9a-fA-F]+", |lex| i64::from_str_radix(&lex.slice()[2..], 16).ok())]
    Hex(i64),
    #[regex("[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
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
            Token::Const => "`const`".to_string(),
            Token::True => "`true`".to_string(),
            Token::False => "`false`".to_string(),
            Token::Enum => "`enum`".to_string(),
            Token::Match => "`match`".to_string(),
            Token::Underscore => "`_`".to_string(),
            Token::FatArrow => "`=>`".to_string(),
            Token::Use => "`use`".to_string(),
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
    src_len: usize,
    pos: usize,
}

impl Parser {
    pub fn new(src: &str) -> Self {
        let tokens = Token::lexer(src)
            .spanned()
            .filter_map(|(res, span)| res.ok().map(|tok| (tok, span)))
            .collect();
        Self {
            tokens,
            src_len: src.len(),
            pos: 0,
        }
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
                    if self.peek() == Some(&Token::ColonColon) {
                        self.next(); // `::`
                        if self.peek() == Some(&Token::LBrace) {
                            self.next(); // `{`
                            while self.peek().is_some() && self.peek() != Some(&Token::RBrace) {
                                self.next();
                            }
                            self.expect(Token::RBrace);
                        } else {
                            self.next(); // single symbol like `Player`
                        }
                    }
                    imports.push(mod_name);
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
                    self.expect(Token::Assign);
                    let expr = self.parse_expr();
                    consts.push((c_name, expr));
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
            enums,
            structs,
            functions,
            tests,
        }
    }

    fn parse_enum(&mut self) -> EnumDef {
        self.expect(Token::Enum);
        let Some(Token::Ident(name)) = self.next() else {
            self.error("Expected enum name after `enum`");
        };
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
        EnumDef { name, variants }
    }

    fn parse_struct(&mut self) -> StructDef {
        self.expect(Token::Struct);
        let Some(Token::Ident(name)) = self.next() else {
            panic!("Struct name expected")
        };
        let type_params = self.parse_type_params();
        self.expect(Token::LBrace);
        let mut fields = Vec::new();
        let mut sync_mask = 0u64;
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
            type_params: Vec::new(),
            params,
            ret_type,
            body: self.parse_block(),
        }
    }

    pub fn parse_function(&mut self) -> Function {
        self.expect(Token::Fn);
        let Some(Token::Ident(name)) = self.next() else {
            panic!("Function Name expected")
        };
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
            type_params,
            params,
            ret_type,
            body: self.parse_block(),
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
            Some(Token::Ident(_)) => {
                let ident_span = self.current_span();
                let Some(Token::Ident(name)) = self.next() else {
                    unreachable!()
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
            Some(Token::Int(v)) => Expr::Int(v),
            Some(Token::Hex(v)) => Expr::Hex(v),
            Some(Token::Float(v)) => Expr::Float(v),
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
                        let Some(Token::Ident(member)) = self.next() else {
                            panic!("Member name expected after `.`")
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
        Parser::new(src).parse_program()
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
            Stmt::Let { name, is_mut, .. } => {
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

pub fn load_program_with_imports(entry_path: &std::path::Path) -> Result<Program, String> {
    let mut visited = std::collections::HashSet::new();
    let mut combined = Program {
        imports: Vec::new(),
        tweaks: Vec::new(),
        consts: Vec::new(),
        enums: Vec::new(),
        structs: Vec::new(),
        functions: Vec::new(),
        tests: Vec::new(),
    };

    load_recursive(entry_path, &mut visited, &mut combined)?;
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
    let prog = Parser::new(&src).parse_program();

    let dir = canonical
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("strv");

    for imp in &prog.imports {
        let dep_path = dir.join(format!("{}.{}", imp, ext));
        if dep_path.exists() {
            load_recursive(&dep_path, visited, out)?;
        }
    }

    out.tweaks.extend(prog.tweaks);
    out.consts.extend(prog.consts);
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
        type_params: vec![],
        params: disp_params,
        ret_type: Some("int".into()),
        body: disp_body,
    });
}
