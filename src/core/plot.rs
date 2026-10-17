use std::collections::HashMap;

use crate::core::node::{EvalContext, Node};

// Fixed x range for the 2D view
pub const X_MIN: f64 = -10.0;
pub const X_MAX: f64 = 10.0;

// Braille: base codepoint and mapping (column, row) -> bit
pub const BRAILLE_BASE: u32 = 0x2800;
pub const DOT_BITS: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

// Samples a function across the x range (one column per pixel)
pub fn sample_curve(
    function: &Node,
    functions: &HashMap<String, (Vec<char>, Node)>,
    base_variables: &HashMap<char, f64>,
    columns: usize,
) -> Vec<Option<f64>> {
    let mut samples = Vec::with_capacity(columns);

    for column in 0..columns {
        let x = if columns <= 1 {
            X_MIN
        } else {
            X_MIN + (X_MAX - X_MIN) * (column as f64) / ((columns - 1) as f64)
        };

        let mut variables = base_variables.clone();

        // A defined variable x wins over the free axis variable
        variables.entry('x').or_insert(x);

        let ctx = EvalContext {
            variables: &variables,
            functions,
            depth: 0,
        };

        let y = function.evaluate(&ctx);

        samples.push(if y.is_finite() { Some(y) } else { None });
    }

    samples
}

// Shared y range across all curves (robust to asymptote spikes)
pub fn y_range(samples: &[&[Option<f64>]]) -> Option<(f64, f64)> {
    let mut values: Vec<f64> = samples
        .iter()
        .flat_map(|curve| curve.iter().flatten().copied())
        .filter(|value| value.is_finite())
        .collect();

    if values.is_empty() {
        return None;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Tukey fences drop spikes (e.g. tan asymptotes) without clipping real data
    let q1 = percentile(&values, 0.25);
    let q3 = percentile(&values, 0.75);
    let iqr = q3 - q1;

    let mut y_min = values[0].max(q1 - 1.5 * iqr);
    let mut y_max = values[values.len() - 1].min(q3 + 1.5 * iqr);

    if (y_max - y_min).abs() < 1e-9 {
        y_min -= 1.0;
        y_max += 1.0;
    }

    let padding = (y_max - y_min) * 0.05;

    Some((y_min - padding, y_max + padding))
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let index = (fraction * (sorted.len() - 1) as f64).round() as usize;

    sorted[index.min(sorted.len() - 1)]
}

// "Nice" tick values (1/2/5 x 10^k) covering [min, max]
pub fn nice_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() || max <= min || target == 0 {
        return Vec::new();
    }

    let raw_step = (max - min) / target as f64;
    let magnitude = 10f64.powf(raw_step.log10().floor());
    let residual = raw_step / magnitude;
    let step = magnitude
        * if residual <= 1.0 {
            1.0
        } else if residual <= 2.0 {
            2.0
        } else if residual <= 5.0 {
            5.0
        } else {
            10.0
        };

    let epsilon = step * 1e-9;
    let mut ticks = Vec::new();
    let mut value = (min / step).ceil() * step;

    while value <= max + epsilon {
        ticks.push(if value.abs() < epsilon { 0.0 } else { value });
        value += step;
    }

    ticks
}

pub fn row_for(y: f64, y_min: f64, y_max: f64, pixel_height: usize) -> usize {
    let row = (y_max - y) / (y_max - y_min) * ((pixel_height - 1) as f64);

    row.round().clamp(0.0, (pixel_height - 1) as f64) as usize
}

pub fn column_for_x(x: f64, pixel_width: usize) -> usize {
    let column = (x - X_MIN) / (X_MAX - X_MIN) * ((pixel_width - 1) as f64);

    column.round().clamp(0.0, (pixel_width - 1) as f64) as usize
}

// Plain colorless text rendering (for non-interactive mode)
pub fn render(curves: &[(&str, &[Option<f64>])], width: usize, height: usize) -> Vec<String> {
    let sample_refs: Vec<&[Option<f64>]> = curves.iter().map(|(_, samples)| *samples).collect();

    let Some((y_min, y_max)) = y_range(&sample_refs) else {
        return vec!["(no finite values to plot)".to_string()];
    };

    let pixel_width = width * 2;
    let pixel_height = height * 4;
    let mut cells = vec![0u32; width * height];

    // x axis at y = 0
    if y_min <= 0.0 && y_max >= 0.0 {
        let row = row_for(0.0, y_min, y_max, pixel_height);

        for column in 0..pixel_width {
            set_pixel(&mut cells, width, height, column, row);
        }
    }

    // y axis at x = 0
    if X_MIN <= 0.0 && X_MAX >= 0.0 {
        let column = column_for_x(0.0, pixel_width);

        for row in 0..pixel_height {
            set_pixel(&mut cells, width, height, column, row);
        }
    }

    for (_, samples) in curves {
        for (column, sample) in samples.iter().enumerate() {
            let Some(y) = sample else { continue };

            // Points outside the visible range are dropped instead of being
            // clamped to the top or bottom edge (which fakes a flat line)
            if !y.is_finite() || *y < y_min || *y > y_max {
                continue;
            }

            let row = row_for(*y, y_min, y_max, pixel_height);

            set_pixel(&mut cells, width, height, column, row);
        }
    }

    let sources = curves
        .iter()
        .map(|(source, _)| *source)
        .collect::<Vec<_>>()
        .join(", ");

    let mut lines = Vec::with_capacity(height + 1);

    lines.push(format!(
        "plot {}   x=[{}, {}]   y=[{}, {}]",
        sources,
        format_number(X_MIN),
        format_number(X_MAX),
        format_number(y_min),
        format_number(y_max),
    ));

    for cell_row in 0..height {
        let mut line = String::with_capacity(width);

        for cell_column in 0..width {
            let bits = cells[cell_row * width + cell_column];

            line.push(char::from_u32(BRAILLE_BASE + bits).unwrap_or(' '));
        }

        lines.push(line);
    }

    lines
}

fn set_pixel(cells: &mut [u32], width: usize, height: usize, column: usize, row: usize) {
    let cell_column = column / 2;
    let cell_row = row / 4;

    if cell_column >= width || cell_row >= height {
        return;
    }

    cells[cell_row * width + cell_column] |= DOT_BITS[column % 2][row % 4];
}

fn format_number(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{:.2}", value)
    }
}
