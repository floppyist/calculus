use crate::core::command::{self, Command, PaneArg};

#[test]
fn test_parse_help_and_quit() {
    assert!(matches!(command::parse_command("?"), Some(Command::Help)));
    assert!(matches!(
        command::parse_command("quit"),
        Some(Command::Quit)
    ));
    assert!(matches!(
        command::parse_command("exit"),
        Some(Command::Quit)
    ));
}

#[test]
fn test_parse_plot() {
    assert!(matches!(
        command::parse_command("plot"),
        Some(Command::Plot(ref s)) if s.is_empty()
    ));
    assert!(matches!(
        command::parse_command("plot sin(x)"),
        Some(Command::Plot(ref s)) if s == "sin(x)"
    ));
    assert!(matches!(
        command::parse_command("plot x^2, x^3"),
        Some(Command::Plot(ref s)) if s == "x^2, x^3"
    ));
}

#[test]
fn test_parse_scales() {
    assert!(matches!(
        command::parse_command("scales"),
        Some(Command::Scales(None))
    ));
    assert!(matches!(
        command::parse_command("scales on"),
        Some(Command::Scales(Some(true)))
    ));
    assert!(matches!(
        command::parse_command("scales off"),
        Some(Command::Scales(Some(false)))
    ));
}

#[test]
fn test_parse_clear_commands() {
    assert!(matches!(
        command::parse_command("clear"),
        Some(Command::ClearOutput)
    ));
    assert!(matches!(
        command::parse_command("clear vars"),
        Some(Command::ClearVariables)
    ));
    assert!(matches!(
        command::parse_command("clear funcs"),
        Some(Command::ClearFunctions)
    ));
    assert!(matches!(
        command::parse_command("clear plots"),
        Some(Command::ClearPlots)
    ));
    assert!(matches!(
        command::parse_command("clear all"),
        Some(Command::ClearAll)
    ));
    assert!(matches!(
        command::parse_command("clear history"),
        Some(Command::ClearHistory)
    ));
    assert!(matches!(
        command::parse_command("clear info"),
        Some(Command::ClearInfo)
    ));
}

#[test]
fn test_parse_history_restore() {
    assert!(matches!(
        command::parse_command("history"),
        Some(Command::History)
    ));
}

#[test]
fn test_parse_show_hide() {
    assert!(matches!(
        command::parse_command("show plot"),
        Some(Command::Show(PaneArg::Plot))
    ));
    assert!(matches!(
        command::parse_command("hide vars"),
        Some(Command::Hide(PaneArg::Variables))
    ));
    assert!(matches!(
        command::parse_command("show funcs"),
        Some(Command::Show(PaneArg::Functions))
    ));
    assert!(matches!(
        command::parse_command("hide info"),
        Some(Command::Hide(PaneArg::Info))
    ));
    assert!(matches!(
        command::parse_command("show all"),
        Some(Command::Show(PaneArg::All))
    ));
    assert!(matches!(
        command::parse_command("hide var"),
        Some(Command::Hide(PaneArg::Variables))
    ));
    assert!(matches!(
        command::parse_command("show fn"),
        Some(Command::Show(PaneArg::Functions))
    ));
    assert!(matches!(
        command::parse_command("hide nope"),
        Some(Command::Error(_))
    ));
    assert!(command::parse_command("showcase").is_none());
}

#[test]
fn test_parse_legacy_clear_spellings_are_gone() {
    assert!(command::parse_command("vars clear").is_none());
    assert!(command::parse_command("funcs clear").is_none());
    assert!(command::parse_command("history clear").is_none());
    assert!(command::parse_command("history -c").is_none());
    assert!(matches!(
        command::parse_command("plot clear"),
        Some(Command::Plot(_))
    ));
}

#[test]
fn test_parse_simplify() {
    assert!(matches!(
        command::parse_command("simplify x+x"),
        Some(Command::Simplify(ref s)) if s == "x+x"
    ));
    assert!(matches!(
        command::parse_command("simp 2*x"),
        Some(Command::Simplify(ref s)) if s == "2*x"
    ));
}

#[test]
fn test_parse_print() {
    assert!(matches!(
        command::parse_command("print f(x)"),
        Some(Command::Print(ref s)) if s == "f(x)"
    ));
    assert!(matches!(
        command::parse_command("print 1+1"),
        Some(Command::Print(ref s)) if s == "1+1"
    ));
    assert!(command::parse_command("printer").is_none());
}

