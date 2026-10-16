use crate::core::environment::{Environment, EvalError, IntegralOutcome};

fn simplified(expression: &str) -> String {
    Environment::new()
        .simplify(expression)
        .expect("simplify failed")
}

fn source(expression: &str) -> String {
    let environment = Environment::new();
    let node = environment.expression(expression).expect("parse failed");

    crate::core::algebra::to_source(&node)
}

fn derivative(expression: &str) -> String {
    let mut environment = Environment::new();

    environment
        .differentiate(expression)
        .expect("differentiate failed")
        .1
}

fn antiderivative(expression: &str) -> String {
    let mut environment = Environment::new();

    match environment
        .integrate(expression, None, None)
        .expect("integrate failed")
    {
        IntegralOutcome::Function { source, .. } => source,
        IntegralOutcome::Value(_) => panic!("expected a function"),
    }
}

fn definite(expression: &str, lower: &str, upper: &str) -> f64 {
    let mut environment = Environment::new();

    match environment
        .integrate(expression, Some((lower, upper)), None)
        .expect("integrate failed")
    {
        IntegralOutcome::Value(value) => value,
        IntegralOutcome::Function { .. } => panic!("expected a value"),
    }
}

#[test]
fn test_simplify_folds_constants() {
    assert_eq!(simplified("2+3"), "5");
    assert_eq!(simplified("2*3"), "6");
}

#[test]
fn test_simplify_keeps_exact_fractions() {
    assert_eq!(simplified("1/3"), "1/3");
    assert_eq!(simplified("2/4"), "1/2");
    assert_eq!(simplified("1/3+1/6"), "1/2");
    assert_eq!(simplified("(1/2)^3"), "1/8");
}

#[test]
fn test_simplify_combines_like_terms() {
    assert_eq!(simplified("x+x"), "2x");
    assert_eq!(simplified("2*x+3*x"), "5x");
    assert_eq!(simplified("x-x"), "0");
}

#[test]
fn test_simplify_combines_powers() {
    assert_eq!(simplified("x*x"), "x^2");
    assert_eq!(simplified("x^2*x^3"), "x^5");
}

#[test]
fn test_simplify_nested_powers_and_roots() {
    assert_eq!(simplified("(x^2)^3"), "x^6");
    assert_eq!(simplified("sqrt(x)^2"), "x");
    assert_eq!(simplified("cbrt(x)^3"), "x");
    assert_eq!(simplified("e^(3x)*e^(3x)"), "e^(6x)");
}

#[test]
fn test_simplify_distributes_factorised_terms_in_sums() {
    assert_eq!(simplified("(x+1)*(x+2) - x^2 - 2*x"), "x + 2");
    assert_eq!(simplified("2*x + (x+1)*3"), "5x + 3");
}

#[test]
fn test_simplify_cancels_rational_expression() {
    let expression = "(-3(e^(3x))^2 + 3e^(3x)*(e^(3x) + 1))\
/(3(cbrt(e^(3x)/(e^(3x) + 1))^3*(e^(3x) + 1)^2))";

    assert_eq!(simplified(expression), "1/(e^(3x) + 1)");
}

#[test]
fn test_simplify_cancels_common_factors() {
    assert_eq!(simplified("2*x/x"), "2");
    assert_eq!(simplified("x*y/x"), "y");
    assert_eq!(simplified("3/(3*x)"), "1/x");
    assert_eq!(simplified("x/(2*y)"), "x/(2y)");
}

#[test]
fn test_simplify_keeps_exact_ratio() {
    assert_eq!(simplified("x^3/3"), "x^3/3");
}

#[test]
fn test_derivative_polynomial() {
    assert_eq!(derivative("x^3"), "3x^2");
    assert_eq!(derivative("x^2+2*x+1"), "2x + 2");
    assert_eq!(derivative("5"), "0");
}

#[test]
fn test_derivative_trig() {
    assert_eq!(derivative("sin(x)"), "cos(x)");
    assert_eq!(derivative("sin(2*x)"), "2*cos(2x)");
    assert_eq!(derivative("cos(x)"), "-sin(x)");
}

#[test]
fn test_derivative_exponential() {
    assert_eq!(derivative("e^x"), "e^x");
    assert_eq!(derivative("e^(2*x)"), "2e^(2x)");
}

#[test]
fn test_derivative_logarithm() {
    assert_eq!(derivative("ln(x)"), "1/x");
    assert_eq!(derivative("ln(3*x)"), "1/x");
}

#[test]
fn test_derivative_reciprocal() {
    assert_eq!(derivative("1/x"), "-1/x^2");
}

#[test]
fn test_derivative_rejects_piecewise() {
    let mut environment = Environment::new();
    let result = environment.differentiate("min(x,1)");

    assert!(matches!(result.unwrap_err(), EvalError::NotDifferentiable));
}

#[test]
fn test_antiderivative_power() {
    assert_eq!(antiderivative("x^2"), "x^3/3");
    assert_eq!(antiderivative("2*x"), "x^2");
}

