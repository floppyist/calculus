use crate::ui::app::{App, selected_text, selection_span};
use crate::ui::input::Key;

fn run(app: &mut App, keys: &[Key]) {
    for key in keys {
        app.handle_key(*key);
    }
}

#[test]
fn test_selection_span_single_line() {
    assert_eq!(selection_span(0, 5, (0, 1), (0, 3)), Some((1, 3)));
    assert_eq!(selection_span(0, 5, (0, 3), (0, 1)), Some((1, 3)));
    assert_eq!(selection_span(1, 5, (0, 1), (0, 3)), None);
    assert_eq!(selection_span(0, 0, (0, 0), (0, 0)), None);
}

#[test]
fn test_selection_span_multiple_lines() {
    assert_eq!(selection_span(0, 6, (0, 2), (2, 1)), Some((2, 5)));
    assert_eq!(selection_span(1, 4, (0, 2), (2, 1)), Some((0, 3)));
    assert_eq!(selection_span(2, 6, (0, 2), (2, 1)), Some((0, 1)));
}

#[test]
fn test_selected_text_joins_lines() {
    let lines = vec![
        "hello".to_string(),
        "wide world".to_string(),
        "abc".to_string(),
    ];

    assert_eq!(selected_text(&lines, (0, 1), (0, 3)), "ell");
    assert_eq!(
        selected_text(&lines, (0, 1), (2, 1)),
        "ello\nwide world\nab"
    );
    assert_eq!(
        selected_text(&lines, (2, 1), (0, 1)),
        "ello\nwide world\nab"
    );
}

#[test]
fn test_visual_mode_enter_and_leave() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.handle_key(Key::Tab);

    assert!(!app.visual_active());

    app.handle_key(Key::Char('v'));
    assert!(app.visual_active());

    app.handle_key(Key::Escape);
    assert!(!app.visual_active());
}

#[test]
fn test_output_cursor_moves_without_selecting() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.handle_key(Key::Tab);

    for key in [
        Key::Char('h'),
        Key::Char('l'),
        Key::Char('w'),
        Key::Char('b'),
    ] {
        app.handle_key(key);
        assert!(!app.visual_active());
    }
}

#[test]
fn test_visual_yank_selection_reports_characters() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('y'));

    assert!(!app.visual_active());

    let messages = app.output_texts();

    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 1 char to")),
        "{:?}",
        messages
    );
}

#[test]
fn test_visual_word_motion_reports_characters() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('w'));
    app.handle_key(Key::Char('y'));

    let messages = app.output_texts();

    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 3 chars to")),
        "{:?}",
        messages
    );
}

#[test]
fn test_yank_line_without_selection() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('y'));

    assert!(!app.visual_active());

    let messages = app.output_texts();

    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 3 chars to")),
        "{:?}",
        messages
    );
}

#[test]
fn test_visual_yank_output_copies_everything() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('Y'));

    assert!(!app.visual_active());

    let messages = app.output_texts();

    assert!(
        messages
            .iter()
            .any(|line| line.contains("to the clipboard")),
        "{:?}",
        messages
    );
}

#[test]
fn test_render_draws_visual_selection() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('g'));

    let frame = app.render(80, 24);

    assert!(frame.contains("VISUAL"), "missing visual label");
    assert!(frame.contains("[0;1;93m"), "missing visual label colour");
    assert!(frame.contains("[0;30;103m"), "missing selection highlight");
}

#[test]
fn test_render_follows_visual_cursor() {
    let mut app = App::new();

    for _ in 0..40 {
        run(&mut app, &[Key::Char('1'), Key::Enter]);
    }

    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('g'));

    let frame = app.render(80, 10);

    assert!(
        frame.contains("[0;30;103m"),
        "selection scrolled out of view"
    );
}

#[test]
fn test_render_visual_cursor_starts_visible_at_bottom() {
    let mut app = App::new();

    for _ in 0..40 {
        run(&mut app, &[Key::Char('1'), Key::Enter]);
    }

    app.handle_key(Key::Tab);

    let frame = app.render(80, 10);

    assert!(
        frame.contains("[0;30;106m"),
        "output cursor block not visible"
    );
}

#[test]
fn test_render_focus_border_is_accented() {
    let mut app = App::new();

    let frame = app.render(80, 24);

    assert!(
        frame.contains("[0;34m"),
        "focused input border not accented"
    );
}

#[test]
fn test_render_selection_spans_wrapped_line() {
    let mut app = App::new();

    for ch in "111111111111111111111111111".chars() {
        app.handle_key(Key::Char(ch));
    }

    app.handle_key(Key::Enter);
    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('G'));

    let frame = app.render(24, 10);

    assert!(frame.contains("VISUAL"), "missing visual label");
    assert!(frame.contains("[0;1;93m"), "missing visual label colour");
    assert!(frame.contains("[0;30;103m"), "missing selection highlight");
}

#[test]
fn test_sidebar_visual_mode_yanks_rows() {
    let mut app = App::new();

    run(
        &mut app,
        &[Key::Char('x'), Key::Char('='), Key::Char('2'), Key::Enter],
    );
    run(
        &mut app,
        &[Key::Char('y'), Key::Char('='), Key::Char('3'), Key::Enter],
    );

    app.handle_key(Key::Tab);
    assert!(!app.visual_active());

    app.handle_key(Key::Char('v'));
    assert!(app.visual_active());

    app.handle_key(Key::Char('j'));
    app.handle_key(Key::Char('y'));

    assert!(!app.visual_active());

    let messages = app.output_texts();

    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 11 chars to")),
        "{:?}",
        messages
    );
}

#[test]
fn test_render_mode_label_follows_focus() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);

    let input_frame = app.render(80, 24);

    assert!(input_frame.contains("INSERT"), "missing input mode label");

    app.handle_key(Key::Tab);

    let output_frame = app.render(80, 24);

    assert!(output_frame.contains("NORMAL"), "missing pane mode label");
    assert!(
        !output_frame.contains("INSERT"),
        "input label should be hidden while another pane is focused"
    );
}
