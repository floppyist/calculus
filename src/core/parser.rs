use std::iter::Peekable;
use std::slice::Iter;

use crate::core::Function;
use crate::core::Token;
use crate::core::node::Node;

pub fn generate_ast(tokens: Vec<Token>) -> Node {
    let mut iter_tokens = tokens.iter().peekable();

    parse_expression(&mut iter_tokens)
}

fn parse_expression(iter: &mut Peekable<Iter<Token>>) -> Node {
    let mut left = parse_term(iter);

    while let Some(token) = iter.peek() {
        match token {
            Token::Plus => {
                iter.next();

                let right = parse_term(iter);

                left = Node::Add(Box::new(left), Box::new(right));
            }

            Token::Minus => {
                iter.next();

                let right = parse_term(iter);

                left = Node::Sub(Box::new(left), Box::new(right));
            }

            _ => break,
        }
    }

    left
}

fn parse_term(iter: &mut Peekable<Iter<Token>>) -> Node {
    let mut left = parse_power(iter);

    while let Some(token) = iter.peek() {
        match token {
            Token::Mul => {
                iter.next();

                let right = parse_power(iter);

                left = Node::Mul(Box::new(left), Box::new(right));
            }

            Token::Div => {
                iter.next();

                let right = parse_power(iter);

                left = Node::Div(Box::new(left), Box::new(right));
            }

            _ => break,
        }
    }

    left
}

fn parse_power(iter: &mut Peekable<Iter<Token>>) -> Node {
    let mut left = parse_factor(iter);

    while let Some(token) = iter.peek() {
        match token {
            Token::Pow => {
                iter.next();

                // Pow is right assoziative so we have to trigger that here
                let right = parse_power(iter);

                left = Node::Pow(Box::new(left), Box::new(right));
            }

            _ => break,
        }
    }

    left
}

fn parse_factor(iter: &mut Peekable<Iter<Token>>) -> Node {
    match iter.next() {
        Some(Token::Int(val)) => Node::IntNumber(*val),
        Some(Token::Float(val)) => Node::FloatNumber(*val),
        Some(Token::Variable(name)) => Node::Variable(*name),
        Some(Token::Function(func)) => {
            match iter.next() {
                Some(Token::Lparen) => {}
                _ => panic!("Syntax error: Expected '(' after function name!"),
            }

            let inner_expression = parse_expression(iter);

            match iter.next() {
                Some(Token::Rparen) => {}
                _ => panic!("Syntax error: Expected ')' after function name!"),
            }

            match func {
                Function::Sqrt => Node::Sqrt(Box::new(inner_expression)),
                Function::Sin => Node::Sin(Box::new(inner_expression)),
                Function::Cos => Node::Cos(Box::new(inner_expression)),
                Function::Tan => Node::Tan(Box::new(inner_expression)),
            }
        }
        Some(Token::Lparen) => {
            let inner_expression = parse_expression(iter);

            match iter.next() {
                Some(Token::Rparen) => inner_expression,
                _ => panic!("Syntax error: no closing bracket ')'!"),
            }
        }
        _ => panic!("Syntax error"),
    }
}
