use crate::core::environment::{Environment, EvalError, Outcome};

fn value_of(environment: &Environment, name: char) -> f64 {
    *environment.values().get(&name).expect("variable is missing")
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
    let definition = environment.functions().get("f").expect("function is missing");

    assert!(environment.function_result(definition).is_none());

    environment.execute("x=4").unwrap();

    let definition = environment.functions().get("f").expect("function is missing");

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

    for name in "fghijklmnopqrstuvwxyzabcde".chars() {
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
    assert_eq!(environment.function_definitions(), vec!["f(x) = 3*x^3"]);
}
