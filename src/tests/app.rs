use crate::ui::app::{App, selected_text, selection_span};
use crate::ui::input::Key;

fn run(app: &mut App, keys: &[Key]) {
    for key in keys {
        app.handle_key(*key);
    }
}

fn type_line(app: &mut App, text: &str) {
    for ch in text.chars() {
        app.handle_key(Key::Char(ch));
    }

    app.handle_key(Key::Enter);
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
    app.focus_output();

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
    app.focus_output();

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
    app.focus_output();
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('y'));

    assert!(!app.visual_active());

    let messages = app.output_texts();

    assert!(
        messages.iter().any(|line| line.contains("Copied")),
        "{:?}",
        messages
    );
}

#[test]
fn test_visual_word_motion_reports_characters() {
    let mut app = App::new();

    run(&mut app, &[Key::Char('2'), Key::Enter]);
    app.focus_output();
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
    app.focus_output();
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
    app.focus_output();
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
    app.focus_output();
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

    app.focus_output();
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

    app.focus_output();

    let frame = app.render(80, 10);

    assert!(
        frame.contains("[0;30;106m"),
        "output cursor block not visible"
    );
}

#[test]
fn test_visual_yank_with_hidden_info_uses_visible_lines() {
    let mut app = App::new();

    type_line(&mut app, "1+1");
    type_line(&mut app, "2+2");
    type_line(&mut app, "hide info");

    app.focus_output();
    app.handle_key(Key::Char('g'));
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('G'));
    app.handle_key(Key::Char('$'));
    app.handle_key(Key::Char('y'));

    let messages = app.output_texts();

    // The two drawn result lines "1+1 = 2" and "2+2 = 4" join to 15 chars
    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 15 chars to")),
        "{:?}",
        messages
    );
}

#[test]
fn test_render_focus_border_is_accented() {
    let mut app = App::new();

    let frame = app.render(80, 24);

    assert!(
        frame.contains("[0;37m"),
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
    app.focus_output();
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

    // The marked variable rows yank as "x [custom] = 2\ny [custom] = 3"
    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 29 chars to")),
        "{:?}",
        messages
    );
}

