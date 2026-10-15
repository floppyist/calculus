use crate::core::environment::{Environment, EvalError, Outcome};

fn value_of(environment: &Environment, name: char) -> f64 {
    *environment
        .values()
        .get(&name)
        .expect("variable is missing")
}

#[test]
fn test_variable_assignment_both_directions() {
    let mut environment = Environment::new();

    environment.execute("x=2").unwrap();
    assert_eq!(value_of(&environment, 'x'), 2.0);

    environment.execute("5=x").unwrap();
    assert_eq!(value_of(&environment, 'x'), 5.0);
}

#[test]
fn test_dependent_variable_is_recomputed() {
    let mut environment = Environment::new();

    environment.execute("x=2").unwrap();
    environment.execute("y=x+1").unwrap();
    assert_eq!(value_of(&environment, 'y'), 3.0);

    // Changing x must update y
    environment.execute("x=5").unwrap();
    assert_eq!(value_of(&environment, 'y'), 6.0);
}

#[test]
fn test_dependency_order_is_independent() {
    let mut environment = Environment::new();

    // y is defined before x but must still compute correctly
    environment.execute("y=x+1").unwrap();
    environment.execute("x=10").unwrap();
    assert_eq!(value_of(&environment, 'y'), 11.0);
}

#[test]
fn test_cyclic_dependency_is_detected() {
    let mut environment = Environment::new();

    environment.execute("a=b+1").unwrap();
    let result = environment.execute("b=a+1");

    assert!(matches!(result.unwrap_err(), EvalError::CyclicDependency));
}

#[test]
fn test_function_definition_and_call() {
    let mut environment = Environment::new();

    environment.execute("f(x,y)=x+y").unwrap();

    let outcome = environment.execute("f(3,4)").unwrap();

    assert!(matches!(outcome, Outcome::Value(value) if value == 7.0));
}

#[test]
fn test_function_wrong_argument_count() {
    let mut environment = Environment::new();

    environment.execute("f(x)=x+1").unwrap();

    let result = environment.execute("f(1,2)");

    assert!(matches!(
        result.unwrap_err(),
        EvalError::WrongArgumentCount(_)
    ));
}

#[test]
fn test_function_uses_global_variables_reactively() {
    let mut environment = Environment::new();

    environment.execute("g(x)=x*a").unwrap();
    environment.execute("a=3").unwrap();

    let outcome = environment.execute("g(2)").unwrap();

    assert!(matches!(outcome, Outcome::Value(value) if value == 6.0));
}

#[test]
fn test_function_result_preview_requires_set_variables() {
    let mut environment = Environment::new();

    environment.execute("f(x)=x^2").unwrap();

    // Without x set there is no preview
    let definition = environment
        .functions()
        .get("f")
        .expect("function is missing");

    assert!(environment.function_result(definition).is_none());

    environment.execute("x=4").unwrap();

    let definition = environment
        .functions()
        .get("f")
        .expect("function is missing");

    assert_eq!(environment.function_result(definition), Some(16.0));
}

#[test]
fn test_invalid_assignment() {
    let mut environment = Environment::new();

    assert!(matches!(
        environment.execute("1+2=3+4").unwrap_err(),
        EvalError::InvalidAssignment
    ));
}

#[test]
fn test_function_calls_compose_in_definition() {
    let mut environment = Environment::new();

    environment.execute("f(x)=x+1").unwrap();
    environment.execute("g(x)=x*2").unwrap();
    environment.execute("h(x)=f(x)^g(x)").unwrap();

    let outcome = environment.execute("h(3)").unwrap();

    assert!(matches!(outcome, Outcome::Value(value) if value == 4096.0));
}

#[test]
fn test_variable_can_be_assigned_a_call() {
    let mut environment = Environment::new();

    environment.execute("f(x)=x+1").unwrap();
    environment.execute("z=f(x)").unwrap();

    // z depends on x and is recomputed when x changes
    environment.execute("x=4").unwrap();

    assert_eq!(value_of(&environment, 'z'), 5.0);
}

#[test]
fn test_bare_variable_signature_is_not_a_right_side_target() {
    let mut environment = Environment::new();

    environment.execute("f(x)=x+1").unwrap();

    // The right side f(x) is a call, not a function definition target
    environment.execute("q(x)=f(x)").unwrap();

    let outcome = environment.execute("q(2)").unwrap();

    assert!(matches!(outcome, Outcome::Value(value) if value == 3.0));
}

#[test]
fn test_next_function_name_skips_taken() {
    let mut environment = Environment::new();

    assert_eq!(environment.next_function_name(), "f");

    environment.execute("f(x)=x+1").unwrap();
    assert_eq!(environment.next_function_name(), "g");

    environment.execute("g(x)=x+1").unwrap();
    assert_eq!(environment.next_function_name(), "h");
}

#[test]
fn test_next_function_name_fallback_suffix() {
    let mut environment = Environment::new();

    for name in "fghijklmnopqrstuvwxyzabcd".chars() {
        environment.execute(&format!("{}(x)=x+1", name)).unwrap();
    }

    assert_eq!(environment.next_function_name(), "f2");
}

