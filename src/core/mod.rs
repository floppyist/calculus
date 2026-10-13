use core::fmt;

pub mod environment;
pub mod node;
pub mod parser;
pub mod plot;
pub mod tokenizer;

#[derive(Debug, Clone)]
pub enum Token {
    // Dynamic
    Variable(char),
    Function(Function),
    UserFunction(String),
    // Numbers
    Int(i64),
    Float(f64),
    // Operators
    Plus,
    Minus,
    Mul,
    Div,
    Pow,
    // Assignment and argument separator
    Assign,
    Comma,
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
            Token::UserFunction(name) => format!("FUNCTION({})", name),
            Token::Plus => "PLUS".to_string(),
            Token::Minus => "MINUS".to_string(),
            Token::Mul => "MUL".to_string(),
            Token::Div => "DIV".to_string(),
            Token::Pow => "POW".to_string(),
            Token::Assign => "ASSIGN".to_string(),
            Token::Comma => "COMMA".to_string(),
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
    Sqrt,
    Sin,
    Cos,
    Tan,
}