#[test]
fn test_sidebar_yank_line_without_selection_copies_value() {
    let mut app = App::new();

    type_line(&mut app, "x=2");

    app.handle_key(Key::Tab);
    app.handle_key(Key::Char('y'));

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
fn test_clear_info_keeps_only_results() {
    let mut app = App::new();

    type_line(&mut app, "1+1");
    type_line(&mut app, "zzz");
    type_line(&mut app, "clear info");

    let texts = app.output_texts();

    assert!(texts.iter().any(|line| line.contains("= 2")), "{:?}", texts);
    assert!(
        !texts.iter().any(|line| line.starts_with("> ")),
        "{:?}",
        texts
    );
    assert!(
        !texts.iter().any(|line| line.contains("[ERROR]")),
        "{:?}",
        texts
    );
}

#[test]
fn test_hide_info_hides_echo_and_errors() {
    let mut app = App::new();

    type_line(&mut app, "1+1");
    type_line(&mut app, "zzz");
    type_line(&mut app, "hide info");

    let frame = app.render(80, 24);

    assert!(frame.contains("= 2"), "result should stay visible");
    assert!(!frame.contains("> 1+1"), "echo should be hidden");
    assert!(!frame.contains("[ERROR]"), "error should be hidden");

    type_line(&mut app, "show info");

    let frame = app.render(80, 24);

    assert!(frame.contains("> 1+1"), "echo should return");
}

#[test]
fn test_sidebar_lists_builtin_constants() {
    let mut app = App::new();

    let frame = app.render(80, 24);

    assert!(frame.contains("pi [const]"), "pi missing: {frame}");
    assert!(frame.contains("e [const]"), "e missing: {frame}");
}

#[test]
fn test_setting_e_marks_it_custom() {
    let mut app = App::new();

    type_line(&mut app, "e=5");

    let frame = app.render(80, 24);

    assert!(frame.contains("e [custom] = 5"), "e value missing: {frame}");
}

#[test]
fn test_plot_constant_from_sidebar() {
    let mut app = App::new();

    // The first row of a fresh session is the constant e
    app.handle_key(Key::Tab);
    app.handle_key(Key::Char(' '));

    let texts = app.output_texts();

    assert!(
        texts.iter().any(|line| line.starts_with("plotted e =")),
        "{:?}",
        texts
    );
}

#[test]
fn test_hidden_info_keeps_result_input() {
    let mut app = App::new();

    type_line(&mut app, "1+1");
    type_line(&mut app, "hide info");

    let frame = app.render(80, 24);

    assert!(
        frame.contains("1+1 = 2"),
        "hidden echo must be folded into the result: {frame}"
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

#[test]
fn test_hide_info_keeps_definitions_in_output() {
    let mut app = App::new();

    type_line(&mut app, "f(x)=x^2");
    type_line(&mut app, "x=3");
    type_line(&mut app, "hide info");

    let frame = app.render(80, 24);

    assert!(
        frame.contains("f(x) = x^2"),
        "function definition missing: {frame}"
    );
    assert!(
        frame.contains("x = 3"),
        "variable definition missing: {frame}"
    );
    assert!(
        !frame.contains("> f(x)=x^2"),
        "echo should be hidden: {frame}"
    );
    assert!(
        !frame.contains("hide info"),
        "status line should be hidden: {frame}"
    );
}

#[test]
fn test_clear_info_keeps_definitions() {
    let mut app = App::new();

    type_line(&mut app, "g(y)=2*y");
    type_line(&mut app, "clear info");

    let texts = app.output_texts();

    assert!(
        texts.iter().any(|line| line.contains("g(y) = 2y")),
        "{:?}",
        texts
    );
    assert!(
        !texts.iter().any(|line| line.starts_with("> ")),
        "{:?}",
        texts
    );
}

#[test]
fn test_hide_info_with_expression_result_folds_origin() {
    let mut app = App::new();

    type_line(&mut app, "y=4");
    type_line(&mut app, "y^2");
    type_line(&mut app, "hide info");

    let frame = app.render(80, 24);

    assert!(frame.contains("y = 4"), "definition missing: {frame}");
    assert!(frame.contains("y^2 = 16"), "folded result missing: {frame}");
}

#[test]
fn test_hide_info_output_reads_as_notebook() {
    let mut app = App::new();

    type_line(&mut app, "f(x)=x^2");
    type_line(&mut app, "x=4");
    type_line(&mut app, "f(x)");
    type_line(&mut app, "2x+1");
    type_line(&mut app, "hide info");

    let frame = app.render(80, 24);

    for expected in ["f(x) = x^2", "x = 4", "f(x) = 16", "2x+1 = 9"] {
        assert!(frame.contains(expected), "missing {expected:?} in {frame}");
    }

    for hidden in ["[ERROR]", "> f(x)=x^2", "> hide info"] {
        assert!(
            !frame.contains(hidden),
            "{hidden:?} should be hidden: {frame}"
        );
    }
}

#[test]
fn test_equations_pane_shows_solutions_below() {
    let mut app = App::new();

    type_line(&mut app, "solve x^2=4, x");

    let frame = app.render(80, 24);

    assert!(
        frame.contains("  x^2=4"),
        "equation should be indented: {frame}"
    );
    assert!(
        frame.contains("(x = 2, x = -2)"),
        "solutions preview missing: {frame}"
    );
}

#[test]
fn test_equations_pane_yanks_equation_without_selection() {
    let mut app = App::new();

    type_line(&mut app, "solve x^2=4, x");
    app.focus_equations();
    app.handle_key(Key::Char('y'));

    let messages = app.output_texts();

    // Without a mark only the equation body is copied
    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 5 chars to")),
        "{:?}",
        messages
    );
}

#[test]
fn test_equations_pane_visual_yank_includes_solutions() {
    let mut app = App::new();

    type_line(&mut app, "solve x^2=4, x");
    app.focus_equations();
    app.handle_key(Key::Char('v'));
    app.handle_key(Key::Char('y'));

    let messages = app.output_texts();

    // The marked row reads "x^2=4 (x = 2, x = -2)" (21 chars)
    assert!(
        messages
            .iter()
            .any(|line| line.starts_with("Copied 21 chars to")),
        "{:?}",
        messages
    );
}