#[test]
fn test_antiderivative_trig() {
    assert_eq!(antiderivative("cos(x)"), "sin(x)");
    assert_eq!(antiderivative("cos(2*x)"), "sin(2x)/2");
}

#[test]
fn test_antiderivative_reciprocal() {
    assert_eq!(antiderivative("1/x"), "ln(abs(x))");
}

#[test]
fn test_definite_integral_polynomial() {
    let value = definite("x^2", "0", "1");

    assert!((value - 1.0 / 3.0).abs() < 1e-12);
}

#[test]
fn test_definite_integral_sine() {
    let value = definite("sin(x)", "0", "pi");

    assert!((value - 2.0).abs() < 1e-12);
}

#[test]
fn test_definite_integral_uses_numeric_fallback() {
    // exp(-x^2) has no elementary antiderivative
    let value = definite("exp(-x^2)", "0", "1");

    assert!((value - 0.7468241328).abs() < 1e-6);
}

#[test]
fn test_integrate_rejects_piecewise() {
    let mut environment = Environment::new();
    let result = environment.integrate("mod(x,2)", None, None);

    assert!(matches!(result.unwrap_err(), EvalError::NotIntegrable));
}

#[test]
fn test_integrate_stores_auto_named_function() {
    let mut environment = Environment::new();
    let outcome = environment.integrate("x^2", None, None).unwrap();

    match outcome {
        IntegralOutcome::Function {
            name,
            param,
            source,
        } => {
            assert_eq!(name, "f");
            assert_eq!(param, 'x');
            assert_eq!(source, "x^3/3");
        }
        IntegralOutcome::Value(_) => panic!("expected a function"),
    }

    let value = environment.execute("f(3)").unwrap();
    let formatted = crate::ui::format::format_outcome(value);
    assert_eq!(formatted, "= 9");
}

#[test]
fn test_integrate_function_call() {
    let mut environment = Environment::new();
    environment.execute("f(x)=2*x").unwrap();

    let outcome = environment.integrate("f(x)", None, None).unwrap();

    match outcome {
        IntegralOutcome::Function { source, .. } => assert_eq!(source, "x^2"),
        IntegralOutcome::Value(_) => panic!("expected a function"),
    }
}

#[test]
fn test_integrate_with_respect_to_variable() {
    let mut environment = Environment::new();
    environment.execute("f(x,y)=x*y").unwrap();

    let outcome = environment.integrate("f(x,y)", None, Some('y')).unwrap();

    match outcome {
        IntegralOutcome::Function { param, source, .. } => {
            assert_eq!(param, 'y');
            assert!(source.contains("y^2"), "unexpected source {source}");
        }
        IntegralOutcome::Value(_) => panic!("expected a function"),
    }
}

#[test]
fn test_integrate_recursive_function_is_not_integrable() {
    let mut environment = Environment::new();
    environment.execute("r(x)=r(x)+1").unwrap();

    let result = environment.integrate("r(x)", None, None);

    assert!(matches!(result.unwrap_err(), EvalError::NotIntegrable));
}

fn solutions(equation: &str, variable: char) -> Vec<String> {
    let environment = Environment::new();

    environment
        .solve(equation, variable)
        .expect("solve failed")
        .solutions
}

#[test]
fn test_solve_linear_symbolic() {
    assert_eq!(solutions("3x=2a+2x+3", 'x'), vec!["2a + 3".to_string()]);
    assert_eq!(solutions("2x+4=0", 'x'), vec!["-2".to_string()]);
}

#[test]
fn test_solve_quadratic() {
    assert_eq!(
        solutions("x^2-1=0", 'x'),
        vec!["1".to_string(), "-1".to_string()]
    );
    assert_eq!(solutions("x^2+1=0", 'x'), Vec::<String>::new());
}

#[test]
fn test_solve_rejects_unsupported() {
    let environment = Environment::new();

    assert!(matches!(
        environment.solve("x^3=8", 'x').unwrap_err(),
        EvalError::NotSolvable
    ));
    assert!(matches!(
        environment.solve("2x=2x+1", 'x').unwrap_err(),
        EvalError::NotSolvable
    ));
}

#[test]
fn test_output_star_after_exponent() {
    assert_eq!(simplified("(3*d-2*z)^3*x^2"), "(3d - 2z)^3*x^2");
    assert_eq!(simplified("2*x^2"), "2x^2");
    assert_eq!(simplified("x^2*y"), "x^2*y");
    assert_eq!(source("sin(x)^2*cos(x)"), "sin(x)^2*cos(x)");
}

