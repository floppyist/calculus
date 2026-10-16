use crate::core::plot::BRAILLE_BASE;
use crate::ui::canvas::{Canvas, Style};

#[test]
fn test_put_and_print_clip() {
    let mut canvas = Canvas::new(5, 2);

    canvas.print(0, 0, "hello world", Style::new());
    canvas.print_clipped(0, 1, 5, "hello world", Style::new());

    let rendered = canvas.render();

    assert!(rendered.contains("hello"));
    assert!(rendered.contains('…'));
}

#[test]
fn test_draw_box_corners() {
    let mut canvas = Canvas::new(12, 4);

    canvas.draw_box(0, 0, 12, 4, Style::new(), Some("T"));

    let rendered = canvas.render();

    assert!(rendered.contains('┌'));
    assert!(rendered.contains('┘'));
    assert!(rendered.contains('T'));
}

#[test]
fn test_put_braille_sets_bits() {
    let mut canvas = Canvas::new(1, 1);

    canvas.put_braille(0, 0, 0, 0, Style::new());

    let rendered = canvas.render();
    let ch = rendered
        .chars()
        .find(|c| (*c as u32) >= BRAILLE_BASE)
        .unwrap();

    assert_eq!(ch, char::from_u32(BRAILLE_BASE + 0x01).unwrap());
}
