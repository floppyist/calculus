use std::io::{self, IsTerminal, Write};

mod core;
#[cfg(test)]
mod tests;
mod ui;

use core::environment::{Definition, Environment};
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

        if line == "quit" || line == "exit" {
            break;
        }

        if let Some(rest) = ui::plot_target(line) {
            match environment.expression(rest) {
                Ok(node) => {
                    let (columns, rows) = terminal_size();
                    let width = columns.clamp(20, 400);
                    let height = rows.saturating_sub(4).clamp(5, 100);

                    let functions = environment.function_map();
                    let samples = core::plot::sample_curve(
                        &node,
                        &functions,
                        environment.values(),
                        width * 2,
                    );
                    let lines = core::plot::render(&[(rest, samples.as_slice())], width, height);

                    for line in lines {
                        writeln!(stdout, "{}", line)?;
                    }
                }
                Err(error) => writeln!(stdout, "{}", error)?,
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
