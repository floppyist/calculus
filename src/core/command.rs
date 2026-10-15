// A parsed command line, shared by the interactive and the pipe front end
#[derive(Debug)]
pub enum Command {
    Help,
    Quit,
    Plot(String),
    Scales(Option<bool>),
    ClearOutput,
    ClearPlots,
    ClearVariables,
    ClearFunctions,
    ClearAll,
    ClearHistory,
    Simplify(String),
    Differentiate(String),
    Integrate {
        expression: String,
        bounds: Option<(String, String)>,
    },
    Error(String),
}

// Recognizes a command line; None means the input is an expression or assignment
pub fn parse_command(input: &str) -> Option<Command> {
    let trimmed = input.trim();

    if trimmed == "?" {
        return Some(Command::Help);
    }

    if trimmed == "quit" || trimmed == "exit" {
        return Some(Command::Quit);
    }

    if trimmed == "plot" {
        return Some(Command::Plot(String::new()));
    }

    if let Some(rest) = trimmed.strip_prefix("plot ") {
        return Some(Command::Plot(rest.trim().to_string()));
    }

    if trimmed == "scales" {
        return Some(Command::Scales(None));
    }

    if trimmed == "scales on" {
        return Some(Command::Scales(Some(true)));
    }

    if trimmed == "scales off" {
        return Some(Command::Scales(Some(false)));
    }

    if trimmed == "clear" {
        return Some(Command::ClearOutput);
    }

    if trimmed == "clear vars" {
        return Some(Command::ClearVariables);
    }

    if trimmed == "clear funcs" {
        return Some(Command::ClearFunctions);
    }

    if trimmed == "clear plots" {
        return Some(Command::ClearPlots);
    }

    if trimmed == "clear all" {
        return Some(Command::ClearAll);
    }

    if trimmed == "clear history" {
        return Some(Command::ClearHistory);
    }

    if let Some(rest) = keyword_argument(trimmed, &["simplify", "simp"]) {
        return Some(Command::Simplify(rest.to_string()));
    }

    if let Some(rest) = keyword_argument(trimmed, &["diff", "derive", "derivative"]) {
        return Some(Command::Differentiate(rest.to_string()));
    }

    if let Some(rest) = keyword_argument(trimmed, &["integrate", "integral", "int"]) {
        let parts: Vec<String> = split_top_level(rest, ',')
            .into_iter()
            .map(|part| part.trim().to_string())
            .collect();

        return Some(match parts.as_slice() {
            [expression] => Command::Integrate {
                expression: expression.clone(),
                bounds: None,
            },
            [expression, lower, upper] => Command::Integrate {
                expression: expression.clone(),
                bounds: Some((lower.clone(), upper.clone())),
            },
            _ => Command::Error(
                "[ERROR]: integrate expects '<expr>' or '<expr>, <a>, <b>'".to_string(),
            ),
        });
    }

    None
}

// Returns the argument of a leading keyword followed by a space
fn keyword_argument<'a>(input: &'a str, keywords: &[&str]) -> Option<&'a str> {
    for keyword in keywords {
        if let Some(rest) = input.strip_prefix(keyword)
            && let Some(rest) = rest.strip_prefix(' ')
        {
            let trimmed = rest.trim();

            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }

    None
}

// Splits on a separator that is not nested inside parentheses
fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;

    for (index, character) in input.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if character == separator && depth == 0 => {
                parts.push(&input[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(&input[start..]);

    parts
}
