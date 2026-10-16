use std::io::{self, IsTerminal, Write};

mod core;
#[cfg(test)]
mod tests;
mod ui;

use core::command::{self, Command};
use core::environment::{Definition, Environment, IntegralOutcome, PrintTarget};
use ui::app::App;
use ui::format::{format_number, format_outcome};
use ui::term::Terminal;

fn main() -> io::Result<()> {
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();

    if interactive {
        let path = ui::persist::default_path();
        let mut app = App::new();

        app.set_state_path(path.clone());

        if let Some(state) = ui::persist::load(&path) {
            app.restore(&state);
        }

        let mut terminal = Terminal::enter()?;

        app.run(&mut terminal);
        app.persist();
        terminal.restore();
        Ok(())
    } else {
        run_plain()
    }
}

// Line mode for pipes and scripts (no raw mode, no alternate screen)
fn run_plain() -> io::Result<()> {
    let mut environment = Environment::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input = String::new();

    loop {
        write!(stdout, "> ")?;
        stdout.flush()?;

        input.clear();

        // 0 means EOF (e.g. Ctrl-D or end of pipe)
        if stdin.read_line(&mut input)? == 0 {
            break;
        }

        let line = input.trim();

        if line.is_empty() {
            continue;
        }

        if let Some(command) = command::parse_command(line) {
            match command {
                Command::Quit => break,
                Command::Plot(rest) => {
                    if rest.is_empty() {
                        writeln!(stdout, "[ERROR]: plot needs an expression")?;
                    } else {
                        draw_plot(&environment, &rest, &mut stdout)?;
                    }
                }
                Command::Help => {
                    writeln!(stdout, "Help is only available in interactive mode")?;
                }
                Command::ClearVariables => {
                    environment.clear_variables();
                    writeln!(stdout, "variables cleared")?;
                }
                Command::ClearFunctions => {
                    environment.clear_functions();
                    writeln!(stdout, "functions cleared")?;
                }
                Command::ClearAll => {
                    environment.clear_variables();
                    environment.clear_functions();
                    writeln!(stdout, "cleared all")?;
                }
                Command::ClearOutput
                | Command::ClearInfo
                | Command::ClearPlots
                | Command::ClearHistory
                | Command::Show(_)
                | Command::Hide(_)
                | Command::Scales(_) => {}
                Command::Simplify(expression) => match environment.simplify(&expression) {
                    Ok(source) => writeln!(stdout, "= {}", source)?,
                    Err(error) => writeln!(stdout, "{}", error)?,
                },
                Command::Print(argument) => match environment.print_target(&argument) {
                    Ok(PrintTarget::Function {
                        name,
                        params,
                        source,
                    }) => {
                        let params: String = params
                            .iter()
                            .map(|param| param.to_string())
                            .collect::<Vec<_>>()
                            .join(",");

                        writeln!(stdout, "{}({}) = {}", name, params, source)?;
                    }
                    Ok(PrintTarget::Variable { name, value }) => {
                        writeln!(stdout, "{} = {}", name, format_number(value))?;
                    }
                    Ok(PrintTarget::Expression(outcome)) => {
                        writeln!(stdout, "{}", format_outcome(outcome))?;
                    }
                    Err(error) => writeln!(stdout, "{}", error)?,
                },
                Command::Differentiate(expression) => {
                    match environment.differentiate(&expression) {
                        Ok((name, source)) => writeln!(stdout, "{}(x) = {}", name, source)?,
                        Err(error) => writeln!(stdout, "{}", error)?,
                    }
                }
                Command::Integrate {
                    expression,
                    variable,
                    bounds,
                } => {
                    let variable_name = variable.as_deref().map(str::trim);
                    let parsed_variable = variable_name.and_then(|name| {
                        let mut chars = name.chars();

                        match (chars.next(), chars.next()) {
                            (Some(letter), None) if letter.is_ascii_alphabetic() => Some(letter),
                            _ => None,
                        }
                    });

                    if variable_name.is_some() && parsed_variable.is_none() {
                        writeln!(
                            stdout,
                            "[ERROR]: unknown integration variable '{}'",
                            variable_name.unwrap_or("")
                        )?;

                        continue;
                    }

                    let bounds = bounds
                        .as_ref()
                        .map(|(lower, upper)| (lower.as_str(), upper.as_str()));

                    match environment.integrate(&expression, bounds, parsed_variable) {
                        Ok(IntegralOutcome::Function {
                            name,
                            param,
                            source,
                        }) => {
                            writeln!(stdout, "{}({}) = {}   (+ C)", name, param, source)?;
                        }
                        Ok(IntegralOutcome::Value(value)) => {
                            writeln!(stdout, "= {}", format_number(value))?;
                        }
                        Err(error) => writeln!(stdout, "{}", error)?,
                    }
                }
                Command::Solve { equation, variable } => {
                    let parsed_variable = {
                        let mut chars = variable.trim().chars();

                        match (chars.next(), chars.next()) {
                            (Some(letter), None) if letter.is_ascii_alphabetic() => Some(letter),
                            _ => None,
                        }
                    };

                    match parsed_variable {
                        None => writeln!(
                            stdout,
                            "[ERROR]: unknown solve variable '{}'",
                            variable.trim()
                        )?,
                        Some(variable) => match environment.solve(&equation, variable) {
                            Ok(outcome) if outcome.solutions.is_empty() => {
                                writeln!(stdout, "no real solutions")?;
                            }
                            Ok(outcome) => {
                                for solution in outcome.solutions {
                                    writeln!(stdout, "{} = {}", outcome.variable, solution)?;
                                }
                            }
                            Err(error) => writeln!(stdout, "{}", error)?,
                        },
                    }
                }
                Command::Error(message) => writeln!(stdout, "{}", message)?,
            }

            continue;
        }

        match environment.execute(line) {
            Ok(outcome) => writeln!(stdout, "{}", format_outcome(outcome))?,
            Err(error) => writeln!(stdout, "{}", error)?,
        }

        draw_status(&environment, &mut stdout)?;
    }

    Ok(())
}

