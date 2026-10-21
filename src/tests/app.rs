use crate::ui::app::{App, selected_text, selection_span};
use crate::ui::input::Key;
use crate::ui::persist::State;

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

// Drops ANSI colour escapes so assertions can match visible text
fn plain(frame: &str) -> String {
    let mut out = String::new();
    let mut chars = frame.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();

                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(ch);
        }
    }

    out
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
fn test_hide_info_hides_echo_but_keeps_error_status() {
    let mut app = App::new();

    type_line(&mut app, "1+1");
    type_line(&mut app, "hide info");
    type_line(&mut app, "zzz");

    let frame = plain(&app.render(80, 24));

    assert!(frame.contains("= 2"), "result should stay visible");
    assert!(!frame.contains("> 1+1"), "echo should be hidden");
    assert!(
        frame.contains("[ERROR]"),
        "error status should stay visible"
    );

    type_line(&mut app, "show info");

    let frame = plain(&app.render(80, 24));

    assert!(frame.contains("> 1+1"), "echo should return");
}

#[test]
fn test_error_shows_in_frame_and_not_in_history() {
    let mut app = App::new();

    type_line(&mut app, "zzz");

    let frame = app.render(80, 24);

    assert!(frame.contains("[ERROR]"), "error should appear: {frame}");
    assert!(
        !app.output_texts()
            .iter()
            .any(|line| line.contains("[ERROR]")),
        "error should stay out of the history"
    );

    type_line(&mut app, "1+1");

    let frame = app.render(80, 24);

    assert!(!frame.contains("[ERROR]"), "error should clear: {frame}");
}

#[test]
fn test_empty_panes_stay_hidden_until_content() {
    let mut app = App::new();

    let frame = app.render(80, 24);

    assert!(!frame.contains("Variables"), "empty pane shown: {frame}");
    assert!(!frame.contains("Equations"), "empty pane shown: {frame}");

    type_line(&mut app, "x=2");

    let frame = app.render(80, 24);

    assert!(frame.contains("Variables"), "pane not revealed: {frame}");
}

#[test]
fn test_show_reveals_empty_pane() {
    let mut app = App::new();

    type_line(&mut app, "show vars");

    let frame = app.render(80, 24);

    assert!(frame.contains("Variables"), "pane not shown: {frame}");
}

#[test]
fn test_restore_reveals_equations_pane() {
    let mut app = App::new();
    let state = State {
        scales: false,
        timestamps: true,
        colors: true,
        hide_info: false,
        definitions: vec!["f(x) = x^2".to_string()],
        plots: Vec::new(),
        equations: vec!["x^2=4, x".to_string()],
        history: Vec::new(),
    };

    app.restore(&state);

    let frame = app.render(80, 24);

    assert!(frame.contains("Equations"), "pane not revealed: {frame}");
    assert!(frame.contains("x^2=4"), "equation missing: {frame}");
    assert!(
        frame.contains("(x = 2, x = -2)"),
        "solutions preview missing: {frame}"
    );
}

#[test]
fn test_clear_all_empties_the_equations_pane() {
    let mut app = App::new();

    type_line(&mut app, "eq x^2=4");
    type_line(&mut app, "clear all");

    let frame = plain(&app.render(80, 24));

    assert!(
        !frame.contains("(x = 2, x = -2)"),
        "clear all must remove solved equations: {frame}"
    );
}

#[test]
fn test_solved_equation_marks_the_intersection() {
    let mut app = App::new();

    type_line(&mut app, "plot x, 2");
    type_line(&mut app, "eq f(x)=g(x)");

    let frame = plain(&app.render(80, 24));

    assert!(
        frame.contains("(2,2)"),
        "intersection label missing: {frame}"
    );
}

#[test]
fn test_intersection_requires_two_plotted_sides() {
    let mut app = App::new();

    // Only one side is plotted, so no marker must appear
    type_line(&mut app, "plot x");
    type_line(&mut app, "eq f(x)=3");

    let frame = plain(&app.render(80, 24));

    assert!(
        !frame.contains("(3,3)"),
        "marker drawn without two curves: {frame}"
    );
}

#[test]
fn test_factor_command_reports_the_product() {
    let mut app = App::new();

    type_line(&mut app, "fac x^2-1");

    let frame = plain(&app.render(80, 24));

    assert!(
        frame.contains("(x + 1)(x - 1)"),
        "factor result missing: {frame}"
    );
}

#[test]
fn test_show_plot_without_curves_draws_axes() {
    let mut app = App::new();

    let frame = plain(&app.render(80, 24));

    assert!(
        !frame.contains("Plot  x:"),
        "plot pane shown by default: {frame}"
    );

    type_line(&mut app, "show plot");

    let frame = plain(&app.render(80, 24));

    assert!(
        frame.contains("Plot  x:"),
        "empty plot pane not revealed: {frame}"
    );
}

#[test]
fn test_equation_with_undefined_function_reports_error() {
    let mut app = App::new();

    type_line(&mut app, "eq f(x)=g(x)");

    let frame = plain(&app.render(80, 24));

    assert!(
        frame.contains("Function 'f' is not defined"),
        "missing undefined-function error: {frame}"
    );
}

// The rows of a rendered frame, with ANSI escapes removed
fn frame_rows(frame: &str) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut chars = frame.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();

                let mut sequence = String::new();

                for next in chars.by_ref() {
                    sequence.push(next);

                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }

                // A "row;1H" cursor move starts a new row
                if sequence.ends_with(";1H") {
                    rows.push(String::new());
                }
            }
        } else if let Some(row) = rows.last_mut() {
            row.push(ch);
        } else {
            rows.push(ch.to_string());
        }
    }

    rows
}

