use logos::Logos;

// AST

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, bool)>,
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
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Vec<Stmt>,
    },
    Return(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Var(String),
    BinOp {
        left: Box<Expr>,
        op: Op,
        right: Box<Expr>,
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

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(":")]
    Colon,
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
}

// Parser

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(src: &str) -> Self {
        let tokens = Token::lexer(src).filter_map(Result::ok).collect();
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn expect(&mut self, expected: Token) {
        let got = self.next();
        assert_eq!(
            got.as_ref(),
            Some(&expected),
            "Expected: {:?}, Found: {:?}",
            expected,
            got
        );
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
            self.next(); // Typ-Name (`int`)
            params.push((p_name, is_mut));
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
            Some(Token::Return) => {
                self.next();
                Stmt::Return(self.parse_expr())
            }
            Some(Token::Ident(_)) => {
                let Some(Token::Ident(name)) = self.next() else {
                    unreachable!()
                };
                self.expect(Token::Assign);
                Stmt::Assign {
                    name,
                    value: self.parse_expr(),
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

    fn parse_primary(&mut self) -> Expr {
        match self.next() {
            Some(Token::Int(v)) => Expr::Int(v),
            Some(Token::Ident(n)) => Expr::Var(n),
            Some(Token::LParen) => {
                let e = self.parse_expr();
                self.expect(Token::RParen);
                e
            }
            t => panic!("Expected expression, found: {:?}", t),
        }
    }
}
