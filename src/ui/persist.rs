use std::fs;
use std::path::Path;

// Persisted session state: definitions plus plotted curves and scale visibility
pub struct State {
    pub scales: bool,
    pub definitions: Vec<String>,
    pub plots: Vec<String>,
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

    for definition in &state.definitions {
        contents.push_str(definition);
        contents.push('\n');
    }

    for plot in &state.plots {
        contents.push_str("plot ");
        contents.push_str(plot);
        contents.push('\n');
    }

    fs::write(path, contents)
}

// Reads the session state; None if the file is missing or unreadable
pub fn load(path: &Path) -> Option<State> {
    let contents = fs::read_to_string(path).ok()?;
    let mut state = State {
        scales: false,
        definitions: Vec::new(),
        plots: Vec::new(),
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

        if let Some(rest) = line.strip_prefix("plot ") {
            let rest = rest.trim();

            if !rest.is_empty() {
                state.plots.push(rest.to_string());
            }

            continue;
        }

        state.definitions.push(line.to_string());
    }

    Some(state)
}
