use core::fmt;

use crate::core::Function;
use crate::core::PreinterceptToken;
use crate::core::Token;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenizerState {
    Character,
    Number,
    Symbol,
}

#[derive(Debug)]
pub enum TokenizerError {
    NoInput,
    NaN,
    UndefinedSymbol(char),
    BracketNotClosed,
    InvalidBrackets,
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            TokenizerError::NoInput => "[ERROR]: No input given".to_string(),
            TokenizerError::NaN => "[ERROR]: NaN (Not a Number)".to_string(),
            TokenizerError::UndefinedSymbol(c) => format!("[ERROR]: Undefined symbol '{}'", c),
            TokenizerError::BracketNotClosed => "[ERROR]: Bracket remains unclosed".to_string(),
            TokenizerError::InvalidBrackets => {
                "[ERROR]: Brackets are set wrong. Maybe there is somewhere ')('?".to_string()
            }
        };

        write!(f, "{}", name)
    }
}

pub fn tokenize(raw_input: &str, known_functions: &[String]) -> Result<Vec<Token>, TokenizerError> {
    tokenize_internal(raw_input, known_functions, false)
}

// For an assignment target: unknown names before '(' count as functions
pub fn tokenize_definition_side(raw_input: &str) -> Result<Vec<Token>, TokenizerError> {
    tokenize_internal(raw_input, &[], true)
}

