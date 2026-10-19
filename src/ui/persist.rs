use std::fs;
use std::path::Path;

// Persisted session state: definitions, plotted curves, scale visibility and input history
pub struct State {
    pub scales: bool,
    pub timestamps: bool,
    pub colors: bool,
    pub hide_info: bool,
    pub definitions: Vec<String>,
    pub plots: Vec<String>,
    pub equations: Vec<String>,
    pub history: Vec<String>,
}

// Default state file: $CALCULUS_STATE, otherwise ./.calculus_state
pub fn default_path() -> std::path::PathBuf {
    match std::env::var("CALCULUS_STATE") {
        Ok(path) if !path.trim().is_empty() => std::path::PathBuf::from(path),
        _ => std::path::PathBuf::from(".calculus_state"),
    }
}

// Writes the session state
pub fn save(path: &Path, state: &State) -> std::io::Result<()> {
    let mut contents = String::new();

    contents.push_str(&format!("scales={}\n", if state.scales { 1 } else { 0 }));
    contents.push_str(&format!("timestamps={}\n", if state.timestamps { 1 } else { 0 }));
    contents.push_str(&format!("colors={}\n", if state.colors { 1 } else { 0 }));
    contents.push_str(&format!("hide_info={}\n", if state.hide_info { 1 } else { 0 }));

    for definition in &state.definitions {
        contents.push_str(definition);
        contents.push('\n');
    }

    for plot in &state.plots {
        contents.push_str("plot ");
        contents.push_str(plot);
        contents.push('\n');
    }

    for equation in &state.equations {
        contents.push_str("eq ");
        contents.push_str(equation);
        contents.push('\n');
    }

    for entry in &state.history {
        contents.push_str("history ");
        contents.push_str(entry);
        contents.push('\n');
    }

    fs::write(path, contents)
}

// Reads the session state; None if the file is missing or unreadable
pub fn load(path: &Path) -> Option<State> {
    let contents = fs::read_to_string(path).ok()?;
    let mut state = State {
        scales: false,
        timestamps: true,
        colors: true,
        hide_info: false,
        definitions: Vec::new(),
        plots: Vec::new(),
        equations: Vec::new(),
        history: Vec::new(),
    };

    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if let Some(value) = line.strip_prefix("scales=") {
            state.scales = value.trim() == "1";
            continue;
        }

        if let Some(value) = line.strip_prefix("timestamps=") {
            state.timestamps = value.trim() != "0";
            continue;
        }

        if let Some(value) = line.strip_prefix("colors=") {
            state.colors = value.trim() != "0";
            continue;
        }

        if let Some(value) = line.strip_prefix("hide_info=") {
            state.hide_info = value.trim() == "1";
            continue;
        }

        if let Some(rest) = line.strip_prefix("history ") {
            state.history.push(rest.to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("plot ") {
            let rest = rest.trim();

            if !rest.is_empty() {
                state.plots.push(rest.to_string());
            }

            continue;
        }

        if let Some(rest) = line.strip_prefix("eq ") {
            let rest = rest.trim();

            if !rest.is_empty() {
                state.equations.push(rest.to_string());
            }

            continue;
        }

        state.definitions.push(line.to_string());
    }

    Some(state)
}
