use logos::Logos;

pub mod fmt;
pub use fmt::format_program;

// AST

#[derive(Debug, Clone)]
pub enum TweakVal {
    Int(i64),
    Str(String),
    Vec(Vec<i64>),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub tweaks: Vec<(String, TweakVal)>,
    pub structs: Vec<StructDef>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<String>, // Field names in order (each field = 8 bytes)
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, String, bool)>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        is_mut: bool,
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
    }, // e.g. p.hp = 80
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
    StrInterp(Vec<StringPart>),
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
    Lt,
    Gt,
    Eq,
}

// Lexer
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
pub enum Token {
    #[token("fn")]
    Fn,
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
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token("->")]
    Arrow,
    #[token("==")]
    EqEq,
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,

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
            Token::Tweak => "`tweak`".to_string(),
            Token::Let => "`let`".to_string(),
            Token::Mut => "`mut`".to_string(),
            Token::Struct => "`struct`".to_string(),
            Token::Impl => "`impl`".to_string(),
            Token::If => "`if`".to_string(),
            Token::Else => "`else`".to_string(),
            Token::While => "`while`".to_string(),
            Token::Return => "`return`".to_string(),
            Token::Print => "`print`".to_string(),
            Token::LParen => "`(`".to_string(),
            Token::RParen => "`)`".to_string(),
            Token::LBrace => "`{`".to_string(),
            Token::RBrace => "`}`".to_string(),
            Token::LBracket => "`[`".to_string(),
            Token::RBracket => "`]`".to_string(),
            Token::Colon => "`:`".to_string(),
            Token::Comma => "`,`".to_string(),
            Token::Dot => "`.`".to_string(),
            Token::Arrow => "`->`".to_string(),
            Token::Assign => "`=`".to_string(),
            Token::EqEq => "`==`".to_string(),
            Token::Plus => "`+`".to_string(),
            Token::Minus => "`-`".to_string(),
            Token::Star => "`*`".to_string(),
            Token::Lt => "`<`".to_string(),
            Token::Gt => "`>`".to_string(),
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
        let mut tweaks = Vec::new();
        let mut structs = Vec::new();
        let mut functions = Vec::new();
        while let Some(tok) = self.peek() {
            match tok {
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
                t => panic!(
                    "Expected `tweak`, `struct`, `impl`, or `fn`, found: {:?}",
                    t
                ),
            }
        }
        Program {
            tweaks,
            structs,
            functions,
        }
    }

    fn parse_struct(&mut self) -> StructDef {
        self.expect(Token::Struct);
        let Some(Token::Ident(name)) = self.next() else {
            panic!("Struct name expected")
        };
        self.expect(Token::LBrace);
        let mut fields = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let Some(Token::Ident(f_name)) = self.next() else {
                panic!("Field name expected")
            };
            self.expect(Token::Colon);
            self.next(); // Typ (`int`) überspringen
            fields.push(f_name);
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RBrace);
        StructDef { name, fields }
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
        if self.peek() == Some(&Token::Arrow) {
            self.next();
            self.next();
        }
        Function {
            name: format!("{}__{}", struct_name, m_name),
            params,
            body: self.parse_block(),
        }
    }

    pub fn parse_function(&mut self) -> Function {
        self.expect(Token::Fn);
        let Some(Token::Ident(name)) = self.next() else {
            panic!("Function Name expected")
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
                panic!("Param name expected")
            };
            self.expect(Token::Colon);
            let Some(Token::Ident(ty_name)) = self.next() else {
                panic!("Type expected")
            };
            params.push((p_name, ty_name, is_mut));
            if self.peek() == Some(&Token::Comma) {
                self.next();
            }
        }
        self.expect(Token::RParen);
        if self.peek() == Some(&Token::Arrow) {
            self.next();
            self.next();
        }
        Function {
            name,
            params,
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
                let is_mut = if self.peek() == Some(&Token::Mut) {
                    self.next();
                    true
                } else {
                    false
                };
                let Some(Token::Ident(name)) = self.next() else {
                    panic!("Name expected")
                };
                self.expect(Token::Assign);
                Stmt::Let {
                    name,
                    is_mut,
                    value: self.parse_expr(),
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
                if self.peek() == Some(&Token::LParen) {
                    Stmt::Call {
                        name,
                        args: self.parse_args(),
                    }
                } else if self.peek() == Some(&Token::Dot) {
                    self.next(); // consume `.`
                    let Some(Token::Ident(member)) = self.next() else {
                        self.error("Expected field or method name after `.`");
                    };
                    if self.peek() == Some(&Token::LParen) {
                        Stmt::MethodCall {
                            receiver: name,
                            method: member,
                            args: self.parse_args(),
                        }
                    } else {
                        self.expect(Token::Assign);
                        Stmt::FieldAssign {
                            var: name,
                            field: member,
                            value: self.parse_expr(),
                        }
                    }
                } else if self.peek() == Some(&Token::LBracket) {
                    self.next(); // `[`
                    let idx = self.parse_expr();
                    self.expect(Token::RBracket);
                    self.expect(Token::Assign);
                    Stmt::IndexAssign {
                        var: name,
                        index: idx,
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

    fn parse_expr(&mut self) -> Expr {
        let mut left = self.parse_primary();
        while let Some(op) = match self.peek() {
            Some(Token::Plus) => Some(Op::Add),
            Some(Token::Minus) => Some(Op::Sub),
            Some(Token::Star) => Some(Op::Mul),
            Some(Token::Lt) => Some(Op::Lt),
            Some(Token::Gt) => Some(Op::Gt),
            Some(Token::EqEq) => Some(Op::Eq),
            _ => None,
        } {
            self.next();
            let right = self.parse_primary();
            left = Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        left
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
        match self.next() {
            Some(Token::Int(v)) => Expr::Int(v),
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
                if self.peek() == Some(&Token::LParen) {
                    Expr::Call {
                        name: n,
                        args: self.parse_args(),
                    }
                } else if self.peek() == Some(&Token::Dot) {
                    self.next(); // `.`
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
                let e = self.parse_expr();
                self.expect(Token::RParen);
                e
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
            _ => {}
        }
    }
}
