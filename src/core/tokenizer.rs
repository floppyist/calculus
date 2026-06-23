use core::fmt;

use crate::core::{Function, Token};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenizerState {
    Character,
    Number,
    Symbol,
}

#[derive(Debug)]
pub enum TokenizerError {
    NaN,
    UndefinedSymbol(char),
    BracketNotClosed,
    InvalidBrackets,
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            TokenizerError::NaN => "ERROR: NaN (Not a Number)".to_string(),
            TokenizerError::UndefinedSymbol(c) => format!("ERROR: Undefined symbol '{}'", c),
            TokenizerError::BracketNotClosed => "ERROR: Bracket remains unclosed".to_string(),
            TokenizerError::InvalidBrackets => {
                "ERROR: Brackets are set wrong. Maybe there is somewhere ')('?".to_string()
            }
        };

        write!(f, "{}", name)
    }
}

pub fn tokenize(raw_input: &str) -> Result<Vec<Token>, TokenizerError> {
    let input = preprocess(raw_input);

    let mut chars = input.chars().peekable();
    let mut result = Vec::<Token>::new();

    let mut paren_pairs_count = 0;

    while let Some(&c) = chars.peek() {
        let mut state_string = String::new();

        match get_state(&c) {
            TokenizerState::Character => {
                while let Some(next_char) = chars.peek() {
                    if get_state(next_char) == TokenizerState::Character {
                        state_string.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                let may_function = if chars.peek() == Some(&'(') {
                    get_function(&state_string)
                } else {
                    None
                };

                if let Some(f) = may_function {
                    result.push(Token::Function(f));
                } else {
                    for var in state_string.chars() {
                        result.push(Token::Variable(var));
                    }
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
        "SIN" => Some(Function::Sin),
        "COS" => Some(Function::Cos),
        "TAN" => Some(Function::Tan),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn check(input: &str) -> Vec<Token> {
        tokenize(input).expect("Tokenisierungs-Fehler bei validem Input")
    }

    #[test]
    fn test_basic_math() {
        let tokens = check("3 + 4 * x");

        assert_eq!(tokens.len(), 5);
        assert_eq!(format!("{:?}", tokens[0]), "Int(3)");
        assert_eq!(format!("{:?}", tokens[2]), "Int(4)");
        assert_eq!(format!("{:?}", tokens[4]), "Variable('x')");
    }

    #[test]
    fn test_floats_and_zeros() {
        let tokens = check("0.00 + .5");

        assert_eq!(format!("{:?}", tokens[0]), "Float(0.0)");
        assert_eq!(format!("{:?}", tokens[2]), "Float(0.5)");
    }

    #[test]
    fn test_functions() {
        let tokens = check("sin(x) + COS(y)");

        assert_eq!(format!("{:?}", tokens[0]), "Function(Sin)");
        assert_eq!(format!("{:?}", tokens[5]), "Function(Cos)");
    }

    #[test]
    fn test_whitespace_handling() {
        let tokens_compact = check("3+sin(x)");
        let tokens_spaced = check("3  +  sin  (  x  )");

        assert_eq!(
            format!("{:?}", tokens_compact),
            format!("{:?}", tokens_spaced)
        );
    }

    #[test]
    fn test_error_unclosed_bracket() {
        let result = tokenize("3 * (x + 5");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TokenizerError::BracketNotClosed
        ));
    }

    #[test]
    fn test_error_invalid_brackets_order() {
        let result = tokenize("3 + )x + 5(");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TokenizerError::InvalidBrackets
        ));
    }

    #[test]
    fn test_error_undefined_symbol() {
        let result = tokenize("3 @ x");

        assert!(result.is_err());

        if let Err(TokenizerError::UndefinedSymbol(c)) = result {
            assert_eq!(c, '@');
        } else {
            panic!("Erwartete UndefinedSymbol Fehler");
        }
    }

    #[test]
    fn test_edge_case_unknown_function_or_variables() {
        let tokens = check("abc(x) + 5");

        assert_eq!(tokens.len(), 8);
        assert_eq!(format!("{:?}", tokens[0]), "Variable('a')");
        assert_eq!(format!("{:?}", tokens[7]), "Int(5)");
    }
}