// Renders a curve to the terminal in plain mode
fn draw_plot(environment: &Environment, expression: &str, out: &mut impl Write) -> io::Result<()> {
    match environment.expression(expression) {
        Ok(node) => {
            let (columns, rows) = terminal_size();
            let width = columns.clamp(20, 400);
            let height = rows.saturating_sub(4).clamp(5, 100);

            let functions = environment.function_map();
            let samples =
                core::plot::sample_curve(&node, &functions, environment.values(), width * 2);
            let lines = core::plot::render(&[(expression, samples.as_slice())], width, height);

            for line in lines {
                writeln!(out, "{}", line)?;
            }
        }
        Err(error) => writeln!(out, "{}", error)?,
    }

    Ok(())
}

fn draw_status(environment: &Environment, out: &mut impl Write) -> io::Result<()> {
    let mut variables: Vec<(&char, &f64)> = environment.values().iter().collect();

    variables.sort_by_key(|(name, _)| **name);

    let variable_list = if variables.is_empty() {
        "(none)".to_string()
    } else {
        variables
            .iter()
            .map(|(name, value)| format!("{} = {}", name, format_number(**value)))
            .collect::<Vec<_>>()
            .join(", ")
    };

    writeln!(out, "Variables: {}", variable_list)?;

    let mut functions: Vec<(&String, &Definition)> = environment.functions().iter().collect();

    functions.sort_by(|a, b| a.0.cmp(b.0));

    let function_list = if functions.is_empty() {
        "(none)".to_string()
    } else {
        functions
            .iter()
            .map(|(name, definition)| format_function(name, definition, environment))
            .collect::<Vec<_>>()
            .join(", ")
    };

    writeln!(out, "Functions: {}", function_list)
}

fn format_function(name: &str, definition: &Definition, environment: &Environment) -> String {
    let params: String = definition
        .params
        .iter()
        .map(|param| param.to_string())
        .collect::<Vec<_>>()
        .join(",");

    match environment.function_result(definition) {
        Some(value) => format!(
            "{}({}) = {} (= {})",
            name,
            params,
            definition.source,
            format_number(value)
        ),
        None => format!("{}({}) = {}", name, params, definition.source),
    }
}

// Uses COLUMNS/LINES if set, otherwise a sensible default
fn terminal_size() -> (usize, usize) {
    let columns = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(80);
    let rows = std::env::var("LINES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(24);

    (columns, rows)
}
