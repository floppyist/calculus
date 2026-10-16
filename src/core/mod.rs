use core::fmt;

pub mod algebra;
pub mod command;
pub mod environment;
pub mod node;
pub mod parser;
pub mod plot;
pub mod rational;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Function {
    // Roots and powers
    Sqrt,
    Cbrt,
    Abs,
    // Exponential and logarithms
    Exp,
    Ln,
    Log,
    Log2,
    // Trigonometry
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    // Hyperbolic
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    // Rounding and sign
    Floor,
    Ceil,
    Round,
    Trunc,
    Sign,
    // Binary helpers
    Min,
    Max,
    Mod,
}

impl Function {
    pub fn name(self) -> &'static str {
        match self {
            Function::Sqrt => "sqrt",
            Function::Cbrt => "cbrt",
            Function::Abs => "abs",
            Function::Exp => "exp",
            Function::Ln => "ln",
            Function::Log => "log",
            Function::Log2 => "log2",
            Function::Sin => "sin",
            Function::Cos => "cos",
            Function::Tan => "tan",
            Function::Asin => "asin",
            Function::Acos => "acos",
            Function::Atan => "atan",
            Function::Atan2 => "atan2",
            Function::Sinh => "sinh",
            Function::Cosh => "cosh",
            Function::Tanh => "tanh",
            Function::Asinh => "asinh",
            Function::Acosh => "acosh",
            Function::Atanh => "atanh",
            Function::Floor => "floor",
            Function::Ceil => "ceil",
            Function::Round => "round",
            Function::Trunc => "trunc",
            Function::Sign => "sign",
            Function::Min => "min",
            Function::Max => "max",
            Function::Mod => "mod",
        }
    }

    // Accepted argument counts; `log(x)` base 10 and `log(x, base)`, round to n digits
    pub fn arities(self) -> &'static [usize] {
        match self {
            Function::Log | Function::Round => &[1, 2],
            Function::Atan2 | Function::Min | Function::Max | Function::Mod => &[2],
            _ => &[1],
        }
    }

    pub fn eval(self, args: &[f64]) -> f64 {
        match self {
            Function::Sqrt => args[0].sqrt(),
            Function::Cbrt => args[0].cbrt(),
            Function::Abs => args[0].abs(),
            Function::Exp => args[0].exp(),
            Function::Ln => args[0].ln(),
            Function::Log => match args {
                [value] => value.log10(),
                [value, base] => value.log(*base),
                _ => f64::NAN,
            },
            Function::Log2 => args[0].log2(),
            Function::Sin => args[0].sin(),
            Function::Cos => args[0].cos(),
            Function::Tan => args[0].tan(),
            Function::Asin => args[0].asin(),
            Function::Acos => args[0].acos(),
            Function::Atan => args[0].atan(),
            Function::Atan2 => args[0].atan2(args[1]),
            Function::Sinh => args[0].sinh(),
            Function::Cosh => args[0].cosh(),
            Function::Tanh => args[0].tanh(),
            Function::Asinh => args[0].asinh(),
            Function::Acosh => args[0].acosh(),
            Function::Atanh => args[0].atanh(),
            Function::Floor => args[0].floor(),
            Function::Ceil => args[0].ceil(),
            Function::Round => match args {
                [value] => value.round(),
                [value, digits] => round_to(*value, *digits),
                _ => f64::NAN,
            },
            Function::Trunc => args[0].trunc(),
            Function::Sign => {
                if args[0].is_nan() {
                    f64::NAN
                } else if args[0] > 0.0 {
                    1.0
                } else if args[0] < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            }
            Function::Min => args[0].min(args[1]),
            Function::Max => args[0].max(args[1]),
            Function::Mod => args[0] % args[1],
        }
    }
}

// Rounds to a number of decimal places (negative digits round to tens, ...)
fn round_to(value: f64, digits: f64) -> f64 {
    let digits = digits.clamp(-15.0, 15.0) as i32;
    let factor = 10f64.powi(digits);

    (value * factor).round() / factor
}
