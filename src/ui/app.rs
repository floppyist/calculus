use std::path::PathBuf;

use crate::core::command::{self, Command};
use crate::core::environment::{Environment, IntegralOutcome};
use crate::core::node::Node;
use crate::core::plot;
use crate::ui::canvas::{Canvas, Color, Style};
use crate::ui::clipboard;
use crate::ui::format::{format_number, format_outcome, format_scale, now_hms};
use crate::ui::input::{Key, KeyReader, LineEditor};
use crate::ui::layout::{self, Layout, Rect};
use crate::ui::persist::{self, State};
use crate::ui::term::Terminal;

const PALETTE: [Color; 6] = [
    Color::BrightRed,
    Color::BrightGreen,
    Color::BrightYellow,
    Color::BrightBlue,
    Color::BrightMagenta,
    Color::BrightCyan,
];

// Content of the help overlay; j/k or arrows scroll it
const HELP_LINES: &[&str] = &[
    "Expressions & assignments",
    "  x = 3                     assign a variable (also 3 = x)",
    "  f(x) = 3*x^3              define a function; f(2+1) works",
    "  + - * / ^   implicit multiplication: 2x, 3sin(x), (1+x)(1-x)",
    "  ^ is right associative: 2^3^2 = 512 ; unary minus: -2^2 = -4",
    "  f(x)^-1                   inverse function (sin(x)^-1 = asin(x))",
    "",
    "Constants",
    "  pi  PI  π                 circle constant 3.14159...",
    "  e                         Euler's number 2.71828...",
    "",
    "Functions",
    "  sqrt(x)                   square root",
    "  cbrt(x)                   cube root",
    "  abs(x)                    absolute value",
    "  exp(x)                    e raised to the power x",
    "  ln(x)                     natural logarithm (base e)",
    "  log(x)                    base-10 logarithm",
    "  log(x, b)                 logarithm of x to base b",
    "  log2(x)                   base-2 logarithm",
    "  sin(x)  cos(x)  tan(x)    trigonometry, argument in radians",
    "  asin(x) acos(x) atan(x)   inverse trigonometry",
    "  atan2(y, x)               angle of the point (x, y)",
    "  sinh(x) cosh(x) tanh(x)   hyperbolic functions",
    "  asinh acosh atanh         inverse hyperbolic functions",
    "  floor(x)                  round towards minus infinity",
    "  ceil(x)                   round towards plus infinity",
    "  round(x)                  round to the nearest integer",
    "  round(x, n)               round to n decimal places",
    "  trunc(x)                  drop the fractional part",
    "  sign(x)                   -1, 0 or 1 for negative, zero or positive",
    "  min(a, b)                 the smaller of two values",
    "  max(a, b)                 the larger of two values",
    "  mod(a, b)                 remainder of a divided by b",
    "",
    "Calculus",
    "  simplify <expr>           rewrite an expression in a compact form",
    "  diff <expr>               derivative d/dx, stored as a new function",
    "  integrate <expr>          antiderivative (+ C), stored as a function",
    "  integrate <expr>, a, b    definite integral from a to b",
    "  (diff and integrate act on the variable x)",
    "",
    "Plotting",
    "  plot sin(x), cos(x)       define & plot, auto named f, g, ...",
    "  plot <expr>               plot by colour index",
    "  clear plots               remove all curves",
    "  scales / scales on / off  toggle the axis labels",
    "",
    "Definitions",
    "  clear                     clear the output",
    "  clear vars                remove all variables",
    "  clear funcs               remove all functions",
    "  clear all                 wipe the session and the state file",
    "  clear history             forget the input history",
    "",
    "Editing (vi mode, Esc = normal)",
    "  h l 0 ^ $ w b             move / start / blank / end / word",
    "  i a I A                   insert before / after / start / end",
    "  x D dd                    delete char / to end / whole line",
    "  k / j                     older / newer history",
    "",
    "Read-only panes (Tab there first)",
    "  h l 0 ^ $ w b / j k       move the cursor, g / G to the top / bottom",
    "  v                         start a selection",
    "  y / Y                     yank the selection or the line / the pane",
    "  Esc / v                   cancel the selection; Esc again leaves",
    "",
    "Keys",
    "  Enter                     evaluate",
    "  Tab                       focus input / variables / functions / output",
    "  Ctrl-C                    clear the input",
    "  Ctrl-D                    quit",
    "  Ctrl-R                    paste from the clipboard",
    "  ?                         show this help",
];

#[derive(PartialEq, Eq, Clone, Copy)]
enum Focus {
    Input,
    Variables,
    Functions,
    Output,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum InputMode {
    Insert,
    Normal,
}

struct Curve {
    source: String,
    node: Node,
    color: Color,
    is_variable: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Result,
    Info,
}

struct OutputLine {
    stamp: Option<String>,
    text: String,
    style: Style,
    kind: Kind,
}

// Cursor and anchor inside the output, both as (line, character column)
struct Visual {
    anchor_line: usize,
    anchor_col: usize,
    line: usize,
    col: usize,
}

pub struct App {
    environment: Environment,
    editor: LineEditor,
    curves: Vec<Curve>,
    output: Vec<OutputLine>,
    focus: Focus,
    var_selection: usize,
    side_selection: usize,
    scroll: usize,
    help_visible: bool,
    help_scroll: usize,
    help_max_scroll: usize,
    scales_visible: bool,
    state_path: Option<PathBuf>,
    input_mode: InputMode,
    pending_d: bool,
    out_cursor: (usize, usize),
    visual: Option<Visual>,
    side_visual: Option<(usize, usize)>,
    input_visual: Option<usize>,
    hide_info: bool,
    show_plot: bool,
    show_vars: bool,
    show_funcs: bool,
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let mut app = App {
            environment: Environment::new(),
            editor: LineEditor::new(),
            curves: Vec::new(),
            output: Vec::new(),
            focus: Focus::Input,
            input_mode: InputMode::Insert,
            pending_d: false,
            var_selection: 0,
            side_selection: 0,
            scroll: 0,
            help_visible: false,
            help_scroll: 0,
            help_max_scroll: 0,
            scales_visible: false,
            state_path: None,
            out_cursor: (0, 0),
            visual: None,
            side_visual: None,
            input_visual: None,
            hide_info: false,
            show_plot: true,
            show_vars: true,
            show_funcs: true,
            should_quit: false,
        };

        app.push_info(
            hint_style(),
            "Type an expression or assignment, then press Enter.".to_string(),
        );
        app.push_info(
            hint_style(),
            "Plot one or more curves: plot sin(x), cos(x). Tab focuses the sidebar, Ctrl-D quits."
                .to_string(),
        );

        app
    }

    // Enables persistence to the given file
    pub fn set_state_path(&mut self, path: PathBuf) {
        self.state_path = Some(path);
    }

    // Applies a loaded session state
    pub fn restore(&mut self, state: &State) {
        self.scales_visible = state.scales;
        self.editor.set_history(state.history.clone());

        for definition in &state.definitions {
            let _ = self.environment.execute(definition);
        }

        if !state.plots.is_empty() {
            self.command_plot(&state.plots.join(", "));
        }
    }

    // Writes the current session state if a path is configured
    pub fn persist(&self) {
        let Some(path) = &self.state_path else {
            return;
        };

        let mut definitions = self.environment.function_definitions();
        definitions.extend(self.environment.variable_definitions());

        let state = State {
            scales: self.scales_visible,
            definitions,
            plots: self
                .curves
                .iter()
                .map(|curve| curve.source.clone())
                .collect(),
            history: self.editor.history().to_vec(),
        };

        let _ = persist::save(path, &state);
    }