#[test]
fn test_output_reparses_to_the_same_values() {
    let expressions = [
        "2xy+x^2-4y+2x^2-5xy",
        "2*(x+y)",
        "x*(y+z)",
        "3*x*y-(x*y)",
        "sin(x)*cos(x)",
        "pi*x+e*x",
        "x^-2",
        "2*(x+y)^2",
        "-(x+y)*2",
        "x/(2*y)",
    ];

    for expression in expressions {
        let source = simplified(expression);

        let mut environment = Environment::new();
        environment.execute("x=2").unwrap();
        environment.execute("y=3").unwrap();
        environment.execute("z=5").unwrap();

        let (original, reparsed) = match (
            environment.execute(expression).unwrap(),
            environment.execute(&source).unwrap(),
        ) {
            (
                crate::core::environment::Outcome::Value(original),
                crate::core::environment::Outcome::Value(reparsed),
            ) => (original, reparsed),
            _ => panic!("expected values for {}", expression),
        };

        assert!(
            (original - reparsed).abs() < 1e-9,
            "{} -> {}",
            expression,
            source
        );
    }
}

#[test]
fn test_calculus_sources_reparses() {
    let mut environment = Environment::new();
    let (_, derivative) = environment.differentiate("x^3").unwrap();

    environment
        .execute(&format!("d(x)={}", derivative))
        .unwrap();

    match environment.execute("d(2)").unwrap() {
        crate::core::environment::Outcome::Value(value) => assert_eq!(value, 12.0),
        _ => panic!("expected a value"),
    }

    let mut environment = Environment::new();
    let (_, derivative) = environment.differentiate("sin(2*x)").unwrap();

    environment
        .execute(&format!("d(x)={}", derivative))
        .unwrap();

    match environment.execute("d(0)").unwrap() {
        crate::core::environment::Outcome::Value(value) => assert_eq!(value, 2.0),
        _ => panic!("expected a value"),
    }
}

#[test]
fn test_simplify_sorts_like_a_polynomial() {
    assert_eq!(simplified("x^3+4+x+2*x^2+c"), "x^3 + 2x^2 + x + c + 4");
    assert_eq!(simplified("sin(x)+x^2"), "x^2 + sin(x)");
}

#[test]
fn test_derivative_hyperbolic_inverse() {
    assert_eq!(derivative("asinh(x)"), "1/sqrt(x^2 + 1)");
    assert_eq!(derivative("atanh(x)"), "1/(-x^2 + 1)");
}

#[test]
fn test_inverse_of_builtins() {
    let environment = Environment::new();

    assert_eq!(environment.simplify("sin(x)^-1").unwrap(), "asin(x)");
    assert_eq!(environment.simplify("exp(x)^-1").unwrap(), "ln(x)");
    assert_eq!(environment.simplify("sqrt(x)^-1").unwrap(), "x^2");
    assert_eq!(environment.simplify("log(x)^-1").unwrap(), "10^x");
}

#[test]
fn test_inverse_of_user_function() {
    let mut environment = Environment::new();

    environment.execute("f(x)=2*x+6").unwrap();
    environment.execute("g(x)=f(x)^-1").unwrap();

    assert_eq!(
        environment.function_definitions(),
        vec!["f(x) = 2x + 6", "g(x) = (x - 6)/2"]
    );

    let value = environment.execute("g(4)").unwrap();

    match value {
        crate::core::environment::Outcome::Value(value) => assert_eq!(value, -1.0),
        _ => panic!("expected a value"),
    }
}

#[test]
fn test_inverse_of_call_with_undefined_variable_is_symbolic() {
    let mut environment = Environment::new();

    environment.execute("g(x)=2*x+3").unwrap();

    match environment.execute("g(x)^-1").unwrap() {
        crate::core::environment::Outcome::Source(source) => assert_eq!(source, "(y - 3)/2"),
        other => panic!("expected a source, got {:?}", other),
    }

    environment.execute("h(t)=3*t+4").unwrap();

    match environment.execute("h(q)^-1").unwrap() {
        crate::core::environment::Outcome::Source(source) => assert_eq!(source, "(y - 4)/3"),
        other => panic!("expected a source, got {:?}", other),
    }
}

#[test]
fn test_inverse_of_builtin_with_undefined_variable_is_symbolic() {
    let mut environment = Environment::new();

    match environment.execute("sin(x)^-1").unwrap() {
        crate::core::environment::Outcome::Source(source) => assert_eq!(source, "asin(y)"),
        other => panic!("expected a source, got {:?}", other),
    }
}

#[test]
fn test_inverse_exponent_keeps_reciprocal_for_plain_terms() {
    let mut environment = Environment::new();

    environment.execute("x=2").unwrap();

    for (expression, expected) in [("x^-1", 0.5), ("(x+1)^-1", 1.0 / 3.0)] {
        match environment.execute(expression).unwrap() {
            crate::core::environment::Outcome::Value(value) => {
                assert_eq!(value, expected, "{}", expression)
            }
            _ => panic!("expected a value"),
        }
    }
}

#[test]
fn test_inverse_rejects_unknown() {
    let mut environment = Environment::new();

    let result = environment.execute("min(x,2)^-1");

    assert!(matches!(result.unwrap_err(), EvalError::NotInvertible));
}
