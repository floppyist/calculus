use crate::ui::layout::{INPUT_HEIGHT, compute};

#[test]
fn test_input_reaches_last_row() {
    let layout = compute(100, 24);

    assert_eq!(layout.input.bottom(), 24);
    assert_eq!(layout.input.height, INPUT_HEIGHT);
}

#[test]
fn test_two_column_layout() {
    let layout = compute(100, 30);

    let main = layout.main;
    let side = layout.side.expect("expected a sidebar");

    assert_eq!(main.right(), side.x);
    assert_eq!(side.right(), 100);
    assert_eq!(main.height, side.height);
    assert_eq!(main.bottom(), layout.input.y);
}

#[test]
fn test_stacked_layout_when_narrow() {
    let layout = compute(50, 20);

    let side = layout.side.expect("expected a sidebar");

    assert_eq!(side.y, layout.main.bottom());
    assert_eq!(side.bottom(), layout.input.y);
}

#[test]
fn test_small_terminal_has_no_side() {
    let layout = compute(80, 8);

    assert!(layout.side.is_none());
    assert!(layout.main.bottom() <= layout.input.y);
}
