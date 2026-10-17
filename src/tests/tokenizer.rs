use crate::core::Token;
use crate::core::tokenizer::{TokenizerError, tokenize, tokenize_definition_side};

fn check(input: &str) -> Vec<Token> {
    tokenize(input, &[], &[]).expect("Tokenisierungs-Fehler bei validem Input")
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
fn test_assignment_tokens() {
    let tokens = check("x=2");

    assert_eq!(format!("{:?}", tokens[0]), "Variable('x')");
    assert_eq!(format!("{:?}", tokens[1]), "Assign");
    assert_eq!(format!("{:?}", tokens[2]), "Int(2)");
}

#[test]
fn test_user_function_call_with_arguments() {
    let tokens = check("f(1,2)");

    // Without a known function, 'f' is split into individual variables
    assert!(tokens.iter().any(|t| matches!(t, Token::Variable('f'))));

    let known = tokenize("f(1,2)", &["f".to_string()], &[]).unwrap();

    assert_eq!(format!("{:?}", known[0]), "UserFunction(\"f\")");
    assert_eq!(format!("{:?}", known[2]), "Int(1)");
    assert_eq!(format!("{:?}", known[3]), "Comma");
    assert_eq!(format!("{:?}", known[4]), "Int(2)");
}

#[test]
fn test_definition_side_recognizes_function_signature() {
    let tokens = tokenize_definition_side("f(x,y)").unwrap();

    assert_eq!(format!("{:?}", tokens[0]), "UserFunction(\"f\")");
    assert_eq!(format!("{:?}", tokens[2]), "Variable('x')");
    assert_eq!(format!("{:?}", tokens[3]), "Comma");
    assert_eq!(format!("{:?}", tokens[4]), "Variable('y')");
}

#[test]
fn test_error_unclosed_bracket() {
    let result = tokenize("3 * (x + 5", &[], &[]);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TokenizerError::BracketNotClosed
    ));
}

#[test]
fn test_error_invalid_brackets_order() {
    let result = tokenize("3 + )x + 5(", &[], &[]);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TokenizerError::InvalidBrackets
    ));
}

#[test]
fn test_error_undefined_symbol() {
    let result = tokenize("3 @ x", &[], &[]);

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

    assert_eq!(tokens.len(), 11);
    assert_eq!(format!("{:?}", tokens[0]), "Variable('a')");
    assert_eq!(format!("{:?}", tokens[10]), "Int(5)");
}

#[test]
fn test_number_after_bracket_without_any_operator() {
    let tokens = check("(1+2)3");

    assert_eq!(tokens.len(), 7);
    assert_eq!(format!("{:?}", tokens[0]), "Lparen");
    assert_eq!(format!("{:?}", tokens[5]), "Mul");
    assert_eq!(format!("{:?}", tokens[6]), "Int(3)");
}

#[test]
fn test_unknown_name_peels_function_suffix() {
    let tokens = check("xsin(x)");

    assert_eq!(format!("{:?}", tokens[0]), "Variable('x')");
    assert_eq!(format!("{:?}", tokens[1]), "Mul");
    assert_eq!(format!("{:?}", tokens[2]), "Function(Sin)");
}

#[test]
fn test_inverse_trig_functions() {
    let tokens = check("asin(x)+acos(x)+atan(x)");

    assert_eq!(format!("{:?}", tokens[0]), "Function(Asin)");
    assert_eq!(format!("{:?}", tokens[5]), "Function(Acos)");
    assert_eq!(format!("{:?}", tokens[10]), "Function(Atan)");
}

#[test]
fn test_hyperbolic_functions() {
    let tokens = check("asinh(x)+ACOSH(x)+atanh(x)");

    assert_eq!(format!("{:?}", tokens[0]), "Function(Asinh)");
    assert_eq!(format!("{:?}", tokens[5]), "Function(Acosh)");
    assert_eq!(format!("{:?}", tokens[10]), "Function(Atanh)");
}

#[test]
fn test_sum_and_product_tokens() {
    let sum = check("sum(k,1,3,k)");
    let prod = check("prod(k,1,3,k)");
    let sym_sum = check("∑(k,1,3,k)");
    let sym_prod = check("∏(k,1,3,k)");

    assert_eq!(format!("{:?}", sum[0]), "Sum");
    assert_eq!(format!("{:?}", sum[1]), "Lparen");
    assert_eq!(format!("{:?}", sum[2]), "Variable('k')");

    assert_eq!(format!("{:?}", prod[0]), "Product");
    assert_eq!(format!("{:?}", sym_sum[0]), "Sum");
    assert_eq!(format!("{:?}", sym_prod[0]), "Product");
}

#[test]
fn test_sum_needs_parens_and_index() {
    // Without '(' the name falls apart into single letters as before
    let bare = check("sum");

    assert_eq!(format!("{:?}", bare[0]), "Variable('s')");

    assert!(matches!(
        tokenize("∑x", &[], &[]).unwrap_err(),
        TokenizerError::UndefinedSymbol('∑')
    ));
}