    pub fn run(&mut self, terminal: &mut Terminal) {
        let mut reader = KeyReader::new();
        let mut dirty = true;
        let mut buffer = [0u8; 256];

        loop {
            if terminal.take_resized() {
                dirty = true;
            }

            if dirty {
                let (columns, rows) = terminal.size();
                let frame = self.render(columns, rows);
                let _ = terminal.present(&frame);

                dirty = false;
            }

            if self.should_quit {
                break;
            }

            match terminal.read(&mut buffer) {
                Ok(0) => {}
                Ok(count) => {
                    reader.feed(&buffer[..count]);
                    while let Some(key) = reader.next_key() {
                        self.handle_key(key);
                        dirty = true;
                        if self.should_quit {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }

    pub(crate) fn render(&mut self, columns: usize, rows: usize) -> String {
        let mut canvas = Canvas::new(columns, rows);
        let layout = layout::compute_with_side(columns, rows, self.side_visible());

        if self.help_visible {
            self.help_max_scroll = self.help_max_scroll(columns, rows);
            self.help_scroll = self.help_scroll.min(self.help_max_scroll);

            self.draw_help(
                &mut canvas,
                Rect {
                    x: 0,
                    y: 0,
                    width: columns,
                    height: layout.input.y,
                },
            );
        } else {
            self.draw_main(&mut canvas, &layout);

            if let Some(side) = layout.side {
                self.draw_side(&mut canvas, side);
            }
        }

        self.draw_input(&mut canvas, &layout);

        canvas.to_string()
    }

    fn draw_help(&self, canvas: &mut Canvas, rect: Rect) {
        canvas.draw_box(
            rect.x as isize,
            rect.y as isize,
            rect.width,
            rect.height,
            border_style(),
            Some("Help  ·  j/k scroll  ·  Esc to close"),
        );

        let inner = inner_rect(rect);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        for (row, line) in HELP_LINES
            .iter()
            .skip(self.help_scroll)
            .take(inner.height)
            .enumerate()
        {
            let style = if line.starts_with("  ") {
                value_style()
            } else {
                Style::fg(Color::BrightCyan).bold()
            };

            canvas.print_clipped(
                inner.x as isize,
                inner.y as isize + row as isize,
                inner.width,
                line,
                style,
            );
        }
    }

    fn help_max_scroll(&self, columns: usize, rows: usize) -> usize {
        let layout = layout::compute(columns, rows);
        let rect = Rect {
            x: 0,
            y: 0,
            width: columns,
            height: layout.input.y,
        };
        let inner = inner_rect(rect);

        HELP_LINES.len().saturating_sub(inner.height)
    }

    // Draws a pane frame with its title and optional "[MODE]" tag next to it
    fn draw_pane_box(&self, canvas: &mut Canvas, rect: Rect, focus: Focus, name: &str) {
        let focused = self.focus == focus;
        let border = pane_border_style(focused);

        canvas.draw_box(
            rect.x as isize,
            rect.y as isize,
            rect.width,
            rect.height,
            border,
            None,
        );

        if rect.width < 4 {
            return;
        }

        let y = rect.y as isize;
        let mut x = rect.x as isize + 2;
        let mut remaining = rect.width.saturating_sub(4);

        let used = draw_title_segment(
            canvas,
            x,
            y,
            remaining,
            &format!(" {} ", name),
            border.bold(),
        );

        x += used as isize;
        remaining = remaining.saturating_sub(used);

        let Some((mode, color)) = self.pane_mode_label(focus) else {
            return;
        };

        let used = draw_title_segment(canvas, x, y, remaining, " [", border.bold());

        x += used as isize;
        remaining = remaining.saturating_sub(used);

        let used = draw_title_segment(canvas, x, y, remaining, mode, Style::fg(color).bold());

        x += used as isize;
        remaining = remaining.saturating_sub(used);

        draw_title_segment(canvas, x, y, remaining, "] ", border.bold());
    }

    // The mode tag shown in the focused pane's title, if any
    fn pane_mode_label(&self, focus: Focus) -> Option<(&'static str, Color)> {
        if self.focus != focus {
            return None;
        }

        match focus {
            Focus::Input => Some(if self.input_visual.is_some() {
                ("VISUAL", Color::BrightYellow)
            } else {
                match self.input_mode {
                    InputMode::Insert => ("INSERT", Color::BrightRed),
                    InputMode::Normal => ("NORMAL", Color::BrightBlue),
                }
            }),
            Focus::Output => Some(if self.visual.is_some() {
                ("VISUAL", Color::BrightYellow)
            } else {
                ("NORMAL", Color::BrightBlue)
            }),
            Focus::Variables | Focus::Functions => Some(if self.side_visual.is_some() {
                ("VISUAL", Color::BrightYellow)
            } else {
                ("NORMAL", Color::BrightBlue)
            }),
        }
    }

    fn draw_input(&self, canvas: &mut Canvas, layout: &Layout) {
        let rect = layout.input;

        self.draw_pane_box(canvas, rect, Focus::Input, "Input");

        if rect.height < 3 || rect.width < 4 {
            return;
        }

        let line_y = rect.y as isize + 1;
        let prompt_x = rect.x as isize + 2;

        canvas.print(prompt_x, line_y, ">", prompt_style());

        let text_x = prompt_x + 2;
        let avail = (rect.right() as isize - text_x - 1).max(0) as usize;

        if avail == 0 {
            return;
        }

        let chars = self.editor.chars();
        let cursor = self.editor.cursor();
        let offset = if cursor >= avail {
            cursor + 1 - avail
        } else {
            0
        };

        let focused = self.focus == Focus::Input;
        let selection = if focused {
            self.input_visual.map(|anchor| {
                let end = cursor.min(chars.len().saturating_sub(1));

                (anchor.min(end), anchor.max(end))
            })
        } else {
            None
        };

        if selection.is_none() {
            let visible: String = chars.iter().skip(offset).take(avail).collect();

            canvas.print(text_x, line_y, &visible, value_style());
        } else {
            let visible = chars.iter().enumerate().skip(offset).take(avail);

            for (index, ch) in visible {
                let style =
                    if selection.is_some_and(|(begin, finish)| index >= begin && index <= finish) {
                        selected_style()
                    } else {
                        value_style()
                    };

                canvas.put(text_x + (index - offset) as isize, line_y, *ch, style);
            }
        }

        if focused {
            let cursor_index = cursor - offset;

            if cursor_index < avail {
                let ch = chars.get(cursor).copied().unwrap_or(' ');

                canvas.put(text_x + cursor_index as isize, line_y, ch, cursor_style());
            }
        }
    }

    fn draw_main(&mut self, canvas: &mut Canvas, layout: &Layout) {
        let main = layout.main;

        if main.height < 2 || main.width < 2 {
            return;
        }

        if !self.show_plot || self.curves.is_empty() {
            self.draw_output(canvas, main);
            return;
        }

        let reserved_output = 3.min(main.height.saturating_sub(3)).max(1);
        let plot_height = main.height.saturating_sub(reserved_output).max(1);
        let plot_rect = Rect {
            x: main.x,
            y: main.y,
            width: main.width,
            height: plot_height,
        };
        let output_rect = Rect {
            x: main.x,
            y: plot_rect.bottom(),
            width: main.width,
            height: main.bottom().saturating_sub(plot_rect.bottom()),
        };

        self.draw_plot(canvas, plot_rect);

        if output_rect.height >= 2 {
            self.draw_output(canvas, output_rect);
        }
    }

    fn draw_output(&mut self, canvas: &mut Canvas, rect: Rect) {
        self.draw_pane_box(canvas, rect, Focus::Output, "Output");

        let inner = inner_rect(rect);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // (stamp, chunk, line index, first character of the chunk)
        let mut wrapped: Vec<(Option<String>, String, usize, usize)> = Vec::new();

        for (line_index, line) in self.output.iter().enumerate() {
            if self.hide_info && line.kind == Kind::Info {
                continue;
            }

            for (stamp, chunk, start) in
                wrap_with_stamp(line.stamp.as_deref(), &line.text, inner.width)
            {
                wrapped.push((stamp, chunk, line_index, start));
            }
        }

        let total = wrapped.len();
        let view = inner.height;
        let max_scroll = total.saturating_sub(view);
        let mut scroll = self.scroll.min(max_scroll);

        let focused = self.focus == Focus::Output;
        let cursor = if let Some(visual) = &self.visual {
            (visual.line, visual.col)
        } else {
            self.out_cursor
        };

        // Keep the output cursor inside the visible window (scroll is offset from the bottom)
        if focused
            && let Some(target) = wrapped.iter().position(|(_, chunk, line, start)| {
                *line == cursor.0
                    && cursor.1 >= *start
                    && cursor.1 < *start + chunk.chars().count().max(1)
            })
        {
            let window_end = total - scroll;
            let window_start = window_end.saturating_sub(view);

            if target < window_start {
                scroll = total - target - view;
            } else if target >= window_end {
                scroll = total - target - 1;
            }

            scroll = scroll.min(max_scroll);
        }

        self.scroll = scroll;

        let end = total - scroll;
        let start = end.saturating_sub(view);
        let selection = if focused {
            self.visual.as_ref().map(|visual| {
                (
                    (visual.anchor_line, visual.anchor_col),
                    (visual.line, visual.col),
                )
            })
        } else {
            None
        };
        let cursor_cell = if focused { Some(cursor) } else { None };

        for (row, (stamp, text, line_index, char_start)) in wrapped[start..end].iter().enumerate() {
            let y = inner.y as isize + row as isize;
            let mut x = inner.x as isize;

            if let Some(stamp) = stamp {
                let label = format!("[{}] ", stamp);

                canvas.print(x, y, &label, hint_style());
                x += label.chars().count() as isize;
            }

            let style = self.output[*line_index].style;

            if cursor_cell.is_none() && selection.is_none() {
                canvas.print(x, y, text, style);
                continue;
            }

            let span = selection.and_then(|(anchor, cursor)| {
                let length = self.output[*line_index].text.chars().count();

                selection_span(*line_index, length, anchor, cursor)
            });

            for (offset, ch) in text.chars().enumerate() {
                let index = char_start + offset;
                let in_selection =
                    span.is_some_and(|(begin, finish)| index >= begin && index <= finish);
                let style = if cursor_cell == Some((*line_index, index)) {
                    cursor_style()
                } else if in_selection {
                    selected_style()
                } else {
                    style
                };

                canvas.put(x + offset as isize, y, ch, style);
            }
        }
    }

    fn draw_plot(&self, canvas: &mut Canvas, rect: Rect) {
        let inner = inner_rect(rect);

        if inner.width < 2 || inner.height < 2 {
            canvas.draw_box(
                rect.x as isize,
                rect.y as isize,
                rect.width,
                rect.height,
                border_style(),
                Some("Plot"),
            );
            return;
        }

        let functions = self.environment.function_map();
        let values = self.environment.values();

        let legend_height = 1;
        let gutter = if self.scales_visible {
            self.scale_gutter(inner.width, inner.height.saturating_sub(legend_height))
        } else {
            0
        };
        let label_row = if self.scales_visible && inner.height >= 5 {
            1
        } else {
            0
        };

        let data = Rect {
            x: inner.x + gutter,
            y: inner.y + legend_height,
            width: inner.width.saturating_sub(gutter),
            height: inner
                .height
                .saturating_sub(legend_height)
                .saturating_sub(label_row),
        };

        if data.width == 0 || data.height == 0 {
            canvas.draw_box(
                rect.x as isize,
                rect.y as isize,
                rect.width,
                rect.height,
                border_style(),
                Some("Plot"),
            );
            return;
        }

        let pixel_width = data.width * 2;
        let pixel_height = data.height * 4;

        let samples: Vec<Vec<Option<f64>>> = self
            .curves
            .iter()
            .map(|curve| plot::sample_curve(&curve.node, &functions, values, pixel_width))
            .collect();

        let refs: Vec<&[Option<f64>]> = samples.iter().map(|sample| sample.as_slice()).collect();
        let y_range = plot::y_range(&refs);

        let title = match y_range {
            Some((y_min, y_max)) => format!(
                "Plot  x: {} .. {}  y: {} .. {}",
                format_scale(plot::X_MIN),
                format_scale(plot::X_MAX),
                format_scale(y_min),
                format_scale(y_max)
            ),
            None => "Plot".to_string(),
        };

        canvas.draw_box(
            rect.x as isize,
            rect.y as isize,
            rect.width,
            rect.height,
            border_style(),
            Some(&title),
        );

        let mut legend_x = inner.x as isize;

        for curve in &self.curves {
            let label = format!("* {} ", curve.source);
            let width = label.chars().count();

            if legend_x + width as isize > inner.right() as isize {
                break;
            }
            canvas.print(legend_x, inner.y as isize, &label, Style::fg(curve.color));
            legend_x += width as isize;
        }

        let Some((y_min, y_max)) = y_range else {
            canvas.print(
                inner.x as isize,
                data.y as isize,
                "(no finite values)",
                hint_style(),
            );
            return;
        };

        // Curves first so the axes can skip cells a curve already occupies
        for (curve, sample) in self.curves.iter().zip(samples.iter()) {
            for (i, value) in sample.iter().enumerate() {
                if let Some(y) = value {
                    let row = plot::row_for(*y, y_min, y_max, pixel_height);

                    canvas.put_braille(
                        data.x + i / 2,
                        data.y + row / 4,
                        i % 2,
                        row % 4,
                        Style::fg(curve.color),
                    );
                }
            }
        }

        if y_min <= 0.0 && y_max >= 0.0 {
            let row = plot::row_for(0.0, y_min, y_max, pixel_height);
            let cell_y = data.y + row / 4;

            for column in 0..data.width {
                let cell_x = data.x + column;

                if !canvas.has_braille(cell_x, cell_y) {
                    canvas.put_braille(cell_x, cell_y, 0, row % 4, axis_style());
                    canvas.put_braille(cell_x, cell_y, 1, row % 4, axis_style());
                }
            }
        }

        if plot::X_MIN <= 0.0 && plot::X_MAX >= 0.0 {
            let column = plot::column_for_x(0.0, pixel_width);
            let cell_x = data.x + column / 2;

            for r in 0..data.height {
                let cell_y = data.y + r;

                if !canvas.has_braille(cell_x, cell_y) {
                    for dot in 0..4 {
                        canvas.put_braille(cell_x, cell_y, column % 2, dot, axis_style());
                    }
                }
            }
        }

        if self.scales_visible {
            self.draw_scales(canvas, inner, data, y_min, y_max, pixel_width, pixel_height);
        }
    }

    // Left gutter width for y-axis labels (0 when there is no room)
    fn scale_gutter(&self, width: usize, height: usize) -> usize {
        if height == 0 {
            return 0;
        }

        let functions = self.environment.function_map();
        let values = self.environment.values();

        let samples: Vec<Vec<Option<f64>>> = self
            .curves
            .iter()
            .map(|curve| plot::sample_curve(&curve.node, &functions, values, width.max(1) * 2))
            .collect();
        let refs: Vec<&[Option<f64>]> = samples.iter().map(|sample| sample.as_slice()).collect();

        let Some((y_min, y_max)) = plot::y_range(&refs) else {
            return 0;
        };

        let label = plot::nice_ticks(y_min, y_max, height)
            .iter()
            .map(|tick| format_scale(*tick).chars().count())
            .max()
            .unwrap_or(0);

        (label + 1).min(width.saturating_sub(4))
    }

    // Tick labels in the left gutter (y) and the bottom row (x)
    fn draw_scales(
        &self,
        canvas: &mut Canvas,
        inner: Rect,
        data: Rect,
        y_min: f64,
        y_max: f64,
        pixel_width: usize,
        pixel_height: usize,
    ) {
        for tick in plot::nice_ticks(y_min, y_max, data.height) {
            let row = plot::row_for(tick, y_min, y_max, pixel_height);
            let text = format_scale(tick);
            let x = data.x as isize - 1 - text.chars().count() as isize;

            if x >= inner.x as isize {
                canvas.print(x, data.y as isize + (row / 4) as isize, &text, hint_style());
            }
        }

        if data.bottom() >= inner.bottom() {
            return;
        }

        let label_y = data.bottom() as isize;

        for tick in plot::nice_ticks(plot::X_MIN, plot::X_MAX, (data.width / 6).max(2)) {
            let column = plot::column_for_x(tick, pixel_width);
            let center = data.x as isize + (column / 2) as isize;
            let text = format_scale(tick);
            let start = center - (text.chars().count() / 2) as isize;

            if start >= inner.x as isize {
                canvas.print(start, label_y, &text, hint_style());
            }
        }
    }

    fn draw_side(&self, canvas: &mut Canvas, rect: Rect) {
        if rect.height < 3 || rect.width < 2 {
            return;
        }

        let vars = self.variables_visible();
        let funcs = self.functions_visible();

        if !vars && !funcs {
            return;
        }

        if rect.height < 8 || !vars || !funcs {
            if vars && !funcs {
                self.draw_variables(canvas, rect);
            } else if funcs && !vars {
                self.draw_functions(canvas, rect);
            } else if self.focus == Focus::Variables {
                self.draw_variables(canvas, rect);
            } else {
                self.draw_functions(canvas, rect);
            }

            return;
        }

        let variables_height = (rect.height / 3).max(3);
        let functions_height = rect.height.saturating_sub(variables_height);

        self.draw_variables(
            canvas,
            Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: variables_height,
            },
        );

        if functions_height >= 3 {
            self.draw_functions(
                canvas,
                Rect {
                    x: rect.x,
                    y: rect.y + variables_height,
                    width: rect.width,
                    height: functions_height,
                },
            );
        }
    }

    fn draw_variables(&self, canvas: &mut Canvas, rect: Rect) {
        self.draw_pane_box(canvas, rect, Focus::Variables, "Variables");

        let inner = inner_rect(rect);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let names = self.variable_names();

        if names.is_empty() {
            canvas.print(inner.x as isize, inner.y as isize, "(none)", hint_style());
            return;
        }

        let selection = self.var_selection.min(names.len() - 1);
        let view = inner.height;
        let offset = if selection >= view {
            selection + 1 - view
        } else {
            0
        };
        let focused = self.focus == Focus::Variables;
        let range = if focused {
            self.side_visual_range()
        } else {
            None
        };

        for (row, name) in names.iter().skip(offset).take(view).enumerate() {
            let index = offset + row;
            let value = self.environment.values().get(name).copied().unwrap_or(0.0);
            let plotted = self
                .curves
                .iter()
                .any(|curve| curve.is_variable && curve.source == name.to_string());
            let marker = if plotted { "* " } else { "  " };
            let text = format!("{}{} = {}", marker, name, format_number(value));
            let in_selection = range.is_some_and(|(begin, end)| index >= begin && index <= end);
            let base = if in_selection {
                selected_style()
            } else if plotted {
                Style::fg(Color::BrightWhite)
            } else {
                value_style()
            };
            let cursor = (focused && range.is_none() && index == selection).then_some(0);

            print_side_row(
                canvas,
                inner.x as isize,
                inner.y as isize + row as isize,
                inner.width,
                &text,
                base,
                cursor,
            );
        }
    }

    fn draw_functions(&self, canvas: &mut Canvas, rect: Rect) {
        self.draw_pane_box(canvas, rect, Focus::Functions, "Functions");

        let inner = inner_rect(rect);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let names = self.function_names();

        if names.is_empty() {
            canvas.print(inner.x as isize, inner.y as isize, "(none)", hint_style());
            return;
        }

        let selection = self.side_selection.min(names.len() - 1);
        let view = inner.height;
        let offset = if selection >= view {
            selection + 1 - view
        } else {
            0
        };
        let focused = self.focus == Focus::Functions;
        let range = if focused {
            self.side_visual_range()
        } else {
            None
        };
        let mut row = 0;

        for (index, name) in names.iter().enumerate().skip(offset) {
            if row >= view {
                break;
            }

            let definition = match self.environment.functions().get(name) {
                Some(definition) => definition,
                None => continue,
            };

            let plotted = self
                .curves
                .iter()
                .any(|curve| curve.source == format!("{}(x)", name));
            let marker = if plotted { "* " } else { "  " };
            let params: String = definition
                .params
                .iter()
                .map(|param| param.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let base = format!("{}{}({}) = {}", marker, name, params, definition.source);
            let preview = match self.environment.function_result(definition) {
                Some(value) => format!("(= {})", format_number(value)),
                None => String::new(),
            };
            let in_selection = range.is_some_and(|(begin, end)| index >= begin && index <= end);
            let style = if in_selection {
                selected_style()
            } else if plotted {
                Style::fg(Color::BrightWhite)
            } else {
                value_style()
            };
            let cursor = (focused && range.is_none() && index == selection).then_some(0);

            let fits = base.chars().count() + preview.chars().count() + 2 <= inner.width;

            if preview.is_empty() || fits {
                let text = if preview.is_empty() {
                    base.clone()
                } else {
                    format!("{}  {}", base, preview)
                };

                print_side_row(
                    canvas,
                    inner.x as isize,
                    inner.y as isize + row as isize,
                    inner.width,
                    &text,
                    style,
                    cursor,
                );
                row += 1;
            } else {
                print_side_row(
                    canvas,
                    inner.x as isize,
                    inner.y as isize + row as isize,
                    inner.width,
                    &base,
                    style,
                    cursor,
                );
                row += 1;

                if row < view {
                    canvas.print_clipped(
                        inner.x as isize,
                        inner.y as isize + row as isize,
                        inner.width,
                        &preview,
                        hint_style(),
                    );
                    row += 1;
                }
            }
        }
    }

    // Inclusive row range currently selected in the focused sidebar pane
    fn side_visual_range(&self) -> Option<(usize, usize)> {
        let (anchor, cursor) = self.side_visual?;

        Some((anchor.min(cursor), anchor.max(cursor)))
    }

    pub(crate) fn handle_key(&mut self, key: Key) {
        if self.help_visible {
            match key {
                Key::Char('?') | Key::Escape | Key::Char('q') | Key::Enter => {
                    self.help_visible = false;
                }
                Key::Char('j') | Key::Down => {
                    self.help_scroll = (self.help_scroll + 1).min(self.help_max_scroll);
                }
                Key::Char('k') | Key::Up => {
                    self.help_scroll = self.help_scroll.saturating_sub(1);
                }
                Key::Char('g') | Key::Home => self.help_scroll = 0,
                Key::Char('G') | Key::End => self.help_scroll = self.help_max_scroll,
                Key::Char(' ') | Key::PageDown => {
                    self.help_scroll = (self.help_scroll + 10).min(self.help_max_scroll);
                }
                Key::PageUp => {
                    self.help_scroll = self.help_scroll.saturating_sub(10);
                }
                _ => {}
            }

            return;
        }

        if key == Key::Tab {
            self.cycle_focus();
            return;
        }

        match self.focus {
            Focus::Input => self.handle_input_key(key),
            Focus::Output => {
                if self.visual.is_some() {
                    self.handle_visual_key(key);
                } else {
                    self.handle_output_key(key);
                }
            }
            Focus::Variables | Focus::Functions => self.handle_side_key(key),
        }
    }

    fn handle_input_key(&mut self, key: Key) {
        match self.input_mode {
            InputMode::Insert => self.handle_insert_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_insert_key(&mut self, key: Key) {
        // '?' opens the help overlay directly, without pressing Enter first
        if key == Key::Char('?') {
            self.help_visible = true;
            self.help_scroll = 0;
            return;
        }

        match key {
            Key::Char(ch) => self.editor.insert(ch),
            Key::Enter => self.submit(),
            Key::Backspace => self.editor.backspace(),
            Key::Delete => self.editor.delete(),
            Key::Left => self.editor.move_left(),
            Key::Right => self.editor.move_right(),
            Key::Home => self.editor.home(),
            Key::End => self.editor.end(),
            Key::Up => self.editor.history_prev(),
            Key::Down => self.editor.history_next(),
            Key::PageUp => self.scroll = self.scroll.saturating_add(5),
            Key::PageDown => self.scroll = self.scroll.saturating_sub(5),
            Key::CtrlR => {}
            Key::CtrlL => {}
            Key::CtrlC => {
                self.editor.clear();
                self.input_mode = InputMode::Insert;
            }
            Key::CtrlD => {
                if self.editor.is_empty() {
                    self.persist();
                    self.should_quit = true;
                } else {
                    self.editor.delete();
                }
            }
            Key::Escape => {
                self.input_mode = InputMode::Normal;
                self.pending_d = false;
                self.editor.clamp_normal();
            }
            Key::Unknown | Key::Tab => {}
        }
    }

    fn handle_normal_key(&mut self, key: Key) {
        if self.input_visual.is_some() && self.handle_input_visual_key(key) {
            return;
        }

        self.input_visual = None;

        // 'dd' clears the line; any other key cancels the pending d
        if key == Key::Char('d') {
            if self.pending_d {
                self.editor.clear();
                self.pending_d = false;
            } else {
                self.pending_d = true;
            }

            return;
        }

        self.pending_d = false;

        match key {
            Key::Char('h') | Key::Left => {
                self.editor.move_left();
                self.editor.clamp_normal();
            }
            Key::Char('l') | Key::Right => {
                self.editor.move_right();
                self.editor.clamp_normal();
            }
            Key::Char('0') | Key::Home => self.editor.home(),
            Key::Char('^') => self.editor.first_nonblank(),
            Key::Char('$') | Key::End => {
                self.editor.end();
                self.editor.clamp_normal();
            }
            Key::Char('w') => {
                self.editor.move_word_forward();
                self.editor.clamp_normal();
            }
            Key::Char('b') => self.editor.move_word_back(),
            Key::Char('x') | Key::Delete => {
                self.editor.delete();
                self.editor.clamp_normal();
            }
            Key::Char('D') => {
                self.editor.delete_to_end();
                self.editor.clamp_normal();
            }
            Key::Char('i') => self.input_mode = InputMode::Insert,
            Key::Char('?') => {
                self.help_visible = true;
                self.help_scroll = 0;
            }
            Key::Char('I') => {
                self.editor.home();
                self.input_mode = InputMode::Insert;
            }
            Key::Char('a') => {
                self.editor.move_right();
                self.input_mode = InputMode::Insert;
            }
            Key::Char('A') => {
                self.editor.end();
                self.input_mode = InputMode::Insert;
            }
            Key::Char('v') => self.start_input_visual(),
            Key::Char('y') | Key::Char('Y') => self.yank_input_line(),
            Key::Char('p') => self.paste_clipboard(),
            Key::Char('k') | Key::Up => {
                self.editor.history_prev();
                self.editor.clamp_normal();
            }
            Key::Char('j') | Key::Down => {
                self.editor.history_next();
                self.editor.clamp_normal();
            }
            Key::Enter => self.submit(),
            Key::Backspace => self.editor.move_left(),
            Key::CtrlC => {
                self.editor.clear();
                self.input_mode = InputMode::Insert;
            }
            Key::CtrlD => {
                if self.editor.is_empty() {
                    self.persist();
                    self.should_quit = true;
                }
            }
            Key::PageUp => self.scroll = self.scroll.saturating_add(5),
            Key::PageDown => self.scroll = self.scroll.saturating_sub(5),
            _ => {}
        }
    }

    fn handle_output_key(&mut self, key: Key) {
        match key {
            Key::Char('h') | Key::Left => self.out_move_left(),
            Key::Char('l') | Key::Right => self.out_move_right(),
            Key::Char('j') | Key::Down => self.out_move_line(1),
            Key::Char('k') | Key::Up => self.out_move_line(-1),
            Key::Char('w') => self.out_word(true),
            Key::Char('b') => self.out_word(false),
            Key::Char('0') | Key::Home => self.out_column(0),
            Key::Char('^') => self.out_first_nonblank(),
            Key::Char('$') | Key::End => self.out_column(usize::MAX),
            Key::Char('g') => self.out_jump(0, false),
            Key::Char('G') => self.out_jump(usize::MAX, true),
            Key::Char('v') => self.start_visual(),
            Key::Char('y') => self.yank_line(),
            Key::Char('Y') => self.yank_output(),
            Key::Char('?') => {
                self.help_visible = true;
                self.help_scroll = 0;
            }
            Key::Escape => {
                self.focus = Focus::Input;
                self.input_mode = InputMode::Insert;
            }
            Key::CtrlC => {
                self.focus = Focus::Input;
                self.input_mode = InputMode::Insert;
                self.editor.clear();
            }
            Key::CtrlD => self.should_quit = true,
            Key::PageUp => self.scroll = self.scroll.saturating_add(5),
            Key::PageDown => self.scroll = self.scroll.saturating_sub(5),
            _ => {}
        }
    }

    fn handle_visual_key(&mut self, key: Key) {
        match key {
            Key::Char('y') => self.yank_selection(),
            Key::Char('Y') => self.yank_output(),
            Key::Escape | Key::Char('v') | Key::CtrlC => self.exit_visual(),
            Key::Char('?') => {
                self.help_visible = true;
                self.help_scroll = 0;
            }
            Key::Char('h') | Key::Left => self.out_move_left(),
            Key::Char('l') | Key::Right => self.out_move_right(),
            Key::Char('j') | Key::Down => self.out_move_line(1),
            Key::Char('k') | Key::Up => self.out_move_line(-1),
            Key::Char('w') => self.out_word(true),
            Key::Char('b') => self.out_word(false),
            Key::Char('0') | Key::Home => self.out_column(0),
            Key::Char('^') => self.out_first_nonblank(),
            Key::Char('$') | Key::End => self.out_column(usize::MAX),
            Key::Char('g') => self.out_jump(0, false),
            Key::Char('G') => self.out_jump(usize::MAX, true),
            _ => {}
        }

        self.sync_visual_cursor();
    }

    // Keeps the selection cursor in step with the output cursor
    fn sync_visual_cursor(&mut self) {
        if let Some(visual) = self.visual.as_mut() {
            visual.line = self.out_cursor.0;
            visual.col = self.out_cursor.1;
        }
    }

    fn start_visual(&mut self) {
        if self.output.is_empty() {
            return;
        }

        self.clamp_out_cursor();

        let (line, col) = self.out_cursor;

        self.visual = Some(Visual {
            anchor_line: line,
            anchor_col: col,
            line,
            col,
        });
    }

    fn exit_visual(&mut self) {
        self.visual = None;
    }

    fn start_input_visual(&mut self) {
        if self.editor.is_empty() {
            return;
        }

        self.editor.clamp_normal();
        self.input_visual = Some(self.editor.cursor());
    }

    // Returns true when the key belongs to the input selection
    fn handle_input_visual_key(&mut self, key: Key) -> bool {
        match key {
            Key::Char('h') | Key::Left => {
                self.editor.move_left();
                self.editor.clamp_normal();
            }
            Key::Char('l') | Key::Right => {
                self.editor.move_right();
                self.editor.clamp_normal();
            }
            Key::Char('0') | Key::Home => self.editor.home(),
            Key::Char('^') => self.editor.first_nonblank(),
            Key::Char('$') | Key::End => {
                self.editor.end();
                self.editor.clamp_normal();
            }
            Key::Char('w') => {
                self.editor.move_word_forward();
                self.editor.clamp_normal();
            }
            Key::Char('b') => self.editor.move_word_back(),
            Key::Char('y') => self.yank_input_selection(),
            Key::Char('Y') => self.yank_input_line(),
            Key::Char('v') | Key::Escape | Key::CtrlC => self.input_visual = None,
            Key::Char('?') => {
                self.help_visible = true;
                self.help_scroll = 0;
            }
            _ => return false,
        }

        true
    }

    fn yank_input_selection(&mut self) {
        let Some(anchor) = self.input_visual.take() else {
            return;
        };

        let chars = self.editor.chars().to_vec();

        if chars.is_empty() {
            self.copy_to_clipboard(String::new());
            return;
        }

        let end = self.editor.cursor().min(chars.len() - 1);
        let (begin, end) = (anchor.min(end), anchor.max(end));
        let text: String = chars[begin..=end].iter().collect();

        self.copy_to_clipboard(text);
    }

    fn yank_input_line(&mut self) {
        self.input_visual = None;

        let text = self.editor.text();

        self.copy_to_clipboard(text);
    }

    // Keeps the output cursor on an existing character
    fn clamp_out_cursor(&mut self) {
        if self.output.is_empty() {
            self.out_cursor = (0, 0);
            return;
        }

        let line = self.out_cursor.0.min(self.output.len() - 1);
        let col = self
            .out_cursor
            .1
            .min(line_length(&self.output, line).saturating_sub(1));

        self.out_cursor = (line, col);
    }

    fn out_move_left(&mut self) {
        let (mut line, mut col) = self.out_cursor;

        if col > 0 {
            col -= 1;
        } else if line > 0 {
            line -= 1;
            col = line_length(&self.output, line).saturating_sub(1);
        }

        self.out_cursor = (line, col);
    }

    fn out_move_right(&mut self) {
        let (mut line, mut col) = self.out_cursor;
        let length = line_length(&self.output, line);

        if col + 1 < length {
            col += 1;
        } else if line + 1 < self.output.len() {
            line += 1;
            col = 0;
        } else {
            col = length.saturating_sub(1);
        }

        self.out_cursor = (line, col);
    }

    fn out_move_line(&mut self, delta: isize) {
        if self.output.is_empty() {
            return;
        }

        let (line, col) = self.out_cursor;
        let last = self.output.len() - 1;
        let line = if delta < 0 {
            line.saturating_sub(delta.unsigned_abs())
        } else {
            (line + delta as usize).min(last)
        };
        let col = col.min(line_length(&self.output, line).saturating_sub(1));

        self.out_cursor = (line, col);
    }

    fn out_word(&mut self, forward: bool) {
        let cells: Vec<(usize, usize, char)> = self
            .output
            .iter()
            .enumerate()
            .flat_map(|(line, entry)| {
                entry
                    .text
                    .chars()
                    .enumerate()
                    .map(move |(col, ch)| (line, col, ch))
            })
            .collect();
        let Some(mut position) = cells
            .iter()
            .position(|&(line, col, _)| (line, col) == self.out_cursor)
        else {
            return;
        };

        if forward {
            while position < cells.len() && !cells[position].2.is_whitespace() {
                position += 1;
            }

            while position < cells.len() && cells[position].2.is_whitespace() {
                position += 1;
            }

            if position >= cells.len() {
                position = cells.len() - 1;
            }
        } else {
            while position > 0 && cells[position].2.is_whitespace() {
                position -= 1;
            }

            while position > 0 && !cells[position - 1].2.is_whitespace() {
                position -= 1;
            }
        }

        let (line, col, _) = cells[position];

        self.out_cursor = (line, col);
    }

    fn out_column(&mut self, col: usize) {
        let line = self.out_cursor.0;

        self.out_cursor.1 = col.min(line_length(&self.output, line).saturating_sub(1));
    }

    fn out_first_nonblank(&mut self) {
        let line = self.out_cursor.0;

        if let Some(entry) = self.output.get(line) {
            let column = entry.text.chars().position(|ch| !ch.is_whitespace());

            self.out_cursor.1 = column.unwrap_or(0);
        }
    }

    fn out_jump(&mut self, line: usize, to_end: bool) {
        let last = self.output.len().saturating_sub(1);
        let line = line.min(last);

        self.out_cursor = (
            line,
            if to_end {
                line_length(&self.output, line).saturating_sub(1)
            } else {
                0
            },
        );
    }

    fn yank_selection(&mut self) {
        let Some(visual) = self.visual.take() else {
            return;
        };

        let lines: Vec<String> = self.output.iter().map(|line| line.text.clone()).collect();
        let text = selected_text(
            &lines,
            (visual.anchor_line, visual.anchor_col),
            (visual.line, visual.col),
        );

        self.copy_to_clipboard(text);
    }

    // 'y' without a selection copies the current output line
    fn yank_line(&mut self) {
        self.clamp_out_cursor();

        let Some(entry) = self.output.get(self.out_cursor.0) else {
            return;
        };

        self.copy_to_clipboard(entry.text.clone());
    }

    fn yank_output(&mut self) {
        let text: Vec<String> = self.output.iter().map(|line| line.text.clone()).collect();
        let text = text.join("\n");

        self.exit_visual();
        self.copy_to_clipboard(text);
    }

    fn copy_to_clipboard(&mut self, text: String) {
        if text.is_empty() {
            self.push_info(hint_style(), "Nothing to copy".to_string());
            return;
        }

        let count = text.chars().count();

        if clipboard::copy(&text) {
            let unit = if count == 1 { "char" } else { "chars" };

            self.push_info(
                hint_style(),
                format!("Copied {} {} to the clipboard", count, unit),
            );
        } else {
            self.push_info(error_style(), "Clipboard is not available".to_string());
        }
    }

    fn paste_clipboard(&mut self) {
        let Some(text) = clipboard::paste() else {
            self.push_info(error_style(), "Clipboard is not available".to_string());
            return;
        };

        // The input line is single-line, so flatten any whitespace from the paste
        let flattened: String = text
            .chars()
            .map(|ch| if ch.is_control() { ' ' } else { ch })
            .collect();

        self.editor.insert_str(&flattened);
    }

    fn handle_side_key(&mut self, key: Key) {
        if self.side_visual.is_some() {
            self.handle_side_visual_key(key);
            return;
        }

        let total = self.side_len();

        match key {
            Key::Up | Key::Char('k') => {
                if total > 0 {
                    let selection = self.current_selection_mut();
                    *selection = selection.saturating_sub(1);
                }
            }
            Key::Down | Key::Char('j') => {
                if total > 0 {
                    let selection = self.current_selection_mut();
                    *selection = (*selection + 1).min(total - 1);
                }
            }
            Key::Char('g') | Key::Home => *self.current_selection_mut() = 0,
            Key::Char('G') | Key::End => {
                if total > 0 {
                    *self.current_selection_mut() = total - 1;
                }
            }
            Key::Char('v') => self.start_side_visual(),
            Key::Char('y') => self.yank_side_line(),
            Key::Char(' ') | Key::Enter => self.toggle_selected(),
            Key::Char('d') | Key::Delete => self.delete_selected(),
            Key::Char('?') => {
                self.help_visible = true;
                self.help_scroll = 0;
            }
            Key::Escape => {
                self.focus = Focus::Input;
                self.input_mode = InputMode::Insert;
            }
            Key::CtrlD => self.should_quit = true,
            Key::CtrlC => {
                self.focus = Focus::Input;
                self.input_mode = InputMode::Insert;
                self.editor.clear();
            }
            _ => {}
        }
    }

    fn handle_side_visual_key(&mut self, key: Key) {
        let total = self.side_len();

        match key {
            Key::Up | Key::Char('k') => {
                let selection = self.current_selection_mut();
                *selection = selection.saturating_sub(1);
            }
            Key::Down | Key::Char('j') => {
                if total > 0 {
                    let selection = self.current_selection_mut();
                    *selection = (*selection + 1).min(total - 1);
                }
            }
            Key::Char('g') | Key::Home => *self.current_selection_mut() = 0,
            Key::Char('G') | Key::End => {
                if total > 0 {
                    *self.current_selection_mut() = total - 1;
                }
            }
            Key::Char('y') => self.yank_side_selection(),
            Key::Escape | Key::Char('v') | Key::CtrlC => self.side_visual = None,
            Key::Char('?') => {
                self.help_visible = true;
                self.help_scroll = 0;
            }
            _ => {}
        }

        self.sync_side_visual();
    }

    // Keeps the selection cursor in step with the sidebar selection
    fn sync_side_visual(&mut self) {
        if self.side_visual.is_some() {
            let row = *self.current_selection();

            if let Some(visual) = self.side_visual.as_mut() {
                visual.1 = row;
            }
        }
    }

    fn start_side_visual(&mut self) {
        let total = self.side_len();

        if total == 0 {
            return;
        }

        let row = (*self.current_selection()).min(total - 1);

        *self.current_selection_mut() = row;
        self.side_visual = Some((row, row));
    }

    fn current_selection(&self) -> &usize {
        match self.focus {
            Focus::Variables => &self.var_selection,
            _ => &self.side_selection,
        }
    }

    fn current_selection_mut(&mut self) -> &mut usize {
        match self.focus {
            Focus::Variables => &mut self.var_selection,
            _ => &mut self.side_selection,
        }
    }

    fn side_len(&self) -> usize {
        match self.focus {
            Focus::Variables => self.variable_names().len(),
            Focus::Functions => self.function_names().len(),
            _ => 0,
        }
    }

    // The plain text shown for one sidebar row, without the plot marker
    fn side_line_text(&self, index: usize) -> Option<String> {
        match self.focus {
            Focus::Variables => {
                let names = self.variable_names();
                let name = names.get(index)?;
                let value = self.environment.values().get(name).copied().unwrap_or(0.0);

                Some(format!("{} = {}", name, format_number(value)))
            }
            Focus::Functions => {
                let names = self.function_names();
                let name = names.get(index)?;
                let definition = self.environment.functions().get(name)?;
                let params: String = definition
                    .params
                    .iter()
                    .map(|param| param.to_string())
                    .collect::<Vec<_>>()
                    .join(",");

                Some(format!("{}({}) = {}", name, params, definition.source))
            }
            Focus::Input | Focus::Output => None,
        }
    }

    fn yank_side_line(&mut self) {
        let total = self.side_len();

        if total == 0 {
            return;
        }

        let index = (*self.current_selection()).min(total - 1);

        if let Some(text) = self.side_line_text(index) {
            self.copy_to_clipboard(text);
        }
    }

    fn yank_side_selection(&mut self) {
        let Some((anchor, cursor)) = self.side_visual.take() else {
            return;
        };

        let (begin, end) = (anchor.min(cursor), anchor.max(cursor));
        let mut rows = Vec::new();

        for index in begin..=end {
            if let Some(text) = self.side_line_text(index) {
                rows.push(text);
            }
        }

        self.copy_to_clipboard(rows.join("\n"));
    }

    fn focusables(&self) -> Vec<Focus> {
        let mut panes = vec![Focus::Input];

        if self.variables_visible() {
            panes.push(Focus::Variables);
        }

        if self.functions_visible() {
            panes.push(Focus::Functions);
        }

        panes.push(Focus::Output);

        panes
    }

    fn cycle_focus(&mut self) {
        self.visual = None;
        self.side_visual = None;
        self.input_visual = None;

        let panes = self.focusables();
        let next = match panes.iter().position(|pane| *pane == self.focus) {
            Some(index) => panes[(index + 1) % panes.len()],
            None => Focus::Input,
        };

        self.focus = next;
        self.enter_focus();
    }

    fn enter_focus(&mut self) {
        match self.focus {
            Focus::Input => {
                self.input_mode = InputMode::Insert;
                self.input_visual = None;
            }
            Focus::Output => {
                if !self.output.is_empty() {
                    self.out_cursor = (self.output.len() - 1, 0);
                }
            }
            Focus::Variables | Focus::Functions => {}
        }
    }

    fn variables_visible(&self) -> bool {
        self.show_vars && !self.environment.values().is_empty()
    }

    fn functions_visible(&self) -> bool {
        self.show_funcs && !self.environment.functions().is_empty()
    }

    fn side_visible(&self) -> bool {
        self.variables_visible() || self.functions_visible()
    }

    fn toggle_selected(&mut self) {
        match self.focus {
            Focus::Variables => self.toggle_variable(),
            Focus::Functions => self.toggle_function(),
            Focus::Input | Focus::Output => {}
        }

        self.persist();
    }

    fn toggle_variable(&mut self) {
        let names = self.variable_names();

        if names.is_empty() {
            return;
        }

        let index = self.var_selection.min(names.len() - 1);
        let name = names[index];
        let source = name.to_string();

        if let Some(position) = self.curves.iter().position(|curve| curve.source == source) {
            self.curves.remove(position);
            self.push_output(result_style(), format!("removed plot {}", source));
            return;
        }

        let value = match self.environment.values().get(&name) {
            Some(value) => *value,
            None => return,
        };
        let used: Vec<Color> = self.curves.iter().map(|curve| curve.color).collect();
        let color = pick_color(&used);

        self.curves.push(Curve {
            source: source.clone(),
            node: Node::FloatNumber(value),
            color,
            is_variable: true,
        });
        let message = format!("plotted {} = {}", name, format_number(value));

        self.push_output(result_style(), message);
    }

    fn toggle_function(&mut self) {
        let names = self.function_names();

        if names.is_empty() {
            return;
        }

        let index = self.side_selection.min(names.len() - 1);
        let name = names[index].clone();

        let params = match self.environment.functions().get(&name) {
            Some(definition) => definition.params.clone(),
            None => return,
        };

        if params.len() != 1 {
            self.push_output(
                hint_style(),
                format!("{} needs exactly one parameter to plot", name),
            );
            return;
        }

        let source = format!("{}(x)", name);

        if let Some(position) = self.curves.iter().position(|curve| curve.source == source) {
            self.curves.remove(position);
            self.push_output(result_style(), format!("removed plot {}", source));
            return;
        }

        let used: Vec<Color> = self.curves.iter().map(|curve| curve.color).collect();
        let color = pick_color(&used);

        self.curves.push(Curve {
            source: source.clone(),
            node: Node::Call(name.clone(), vec![Node::Variable('x')]),
            color,
            is_variable: false,
        });
        self.push_output(result_style(), format!("plotted {}", source));
    }

    fn delete_selected(&mut self) {
        match self.focus {
            Focus::Variables => self.delete_variable(),
            Focus::Functions => self.delete_function(),
            Focus::Input | Focus::Output => {}
        }

        self.persist();
    }

    fn delete_variable(&mut self) {
        let names = self.variable_names();

        if names.is_empty() {
            return;
        }

        let index = self.var_selection.min(names.len() - 1);
        let name = names[index];

        if self.environment.remove_variable(name) {
            self.curves.retain(|curve| curve.source != name.to_string());
            self.push_output(result_style(), format!("{} deleted", name));

            let remaining = self.variable_names().len();

            if remaining == 0 {
                self.var_selection = 0;
            } else {
                self.var_selection = self.var_selection.min(remaining - 1);
            }
        }
    }

    fn delete_function(&mut self) {
        let names = self.function_names();

        if names.is_empty() {
            return;
        }

        let index = self.side_selection.min(names.len() - 1);
        let name = names[index].clone();

        if self.environment.remove_function(&name) {
            self.curves
                .retain(|curve| curve.source != format!("{}(x)", name));
            self.push_output(result_style(), format!("{} deleted", name));

            let remaining = self.function_names().len();

            if remaining == 0 {
                self.side_selection = 0;
            } else {
                self.side_selection = self.side_selection.min(remaining - 1);
            }
        }
    }

    fn submit(&mut self) {
        let line = self.editor.text();
        let trimmed = line.trim().to_string();

        self.editor.push_history(trimmed.clone());
        self.editor.clear();
        self.scroll = 0;
        self.input_mode = InputMode::Insert;
        self.pending_d = false;

        if trimmed.is_empty() {
            return;
        }

        if trimmed == "?" {
            self.help_visible = true;
            self.help_scroll = 0;
            return;
        }

        self.push_output(hint_style(), format!("> {}", trimmed));

        if let Some(command) = command::parse_command(&trimmed) {
            self.run_command(command);
            return;
        }

        match self.environment.execute(&trimmed) {
            Ok(outcome) => self.push_output(result_style(), format_outcome(outcome)),
            Err(error) => self.push_info(error_style(), format!("{}", error)),
        }

        self.persist();
    }

    fn run_command(&mut self, command: Command) {
        match command {
            Command::Help => {
                self.help_visible = true;
                self.help_scroll = 0;
            }
            Command::Quit => {
                self.persist();
                self.should_quit = true;
            }
            Command::Plot(rest) => {
                self.command_plot(&rest);
                self.persist();
            }
            Command::Scales(value) => {
                self.command_scales(value);
                self.persist();
            }
            Command::ClearOutput => {
                self.command_clear_output();
            }
            Command::ClearPlots => {
                self.curves.clear();
                self.push_output(result_style(), "plot cleared".to_string());
                self.persist();
            }
            Command::ClearVariables => {
                self.command_clear_variables();
                self.persist();
            }
            Command::ClearFunctions => {
                self.command_clear_functions();
                self.persist();
            }
            Command::ClearAll => self.command_clear_all(),
            Command::ClearHistory => {
                self.command_clear_history();
                self.persist();
            }
            Command::Simplify(expression) => self.command_simplify(&expression),
            Command::Differentiate(expression) => {
                self.command_differentiate(&expression);
                self.persist();
            }
            Command::Integrate { expression, bounds } => {
                self.command_integrate(&expression, bounds);
                self.persist();
            }
            Command::Error(message) => self.push_info(error_style(), message),
        }
    }

    fn command_simplify(&mut self, expression: &str) {
        match self.environment.simplify(expression) {
            Ok(source) => self.push_output(result_style(), format!("= {}", source)),
            Err(error) => self.push_info(error_style(), format!("{}", error)),
        }
    }

    fn command_differentiate(&mut self, expression: &str) {
        match self.environment.differentiate(expression) {
            Ok((name, source)) => {
                self.push_output(result_style(), format!("{}(x) = {}", name, source));
            }
            Err(error) => self.push_info(error_style(), format!("{}", error)),
        }
    }

    fn command_integrate(&mut self, expression: &str, bounds: Option<(String, String)>) {
        let bounds = bounds
            .as_ref()
            .map(|(lower, upper)| (lower.as_str(), upper.as_str()));

        match self.environment.integrate(expression, bounds) {
            Ok(IntegralOutcome::Function { name, source }) => {
                self.push_output(result_style(), format!("{}(x) = {}   (+ C)", name, source));
            }
            Ok(IntegralOutcome::Value(value)) => {
                self.push_output(result_style(), format!("= {}", format_number(value)));
            }
            Err(error) => self.push_info(error_style(), format!("{}", error)),
        }
    }

    fn command_scales(&mut self, value: Option<bool>) {
        self.scales_visible = value.unwrap_or(!self.scales_visible);

        let state = if self.scales_visible { "on" } else { "off" };

        self.push_output(result_style(), format!("scales {}", state));
    }

    fn command_clear_all(&mut self) {
        self.environment.clear_variables();
        self.environment.clear_functions();
        self.curves.clear();
        self.scales_visible = false;
        self.var_selection = 0;
        self.side_selection = 0;

        if let Some(path) = &self.state_path {
            let _ = std::fs::remove_file(path);
        }

        self.push_output(result_style(), "cleared all".to_string());
    }

    fn command_clear_variables(&mut self) {
        self.environment.clear_variables();
        self.curves.retain(|curve| !curve.is_variable);
        self.var_selection = 0;
        self.push_output(result_style(), "variables cleared".to_string());
    }

    fn command_clear_functions(&mut self) {
        self.environment.clear_functions();
        self.curves.retain(|curve| curve.is_variable);
        self.side_selection = 0;
        self.push_output(result_style(), "functions cleared".to_string());
    }

    fn command_clear_output(&mut self) {
        self.output.clear();
    }

    fn command_clear_history(&mut self) {
        self.editor.set_history(Vec::new());
        self.push_output(result_style(), "history cleared".to_string());
    }

    fn command_plot(&mut self, rest: &str) {
        let rest = rest.trim();

        if rest.is_empty() {
            self.push_info(
                error_style(),
                "usage: plot <expression>[, <expression>]".to_string(),
            );
            return;
        }

        let mut curves = Vec::new();
        let mut errors = Vec::new();
        let mut used: Vec<Color> = Vec::new();

        for part in rest
            .split(',')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
        {
            // A lone defined variable plots as a horizontal line
            if let Some(name) = single_variable(part)
                && let Some(value) = self.environment.values().get(&name).copied()
            {
                let color = pick_color(&used);

                used.push(color);
                curves.push(Curve {
                    source: name.to_string(),
                    node: Node::FloatNumber(value),
                    color,
                    is_variable: true,
                });
                continue;
            }

            let node = match self.environment.expression(part) {
                Ok(node) => node,
                Err(error) => {
                    errors.push(format!("{}: {}", part, error));
                    continue;
                }
            };

            // An existing f(x) call is referenced, not redefined
            if let Some(name) = self.function_reference(&node) {
                let color = pick_color(&used);

                used.push(color);
                curves.push(Curve {
                    source: format!("{}(x)", name),
                    node,
                    color,
                    is_variable: false,
                });
                continue;
            }

            // Everything else is stored as a new function and plotted
            let name = self.environment.next_function_name();
            let definition = format!("{}(x) = {}", name, part);

            match self.environment.execute(&definition) {
                Ok(_) => {
                    let color = pick_color(&used);

                    used.push(color);
                    curves.push(Curve {
                        source: format!("{}(x)", name),
                        node: Node::Call(name, vec![Node::Variable('x')]),
                        color,
                        is_variable: false,
                    });
                }
                Err(error) => errors.push(format!("{}: {}", part, error)),
            }
        }

        for error in &errors {
            self.push_info(error_style(), error.clone());
        }

        if curves.is_empty() {
            return;
        }

        self.curves = curves;
    }

    // Some("f") when the node is exactly f(x) for an existing user function
    fn function_reference(&self, node: &Node) -> Option<String> {
        if let Node::Call(name, args) = node
            && args.len() == 1
            && matches!(args[0], Node::Variable('x'))
            && self.environment.functions().contains_key(name)
        {
            return Some(name.clone());
        }

        None
    }

    fn push_output(&mut self, style: Style, text: String) {
        self.push_line(style, text, Kind::Result);
    }

    fn push_info(&mut self, style: Style, text: String) {
        self.push_line(style, text, Kind::Info);
    }

    fn push_line(&mut self, style: Style, text: String, kind: Kind) {
        self.output.push(OutputLine {
            stamp: Some(now_hms()),
            text,
            style,
            kind,
        });

        self.trim_output();
    }

    fn trim_output(&mut self) {
        if self.output.len() > 1000 {
            self.output.drain(0..500);
        }
    }

    fn function_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.environment.functions().keys().cloned().collect();

        names.sort();
        names
    }

    fn variable_names(&self) -> Vec<char> {
        let mut names: Vec<char> = self.environment.values().keys().copied().collect();

        names.sort();
        names
    }

    // Test helpers
    #[cfg(test)]
    pub(crate) fn visual_active(&self) -> bool {
        self.visual.is_some() || self.side_visual.is_some() || self.input_visual.is_some()
    }

    #[cfg(test)]
    pub(crate) fn output_texts(&self) -> Vec<String> {
        self.output.iter().map(|line| line.text.clone()).collect()
    }
}

// Number of characters in one output line, zero for a missing line
fn line_length(output: &[OutputLine], index: usize) -> usize {
    output
        .get(index)
        .map(|line| line.text.chars().count())
        .unwrap_or(0)
}

fn inner_rect(rect: Rect) -> Rect {
    Rect {
        x: rect.x + 1,
        y: rect.y + 1,
        width: rect.width.saturating_sub(2),
        height: rect.height.saturating_sub(2),
    }
}

// Some(name) when the text is a single alphabetic character
fn single_variable(text: &str) -> Option<char> {
    let mut chars = text.chars();

    match (chars.next(), chars.next()) {
        (Some(name), None) if name.is_ascii_alphabetic() => Some(name),
        _ => None,
    }
}

// Wraps text to width; every part also reports its first character index
fn wrap_with_stamp(
    stamp: Option<&str>,
    text: &str,
    width: usize,
) -> Vec<(Option<String>, String, usize)> {
    let prefix = stamp.map(|stamp| stamp.chars().count() + 3).unwrap_or(0);

    let mut result = Vec::new();
    let mut chars: Vec<char> = text.chars().collect();
    let mut consumed = 0;
    let mut first = true;

    loop {
        let available = if first {
            width.saturating_sub(prefix).max(1)
        } else {
            width.max(1)
        };

        let take = chars.len().min(available);
        let chunk: String = chars.drain(..take).collect();
        let stamp_here = if first {
            stamp.map(|stamp| stamp.to_string())
        } else {
            None
        };

        result.push((stamp_here, chunk, consumed));
        consumed += take;
        first = false;

        if chars.is_empty() {
            break;
        }
    }

    result
}

// Inclusive character range selected on one line, if any
pub(crate) fn selection_span(
    line: usize,
    line_len: usize,
    anchor: (usize, usize),
    cursor: (usize, usize),
) -> Option<(usize, usize)> {
    if line_len == 0 {
        return None;
    }

    let low = anchor.min(cursor);
    let high = anchor.max(cursor);

    if line < low.0 || line > high.0 {
        return None;
    }

    let begin = if line == low.0 { low.1 } else { 0 };
    let finish = if line == high.0 { high.1 } else { line_len - 1 };

    Some((begin.min(line_len - 1), finish.min(line_len - 1)))
}

// Text between the two positions, one column per character, lines joined with newlines
pub(crate) fn selected_text(
    lines: &[String],
    anchor: (usize, usize),
    cursor: (usize, usize),
) -> String {
    let low = anchor.min(cursor);
    let high = anchor.max(cursor);
    let mut parts: Vec<String> = Vec::new();

    for line_index in low.0..=high.0 {
        let Some(line) = lines.get(line_index) else {
            break;
        };

        let chars: Vec<char> = line.chars().collect();

        match selection_span(line_index, chars.len(), anchor, cursor) {
            Some((begin, finish)) => parts.push(chars[begin..=finish].iter().collect()),
            None => parts.push(String::new()),
        }
    }

    parts.join("\n")
}

fn border_style() -> Style {
    Style::fg(Color::BrightBlack)
}

// Focused pane gets a plain white frame
fn pane_border_style(focused: bool) -> Style {
    if focused {
        Style::fg(Color::White)
    } else {
        border_style()
    }
}

// First palette colour not currently in use, so plots keep distinct colours
fn pick_color(used: &[Color]) -> Color {
    for color in PALETTE {
        if !used.contains(&color) {
            return color;
        }
    }

    PALETTE[used.len() % PALETTE.len()]
}

// Prints one sidebar row, placing a block cursor on the given column
fn print_side_row(
    canvas: &mut Canvas,
    x: isize,
    y: isize,
    width: usize,
    text: &str,
    style: Style,
    cursor: Option<usize>,
) {
    if width == 0 {
        return;
    }

    let chars: Vec<char> = text.chars().collect();

    if chars.is_empty() {
        return;
    }

    let count = if chars.len() <= width {
        chars.len()
    } else {
        width
    };

    for index in 0..count {
        let ch = if chars.len() > width && index == width - 1 {
            '…'
        } else {
            chars[index]
        };
        let style = if cursor == Some(index) {
            cursor_style()
        } else {
            style
        };

        canvas.put(x + index as isize, y, ch, style);
    }
}

// Draws one part of a box title clipped with an ellipsis; returns columns used
fn draw_title_segment(
    canvas: &mut Canvas,
    x: isize,
    y: isize,
    max_width: usize,
    text: &str,
    style: Style,
) -> usize {
    if max_width == 0 {
        return 0;
    }

    let chars: Vec<char> = text.chars().collect();

    if chars.len() <= max_width {
        canvas.print(x, y, text, style);
        return chars.len();
    }

    let mut clipped: String = chars[..max_width - 1].iter().collect();

    clipped.push('…');
    canvas.print(x, y, &clipped, style);

    max_width
}

fn hint_style() -> Style {
    Style::fg(Color::BrightBlack)
}

fn prompt_style() -> Style {
    Style::fg(Color::BrightCyan).bold()
}

fn value_style() -> Style {
    Style::fg(Color::White)
}

fn result_style() -> Style {
    Style::fg(Color::BrightGreen)
}

fn error_style() -> Style {
    Style::fg(Color::BrightRed)
}

fn axis_style() -> Style {
    Style::fg(Color::BrightBlack)
}

fn selected_style() -> Style {
    Style::fg(Color::Black).on(Color::BrightYellow)
}

fn cursor_style() -> Style {
    Style::fg(Color::Black).on(Color::BrightCyan)
}
