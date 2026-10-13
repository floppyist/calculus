use std::collections::HashMap;

use crate::core::node::Node;
use crate::core::plot::{BRAILLE_BASE, nice_ticks, render, sample_curve, y_range};

fn x_samples(columns: usize) -> Vec<Option<f64>> {
    // The identity y = x suffices for the tests
    let node = Node::Variable('x');

    sample_curve(&node, &HashMap::new(), &HashMap::new(), columns)
}

#[test]
fn test_render_returns_expected_size() {
    let samples = x_samples(20);
    let lines = render(&[("x", &samples)], 10, 5);

    // One header line plus the actual plot area
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[1].chars().count(), 10);
}

#[test]
fn test_render_contains_braille() {
    let samples = x_samples(20);
    let lines = render(&[("x", &samples)], 10, 5);

    let has_braille = lines
        .iter()
        .any(|line| line.chars().any(|c| (c as u32) > BRAILLE_BASE));

    assert!(has_braille);
}

#[test]
fn test_render_without_finite_values() {
    // Division by zero yields inf and thus nothing plottable
    let node = Node::Div(Box::new(Node::IntNumber(1)), Box::new(Node::IntNumber(0)));
    let samples = sample_curve(&node, &HashMap::new(), &HashMap::new(), 20);
    let lines = render(&[("1/0", &samples)], 10, 5);

    assert_eq!(lines.len(), 1);
}

#[test]
fn test_y_range_spans_multiple_curves() {
    let flat = vec![Some(0.0); 4];
    let rising = vec![Some(1.0), Some(2.0), Some(3.0), Some(4.0)];

    let (y_min, y_max) = y_range(&[flat.as_slice(), rising.as_slice()]).unwrap();

    assert!(y_min < 0.0);
    assert!(y_max > 4.0);
}

#[test]
fn test_y_range_constant_gets_height() {
    let flat = vec![Some(2.0); 4];
    let (y_min, y_max) = y_range(&[flat.as_slice()]).unwrap();

    assert!(y_min < 2.0);
    assert!(y_max > 2.0);
}

#[test]
fn test_y_range_ignores_spikes() {
    let mut data: Vec<Option<f64>> = (0..=100).map(|i| Some(i as f64 / 10.0 - 5.0)).collect();

    data[10] = Some(1000.0);
    data[90] = Some(-1000.0);

    let (y_min, y_max) = y_range(&[data.as_slice()]).unwrap();

    assert!(y_min > -20.0);
    assert!(y_max < 20.0);
}

#[test]
fn test_nice_ticks_cover_range() {
    let ticks = nice_ticks(-10.0, 10.0, 5);

    assert!(ticks.len() >= 3);
    assert!(ticks.contains(&0.0));
    assert!(ticks.iter().all(|tick| *tick >= -10.0 - 1e-9 && *tick <= 10.0 + 1e-9));
}

#[test]
fn test_nice_ticks_degenerate() {
    assert!(nice_ticks(1.0, 1.0, 5).is_empty());
    assert!(nice_ticks(2.0, -2.0, 5).is_empty());
}
