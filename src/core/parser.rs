use std::fmt;
use std::iter::Peekable;
use std::slice::Iter;

use crate::core::Function;
use crate::core::Token;
use crate::core::node::Node;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken,
    ExpectedOpeningParen,
    ExpectedClosingParen,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ParseError::UnexpectedToken => "[ERROR]: Syntax error: Unexpected token".to_string(),
            ParseError::ExpectedOpeningParen => {
                "[ERROR]: Syntax error: Expected '(' after function name!".to_string()
            }
            ParseError::ExpectedClosingParen => {
                "[ERROR]: Syntax error: Expected ')'!".to_string()
            }
        };

        write!(f, "{}", name)
    }
}

pub fn generate_ast(tokens: Vec<Token>) -> Result<Node, ParseError> {
    let mut iter_tokens = tokens.iter().peekable();

    parse_expression(&mut iter_tokens)
}

fn parse_expression(iter: &mut Peekable<Iter<Token>>) -> Result<Node, ParseError> {
    let mut left = parse_term(iter)?;

    while let Some(token) = iter.peek() {
        match token {
            Token::Plus => {
                iter.next();

                let right = parse_term(iter)?;

                left = Node::Add(Box::new(left), Box::new(right));
            }

            Token::Minus => {
                iter.next();

                let right = parse_term(iter)?;

                left = Node::Sub(Box::new(left), Box::new(right));
            }

            _ => break,
        }
    }

    Ok(left)
}

fn parse_term(iter: &mut Peekable<Iter<Token>>) -> Result<Node, ParseError> {
    let mut left = parse_power(iter)?;

    while let Some(token) = iter.peek() {
        match token {
            Token::Mul => {
                iter.next();

                let right = parse_power(iter)?;

                left = Node::Mul(Box::new(left), Box::new(right));
            }

            Token::Div => {
                iter.next();

                let right = parse_power(iter)?;

                left = Node::Div(Box::new(left), Box::new(right));
            }

            _ => break,
        }
    }

    Ok(left)
}

fn parse_power(iter: &mut Peekable<Iter<Token>>) -> Result<Node, ParseError> {
    let mut left = parse_factor(iter)?;

    while let Some(token) = iter.peek() {
        match token {
            Token::Pow => {
                iter.next();

                // Pow is right assoziative so we have to trigger that here
                let right = parse_power(iter)?;

                left = Node::Pow(Box::new(left), Box::new(right));
            }

            _ => break,
        }
    }

    Ok(left)
}

fn parse_factor(iter: &mut Peekable<Iter<Token>>) -> Result<Node, ParseError> {
    match iter.next() {
        Some(Token::Int(val)) => Ok(Node::IntNumber(*val)),
        Some(Token::Float(val)) => Ok(Node::FloatNumber(*val)),
        Some(Token::Variable(name)) => Ok(Node::Variable(*name)),

        Some(Token::Function(func)) => {
            expect_lparen(iter)?;

            let inner_expression = parse_expression(iter)?;

            expect_rparen(iter)?;

            Ok(match func {
                Function::Sqrt => Node::Sqrt(Box::new(inner_expression)),
                Function::Sin => Node::Sin(Box::new(inner_expression)),
                Function::Cos => Node::Cos(Box::new(inner_expression)),
                Function::Tan => Node::Tan(Box::new(inner_expression)),
            })
        }

        Some(Token::UserFunction(name)) => {
            expect_lparen(iter)?;

            let arguments = parse_arguments(iter)?;

            expect_rparen(iter)?;

            Ok(Node::Call(name.clone(), arguments))
        }

        Some(Token::Lparen) => {
            let inner_expression = parse_expression(iter)?;

            expect_rparen(iter)?;

            Ok(inner_expression)
        }

        _ => Err(ParseError::UnexpectedToken),
    }
}

// Comma-separated argument list; empty parentheses allowed
fn parse_arguments(iter: &mut Peekable<Iter<Token>>) -> Result<Vec<Node>, ParseError> {
    let mut arguments = Vec::new();

    if let Some(Token::Rparen) = iter.peek() {
        return Ok(arguments);
    }

    loop {
        arguments.push(parse_expression(iter)?);

        match iter.peek() {
            Some(Token::Comma) => {
                iter.next();
            }
            _ => break,
        }
    }

    Ok(arguments)
}

fn expect_lparen(iter: &mut Peekable<Iter<Token>>) -> Result<(), ParseError> {
    match iter.next() {
        Some(Token::Lparen) => Ok(()),
        _ => Err(ParseError::ExpectedOpeningParen),
    }
}

fn expect_rparen(iter: &mut Peekable<Iter<Token>>) -> Result<(), ParseError> {
    match iter.next() {
        Some(Token::Rparen) => Ok(()),
        _ => Err(ParseError::ExpectedClosingParen),
    }
}
