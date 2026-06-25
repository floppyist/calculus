use core::fmt;

pub mod parser;
pub mod tokenizer;

#[derive(Debug, Clone)]
pub enum Token {
    // Dynamic
    Variable(char),
    Function(Function),
    // Numbers
    Int(i64),
    Float(f64),
    // Operators
    Plus,
    Minus,
    Mul,
    Div,
    Pow,
    // Symbols
    Lparen,
    Rparen,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Token::Int(num) => format!("INTEGER({})", num),
            Token::Float(num) => format!("FLOAT({})", num),
            Token::Variable(chr) => format!("VARIABLE({})", chr),
            Token::Function(fct) => format!("FUNCTION({:?})", fct),
            Token::Plus => "PLUS".to_string(),
            Token::Minus => "MINUS".to_string(),
            Token::Mul => "MUL".to_string(),
            Token::Div => "DIV".to_string(),
            Token::Pow => "POW".to_string(),
            Token::Lparen => "LPAREN".to_string(),
            Token::Rparen => "RPAREN".to_string(),
        };

        write!(f, "{}", name)
    }
}

// Used for preintercept in tokenizer
#[derive(PartialEq)]
pub enum PreinterceptToken {
    Variable,
    Function,
    Number,
    Lparen,
    Rparen,
    Irrelevant,
}

#[derive(Debug, Clone)]
pub enum Function {
    Sin,
    Cos,
    Tan,
}

#[derive(Debug)]
pub enum Node {
    IntNumber(i64),
    FloatNumber(f64),

    Variable(char),

    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),

    Pow(Box<Node>, Box<Node>),

    Sin(Box<Node>),
    Cos(Box<Node>),
    Tan(Box<Node>)
}