fn row_with(frame: &str, needle: &str) -> Option<String> {
    frame_rows(frame)
        .into_iter()
        .find(|row| row.contains(needle))
}

#[test]
fn test_error_status_line_is_indented_inside_the_border() {
    let mut app = App::new();

    type_line(&mut app, "zzz");

    let frame = app.render(80, 24);
    let row = row_with(&frame, "[ERROR]").expect("error row missing");

    assert!(
        row.starts_with("└── [ERROR]"),
        "error should be indented after the corner: {row:?}"
    );
}

#[test]
fn test_intersection_points_refresh_reactively() {
    let mut app = App::new();

    type_line(&mut app, "a=2");
    type_line(&mut app, "plot x, x/2+a");
    type_line(&mut app, "eq f(x)=g(x)");

    let frame = plain(&app.render(80, 24));

    assert!(
        frame.contains("(4,4)"),
        "first intersection missing: {frame}"
    );

    type_line(&mut app, "a=5");

    let frame = plain(&app.render(80, 24));

    assert!(
        frame.contains("(10,10)"),
        "intersection did not refresh: {frame}"
    );
}

#[test]
fn test_input_colors_a_typed_command_keyword() {
    let mut app = App::new();

    for ch in "plot x".chars() {
        app.handle_key(Key::Char(ch));
    }

    let frame = app.render(80, 24);

    assert!(
        frame.contains("\u{1b}[0;1;95mplot"),
        "command keyword not colored: {frame:?}"
    );
    assert!(
        !frame.contains("\u{1b}[0;1;95mx"),
        "the argument must stay plain: {frame:?}"
    );
}

#[test]
fn test_input_leaves_partial_keyword_plain() {
    let mut app = App::new();

    for ch in "pl".chars() {
        app.handle_key(Key::Char(ch));
    }

    let frame = app.render(80, 24);

    assert!(
        !frame.contains("\u{1b}[0;1;95m"),
        "a half-typed keyword must stay plain: {frame:?}"
    );
}

#[test]
fn test_sidebar_lists_builtin_constants() {
    let mut app = App::new();

    type_line(&mut app, "show vars");

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

    // Reveal the Variables pane; its first row is the constant e
    type_line(&mut app, "show vars");
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
fn test_clear_then_history_restores_output() {
    let mut app = App::new();

    type_line(&mut app, "2+3");
    let before = app.output_texts();

    type_line(&mut app, "clear");
    assert!(app.output_texts().is_empty(), "clear must empty the output");

    type_line(&mut app, "history");
    assert_eq!(
        app.output_texts(),
        before,
        "history must restore the output"
    );
}

#[test]
fn test_clear_history_discards_restorable_output() {
    let mut app = App::new();

    type_line(&mut app, "2+3");
    type_line(&mut app, "clear");
    type_line(&mut app, "clear history");
    type_line(&mut app, "history");

    let texts = app.output_texts();

    assert!(
        !texts.iter().any(|line| line.contains("2+3")),
        "cleared output must not come back: {texts:?}"
    );
}

#[test]
fn test_clear_history_wipes_the_screen_before_clear() {
    let mut app = App::new();

    type_line(&mut app, "2+3");
    type_line(&mut app, "clear history");

    assert!(
        !app.output_texts().iter().any(|line| line.contains("2+3")),
        "clear history must wipe the visible output"
    );

    type_line(&mut app, "history");

    let texts = app.output_texts();

    assert!(
        !texts.iter().any(|line| line.contains("2+3")),
        "wiped output must not be restorable: {texts:?}"
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

    // Timestamps are on by default; hiding them should remove the bracket prefix
    let mut stamp_visible = false;
    for hour in 0..24 {
        for minute in 0..60 {
            let pattern = format!("[{:02}:{:02}:", hour, minute);
            if frame.contains(&pattern) {
                stamp_visible = true;
                break;
            }
        }
        if stamp_visible {
            break;
        }
    }
    assert!(stamp_visible, "expected timestamps visible: {frame}");

    type_line(&mut app, "hide timestamp");

    let frame = app.render(80, 24);
    let mut stamp_visible = false;
    for hour in 0..24 {
        for minute in 0..60 {
            let pattern = format!("[{:02}:{:02}:", hour, minute);
            if frame.contains(&pattern) {
                stamp_visible = true;
                break;
            }
        }
        if stamp_visible {
            break;
        }
    }
    assert!(!stamp_visible, "expected timestamps hidden: {frame}");
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
fn test_eq_command_stores_equation_for_x() {
    let mut app = App::new();

    type_line(&mut app, "eq x^2=4");

    let frame = app.render(80, 24);

    assert!(
        frame.contains("  x^2=4"),
        "equation should be stored: {frame}"
    );
    assert!(
        frame.contains("(x = 2, x = -2)"),
        "solutions preview missing: {frame}"
    );
}

#[test]
fn test_eq_command_solves_generated_antiderivative() {
    let mut app = App::new();

    type_line(&mut app, "diff x^2");
    type_line(&mut app, "eq f(x)=0");

    let texts = app.output_texts();

    assert!(
        texts.iter().any(|line| line == "x = 0"),
        "expected the root of the generated function: {texts:?}"
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