#[test]
fn test_definitions_round_trip() {
    let mut environment = Environment::new();

    environment.execute("x=3").unwrap();
    environment.execute("f(x)=3*x^3").unwrap();

    assert_eq!(environment.variable_definitions(), vec!["x = 3"]);
    assert_eq!(environment.function_definitions(), vec!["f(x) = 3x^3"]);
}

fn eval(environment: &mut Environment, expression: &str) -> f64 {
    match environment.execute(expression).unwrap() {
        Outcome::Value(value) => value,
        _ => panic!("expected a value"),
    }
}

#[test]
fn test_unary_minus_binding() {
    let mut environment = Environment::new();

    assert_eq!(eval(&mut environment, "-3"), -3.0);
    assert_eq!(eval(&mut environment, "2*-3"), -6.0);
    assert_eq!(eval(&mut environment, "3--2"), 5.0);
    assert_eq!(eval(&mut environment, "-2^2"), -4.0);
    assert_eq!(eval(&mut environment, "2^-2"), 0.25);
    assert_eq!(eval(&mut environment, "2^3^2"), 512.0);
}

#[test]
fn test_constants() {
    let mut environment = Environment::new();

    assert_eq!(eval(&mut environment, "pi"), std::f64::consts::PI);
    assert_eq!(eval(&mut environment, "2pi"), 2.0 * std::f64::consts::PI);
    assert_eq!(eval(&mut environment, "e"), std::f64::consts::E);
    assert_eq!(eval(&mut environment, "π"), std::f64::consts::PI);
    assert_eq!(eval(&mut environment, "ln(e)"), 1.0);
    assert_eq!(eval(&mut environment, "ln(exp(2))"), 2.0);
}

#[test]
fn test_builtin_math_functions() {
    let mut environment = Environment::new();

    assert_eq!(eval(&mut environment, "sqrt(4)"), 2.0);
    assert_eq!(eval(&mut environment, "cbrt(27)"), 3.0);
    assert_eq!(eval(&mut environment, "abs(-3)"), 3.0);
    assert_eq!(eval(&mut environment, "exp(1)"), std::f64::consts::E);
    assert_eq!(eval(&mut environment, "log(1000)"), 3.0);
    assert_eq!(eval(&mut environment, "log(8,2)"), 3.0);
    assert_eq!(eval(&mut environment, "log2(1024)"), 10.0);
    assert_eq!(eval(&mut environment, "sin(pi/2)"), 1.0);
    assert_eq!(eval(&mut environment, "cos(0)"), 1.0);
    assert_eq!(
        eval(&mut environment, "atan(1)"),
        std::f64::consts::FRAC_PI_4
    );
    assert_eq!(
        eval(&mut environment, "atan2(1,1)"),
        std::f64::consts::FRAC_PI_4
    );
    assert_eq!(eval(&mut environment, "sinh(1)"), 1.0_f64.sinh());
    assert_eq!(eval(&mut environment, "tanh(0)"), 0.0);
    assert_eq!(eval(&mut environment, "floor(-1.5)"), -2.0);
    assert_eq!(eval(&mut environment, "ceil(1.2)"), 2.0);
    assert_eq!(eval(&mut environment, "round(2.5)"), 3.0);
    assert_eq!(eval(&mut environment, "round(3.14159,2)"), 314.0 / 100.0);
    assert_eq!(eval(&mut environment, "trunc(-1.9)"), -1.0);
    assert_eq!(eval(&mut environment, "sign(-4)"), -1.0);
    assert_eq!(eval(&mut environment, "min(3,7)"), 3.0);
    assert_eq!(eval(&mut environment, "max(3,7)"), 7.0);
    assert_eq!(eval(&mut environment, "mod(5,2)"), 1.0);
}

#[test]
fn test_unknown_variable_is_an_error() {
    let mut environment = Environment::new();

    let result = environment.execute("q");
    assert!(matches!(result, Err(EvalError::UnknownVariable('q'))));

    // Known variable still works
    environment.execute("q=1").unwrap();
    assert_eq!(eval(&mut environment, "sqrt(q)"), 1.0);
}

#[test]
fn test_wrong_arity_is_an_error() {
    let mut environment = Environment::new();

    assert!(matches!(
        environment.execute("atan2(1)"),
        Err(EvalError::WrongArgumentCount(_))
    ));
    assert!(matches!(
        environment.execute("sqrt(4,2)"),
        Err(EvalError::WrongArgumentCount(_))
    ));
}

#[test]
fn test_digit_function_names() {
    let mut environment = Environment::new();

    environment.execute("f2(x)=x*3").unwrap();
    assert_eq!(eval(&mut environment, "f2(4)"), 12.0);
    assert_eq!(eval(&mut environment, "log2(1024)"), 10.0);
    assert_eq!(
        eval(&mut environment, "atan2(1,1)"),
        std::f64::consts::FRAC_PI_4
    );
}

#[test]
fn test_function_source_is_canonical() {
    let mut environment = Environment::new();

    environment.execute("n=1").unwrap();
    environment.execute("g(x)=3*x+4*n^x").unwrap();
    environment.execute("h(x)=g(x)+2*(x+1)").unwrap();

    assert_eq!(
        environment.function_definitions(),
        vec!["g(x) = 3x + 4n^x", "h(x) = g(x) + 2(x + 1)"]
    );
}