#[test]
fn test_parse_differentiate() {
    assert!(matches!(
        command::parse_command("diff x^3"),
        Some(Command::Differentiate(ref s)) if s == "x^3"
    ));
    assert!(matches!(
        command::parse_command("derive sin(x)"),
        Some(Command::Differentiate(ref s)) if s == "sin(x)"
    ));
    assert!(matches!(
        command::parse_command("derivative ln(x)"),
        Some(Command::Differentiate(ref s)) if s == "ln(x)"
    ));
}

#[test]
fn test_parse_integrate() {
    match command::parse_command("integrate x^2") {
        Some(Command::Integrate {
            expression,
            variable,
            bounds,
        }) => {
            assert_eq!(expression, "x^2");
            assert!(variable.is_none());
            assert!(bounds.is_none());
        }
        _ => panic!("expected integrate"),
    }

    match command::parse_command("integral x^2, 0, 1") {
        Some(Command::Integrate {
            expression,
            variable,
            bounds,
        }) => {
            assert_eq!(expression, "x^2");
            assert!(variable.is_none());
            assert_eq!(bounds, Some(("0".to_string(), "1".to_string())));
        }
        _ => panic!("expected integrate with bounds"),
    }

    match command::parse_command("int sin(x), 0, pi") {
        Some(Command::Integrate {
            expression,
            variable,
            bounds,
        }) => {
            assert_eq!(expression, "sin(x)");
            assert!(variable.is_none());
            assert_eq!(bounds, Some(("0".to_string(), "pi".to_string())));
        }
        _ => panic!("expected integrate with bounds"),
    }

    match command::parse_command("int f(x,y), y") {
        Some(Command::Integrate {
            expression,
            variable,
            bounds,
        }) => {
            assert_eq!(expression, "f(x,y)");
            assert_eq!(variable, Some("y".to_string()));
            assert!(bounds.is_none());
        }
        _ => panic!("expected integrate with variable"),
    }

    match command::parse_command("int f(x,y), y, 0, 1") {
        Some(Command::Integrate {
            expression,
            variable,
            bounds,
        }) => {
            assert_eq!(expression, "f(x,y)");
            assert_eq!(variable, Some("y".to_string()));
            assert_eq!(bounds, Some(("0".to_string(), "1".to_string())));
        }
        _ => panic!("expected integrate with variable and bounds"),
    }

    match command::parse_command("integrate x^2, 0") {
        Some(Command::Integrate { variable, .. }) => assert_eq!(variable, Some("0".to_string())),
        _ => panic!("expected two-part integrate to parse as variable form"),
    }

    match command::parse_command("integrate x^2, a, b, c, d") {
        Some(Command::Error(_)) => {}
        _ => panic!("expected error for too many parts"),
    }
}

#[test]
fn test_parse_solve() {
    match command::parse_command("solve 3x=2a+2x+3, x") {
        Some(Command::Solve { equation, variable }) => {
            assert_eq!(equation, "3x=2a+2x+3");
            assert_eq!(variable, "x");
        }
        _ => panic!("expected solve"),
    }

    match command::parse_command("solve x^2-1=0, y") {
        Some(Command::Solve { equation, variable }) => {
            assert_eq!(equation, "x^2-1=0");
            assert_eq!(variable, "y");
        }
        _ => panic!("expected solve"),
    }

    assert!(matches!(
        command::parse_command("solve x^2-1=0"),
        Some(Command::Error(_))
    ));
}

#[test]
fn test_parse_equation() {
    match command::parse_command("eq x^2=4") {
        Some(Command::Equation { equation, variable }) => {
            assert_eq!(equation, "x^2=4");
            assert!(variable.is_none());
        }
        _ => panic!("expected equation"),
    }

    match command::parse_command("eq f(x)=0, y") {
        Some(Command::Equation { equation, variable }) => {
            assert_eq!(equation, "f(x)=0");
            assert_eq!(variable.as_deref(), Some("y"));
        }
        _ => panic!("expected equation with variable"),
    }

    assert!(matches!(
        command::parse_command("equation (x+1)^2=4, x"),
        Some(Command::Equation { .. })
    ));
}

#[test]
fn test_parse_integrate_respects_parentheses() {
    match command::parse_command("integrate (x+1)^2, 0, 2") {
        Some(Command::Integrate {
            expression,
            variable,
            bounds,
        }) => {
            assert_eq!(expression, "(x+1)^2");
            assert!(variable.is_none());
            assert_eq!(bounds, Some(("0".to_string(), "2".to_string())));
        }
        _ => panic!("expected integrate"),
    }
}

#[test]
fn test_parse_arithmetic_is_not_command() {
    assert!(command::parse_command("x+1").is_none());
    assert!(command::parse_command("2*x").is_none());
    assert!(command::parse_command("f(x)=3").is_none());
}
