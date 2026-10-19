use crate::ui::persist::{self, State};

fn temp_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("calculus_state_{}_{}", std::process::id(), tag))
}

#[test]
fn test_state_round_trip() {
    let path = temp_path("roundtrip");
    let state = State {
        scales: true,
        timestamps: false,
        colors: false,
        hide_info: true,
        definitions: vec!["f(x) = 3*x^3".to_string(), "x = 3".to_string()],
        plots: vec!["f(x)".to_string(), "sin(x)".to_string()],
        equations: vec!["x^2=4, x".to_string(), "2x+6=0, x".to_string()],
        history: vec!["x = 3".to_string(), "plot sin(x)".to_string()],
    };

    persist::save(&path, &state).unwrap();

    let loaded = persist::load(&path).expect("state is missing");

    assert!(loaded.scales);
    assert_eq!(loaded.timestamps, state.timestamps);
    assert_eq!(loaded.colors, state.colors);
    assert_eq!(loaded.hide_info, state.hide_info);
    assert_eq!(loaded.definitions, state.definitions);
    assert_eq!(loaded.plots, state.plots);
    assert_eq!(loaded.equations, state.equations);
    assert_eq!(loaded.history, state.history);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_load_missing_file() {
    let path = temp_path("missing");

    assert!(persist::load(&path).is_none());
}
