pub mod app;
pub mod canvas;
pub mod format;
pub mod input;
pub mod layout;
pub mod persist;
pub mod term;

// Detects a plot command and returns the expression part
pub fn plot_target(input: &str) -> Option<&str> {
    let trimmed = input.trim();

    if trimmed == "plot" {
        return Some("");
    }

    trimmed.strip_prefix("plot ").map(|rest| rest.trim())
}
