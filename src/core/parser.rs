use std::fmt;
use std::iter::Peekable;
use std::slice::Iter;

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
            ParseError::ExpectedClosingParen => "[ERROR]: Syntax error: Expected ')'!".to_string(),
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
    let mut left = parse_unary(iter)?;

    while let Some(token) = iter.peek() {
        match token {
            Token::Mul => {
                iter.next();

                let right = parse_unary(iter)?;

                left = Node::Mul(Box::new(left), Box::new(right));
            }

            Token::Div => {
                iter.next();

                let right = parse_unary(iter)?;

                left = Node::Div(Box::new(left), Box::new(right));
            }

            _ => break,
        }
    }

    Ok(left)
}

// Unary plus/minus binds looser than the power operator: -2^2 = -(2^2)
fn parse_unary(iter: &mut Peekable<Iter<Token>>) -> Result<Node, ParseError> {
    match iter.peek() {
        Some(Token::Minus) => {
            iter.next();

            let inner = parse_unary(iter)?;

            Ok(Node::Neg(Box::new(inner)))
        }
        Some(Token::Plus) => {
            iter.next();

            parse_unary(iter)
        }
        _ => parse_power(iter),
    }
}

fn parse_power(iter: &mut Peekable<Iter<Token>>) -> Result<Node, ParseError> {
    let left = parse_factor(iter)?;

    if let Some(Token::Pow) = iter.peek() {
        iter.next();

        // Pow is right associative, and its exponent may be signed
        let right = parse_unary(iter)?;

        return Ok(Node::Pow(Box::new(left), Box::new(right)));
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

            let arguments = parse_arguments(iter)?;

            expect_rparen(iter)?;

            Ok(Node::Builtin(*func, arguments))
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