fn tokenize_internal(
    raw_input: &str,
    known_functions: &[String],
    definition_mode: bool,
) -> Result<Vec<Token>, TokenizerError> {
    let input = preprocess(raw_input);

    if input.is_empty() {
        return Err(TokenizerError::NoInput);
    }

    let mut chars = input.chars().peekable();
    let mut result = Vec::<Token>::new();

    let mut paren_pairs_count = 0;

    while let Some(&c) = chars.peek() {
        let mut state_string = String::new();

        match get_state(&c) {
            TokenizerState::Character => {
                // An identifier is letters plus digits, so names like log2 or atan2 work
                while let Some(next_char) = chars.peek() {
                    if next_char.is_ascii_alphabetic() || next_char.is_ascii_digit() {
                        state_string.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                let followed_by_lparen = chars.peek() == Some(&'(');
                let may_function = if followed_by_lparen {
                    get_function(&state_string)
                } else {
                    None
                };

                let may_user_function = followed_by_lparen
                    && (definition_mode || known_functions.iter().any(|n| n == &state_string));

                if let Some(f) = may_function {
                    result.push(Token::Function(f));
                } else if let Some(value) = get_constant(&state_string) {
                    result.push(Token::Float(value));
                } else if may_user_function {
                    result.push(Token::UserFunction(state_string.clone()));
                } else {
                    // Unknown name: split back so "x2" becomes x*2
                    push_split(&mut result, &state_string);
                }

                state_string.clear();
            }

            TokenizerState::Number => {
                while let Some(next_char) = chars.peek() {
                    if get_state(next_char) == TokenizerState::Number {
                        state_string.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                result.push(parse_number(&state_string)?);

                state_string.clear();
            }

            TokenizerState::Symbol => {
                match c {
                    '(' => {
                        paren_pairs_count += 1;
                        result.push(Token::Lparen);
                    }
                    ')' => {
                        paren_pairs_count -= 1;
                        result.push(Token::Rparen);
                    }
                    '+' => result.push(Token::Plus),
                    '-' => result.push(Token::Minus),
                    '*' => result.push(Token::Mul),
                    '/' => result.push(Token::Div),
                    '^' => result.push(Token::Pow),
                    '=' => result.push(Token::Assign),
                    ',' => result.push(Token::Comma),
                    'π' => result.push(Token::Float(std::f64::consts::PI)),
                    _ => {
                        return Err(TokenizerError::UndefinedSymbol(c));
                    }
                }

                chars.next();
            }
        }

        if paren_pairs_count < 0 {
            return Err(TokenizerError::InvalidBrackets);
        }
    }

    if paren_pairs_count != 0 {
        return Err(TokenizerError::BracketNotClosed);
    }

    result = preintercept(result)?;

    Ok(result)
}

fn preprocess(raw_input: &str) -> String {
    raw_input.chars().filter(|c| !c.is_whitespace()).collect()
}

fn get_state(c: &char) -> TokenizerState {
    match c {
        'a'..='z' | 'A'..='Z' => TokenizerState::Character,
        '0'..='9' | '.' => TokenizerState::Number,
        _ => TokenizerState::Symbol,
    }
}

fn get_function(input: &str) -> Option<Function> {
    match input.to_ascii_uppercase().as_str() {
        "SQRT" => Some(Function::Sqrt),
        "CBRT" => Some(Function::Cbrt),
        "ABS" => Some(Function::Abs),
        "EXP" => Some(Function::Exp),
        "LN" => Some(Function::Ln),
        "LOG" => Some(Function::Log),
        "LOG2" => Some(Function::Log2),
        "SIN" => Some(Function::Sin),
        "COS" => Some(Function::Cos),
        "TAN" => Some(Function::Tan),
        "ATAN2" => Some(Function::Atan2),
        "SINH" => Some(Function::Sinh),
        "COSH" => Some(Function::Cosh),
        "TANH" => Some(Function::Tanh),
        "ASINH" => Some(Function::Asinh),
        "ACOSH" => Some(Function::Acosh),
        "ATANH" => Some(Function::Atanh),
        "ASIN" => Some(Function::Asin),
        "ACOS" => Some(Function::Acos),
        "ATAN" => Some(Function::Atan),
        "FLOOR" => Some(Function::Floor),
        "CEIL" => Some(Function::Ceil),
        "ROUND" => Some(Function::Round),
        "TRUNC" => Some(Function::Trunc),
        "SIGN" => Some(Function::Sign),
        "MIN" => Some(Function::Min),
        "MAX" => Some(Function::Max),
        "MOD" => Some(Function::Mod),
        _ => None,
    }
}

// Splits an unrecognized identifier back into variable and number tokens
fn push_split(result: &mut Vec<Token>, name: &str) {
    let mut digits = String::new();

    for c in name.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }

        if !digits.is_empty()
            && let Ok(token) = parse_number(&digits)
        {
            result.push(token);
        }
        digits.clear();

        match get_constant(&c.to_string()) {
            Some(value) => result.push(Token::Float(value)),
            None => result.push(Token::Variable(c)),
        }
    }

    if !digits.is_empty()
        && let Ok(token) = parse_number(&digits)
    {
        result.push(token);
    }
}

// Named constants; `e` also covers a lone `e` inside a letter run
fn get_constant(input: &str) -> Option<f64> {
    match input.to_ascii_uppercase().as_str() {
        "PI" => Some(std::f64::consts::PI),
        "E" => Some(std::f64::consts::E),
        _ => None,
    }
}

fn parse_number(number: &str) -> Result<Token, TokenizerError> {
    if let Ok(num) = number.parse::<i64>() {
        Ok(Token::Int(num))
    } else if let Ok(num) = number.parse::<f64>() {
        Ok(Token::Float(num))
    } else {
        Err(TokenizerError::NaN)
    }
}

pub fn preintercept(tokens: Vec<Token>) -> Result<Vec<Token>, TokenizerError> {
    let mut result = Vec::<Token>::new();
    let mut token_array = tokens.into_iter().peekable();

    while let Some(curr_token) = token_array.next() {
        let mut insert_mul = false;

        if let Some(next_token) = token_array.peek() {
            let next_token_preintercept_token = convert_to_preintercept_token(next_token);
            let curr_token_preintercept_token = convert_to_preintercept_token(&curr_token);

            if (curr_token_preintercept_token == PreinterceptToken::Variable
                || curr_token_preintercept_token == PreinterceptToken::Number
                || curr_token_preintercept_token == PreinterceptToken::Rparen)
                && (next_token_preintercept_token == PreinterceptToken::Function
                    || next_token_preintercept_token == PreinterceptToken::Lparen
                    || next_token_preintercept_token == PreinterceptToken::Variable
                    || next_token_preintercept_token == PreinterceptToken::Number)
            {
                insert_mul = true;
            }
        }

        result.push(curr_token);

        if insert_mul {
            result.push(Token::Mul);
        }
    }

    Ok(result)
}

fn convert_to_preintercept_token(token: &Token) -> PreinterceptToken {
    match token {
        Token::Variable(_) => PreinterceptToken::Variable,
        Token::Function(_) => PreinterceptToken::Function,
        Token::UserFunction(_) => PreinterceptToken::Function,
        Token::Int(_) => PreinterceptToken::Number,
        Token::Float(_) => PreinterceptToken::Number,
        Token::Plus => PreinterceptToken::Irrelevant,
        Token::Minus => PreinterceptToken::Irrelevant,
        Token::Mul => PreinterceptToken::Irrelevant,
        Token::Div => PreinterceptToken::Irrelevant,
        Token::Pow => PreinterceptToken::Irrelevant,
        Token::Assign => PreinterceptToken::Irrelevant,
        Token::Comma => PreinterceptToken::Irrelevant,
        Token::Lparen => PreinterceptToken::Lparen,
        Token::Rparen => PreinterceptToken::Rparen,
    }
}
