use std::path::PathBuf;

use crate::core::environment::Environment;
use crate::core::node::Node;
use crate::core::plot;
use crate::ui::canvas::{Canvas, Color, Style};
use crate::ui::format::{format_number, format_outcome, format_scale, now_hms};
use crate::ui::input::{Key, KeyReader, LineEditor};
use crate::ui::layout::{self, Layout, Rect};
use crate::ui::persist::{self, State};
use crate::ui::plot_target;
use crate::ui::term::Terminal;

const PALETTE: [Color; 6] = [
    Color::BrightRed,
    Color::BrightGreen,
    Color::BrightYellow,
    Color::BrightBlue,
    Color::BrightMagenta,
    Color::BrightCyan,
];

#[derive(PartialEq, Eq, Clone, Copy)]
enum Focus {
    Input,
    Variables,
    Functions,
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

struct OutputLine {
    stamp: Option<String>,
    text: String,
    style: Style,
}

pub struct App {
    environment: Environment,
    editor: LineEditor,
    curves: Vec<Curve>,
    output: Vec<OutputLine>,
    focus: Focus,
    var_selection: usize,
    side_selection: usize,
    status: String,
    scroll: usize,
    help_visible: bool,
    scales_visible: bool,
    state_path: Option<PathBuf>,
    input_mode: InputMode,
    pending_d: bool,
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
            status: String::new(),
            scroll: 0,
            help_visible: false,
            scales_visible: false,
            state_path: None,
            should_quit: false,
        };

        app.push_notice(
            hint_style(),
            "Type an expression or assignment, then press Enter.".to_string(),
        );
        app.push_notice(
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
            plots: self.curves.iter().map(|curve| curve.source.clone()).collect(),
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

    fn render(&self, columns: usize, rows: usize) -> String {
        let mut canvas = Canvas::new(columns, rows);
        let layout = layout::compute(columns, rows);

        if self.help_visible {
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
            Some("Help  ·  Esc to close"),
        );

        let inner = inner_rect(rect);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let lines = [
            "Expressions & assignments",
            "  x = 3          define a variable",
            "  f(x) = 3*x^3   define a function",
            "Plotting",
            "  plot sin(x), cos(x)     define & plot (auto f, g, ...)",
            "  plot clear / scales     clear curves / toggle labels",
            "Definitions",
            "  vars clear   funcs clear   clear all",
            "Editing (vi mode, Esc = normal)",
            "  h l 0 $ w b    move / start / end / word",
            "  i a I A        insert before / after / start / end",
            "  x D dd         delete char / to end / whole line",
            "  k / j          older / newer history",
            "Keys",
            "  Enter          evaluate",
            "  Tab            input / variables / functions",
            "  Ctrl-C / Ctrl-D   clear input / quit",
            "  ?              show this help",
        ];

        for (row, line) in lines.iter().take(inner.height).enumerate() {
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

    fn draw_input(&self, canvas: &mut Canvas, layout: &Layout) {
        let rect = layout.input;
        let mode = match self.input_mode {
            InputMode::Insert => "INSERT",
            InputMode::Normal => "NORMAL",
        };
        let label = format!("Input  [{}]", mode);
        let title = if self.status.is_empty() {
            label
        } else {
            format!("{}  |  {}", label, self.status)
        };

        canvas.draw_box(
            rect.x as isize,
            rect.y as isize,
            rect.width,
            rect.height,
            border_style(),
            Some(&title),
        );

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
        let offset = if cursor >= avail { cursor + 1 - avail } else { 0 };
        let visible: String = chars.iter().skip(offset).take(avail).collect();

        canvas.print(text_x, line_y, &visible, value_style());

        if self.focus == Focus::Input {
            let cursor_index = cursor - offset;

            if cursor_index < avail {
                let ch = chars.get(cursor).copied().unwrap_or(' ');

                canvas.put(
                    text_x + cursor_index as isize,
                    line_y,
                    ch,
                    cursor_style(),
                );
            }
        }
    }

    fn draw_main(&self, canvas: &mut Canvas, layout: &Layout) {
        let main = layout.main;

        if main.height < 2 || main.width < 2 {
            return;
        }

        if self.curves.is_empty() {
            self.draw_output(canvas, main, "Output");
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
            self.draw_output(canvas, output_rect, "Output");
        }
    }

    fn draw_output(&self, canvas: &mut Canvas, rect: Rect, title: &str) {
        canvas.draw_box(
            rect.x as isize,
            rect.y as isize,
            rect.width,
            rect.height,
            border_style(),
            Some(title),
        );

        let inner = inner_rect(rect);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let mut wrapped: Vec<(Option<String>, String, Style)> = Vec::new();

        for line in &self.output {
            for (stamp, chunk) in wrap_with_stamp(line.stamp.as_deref(), &line.text, inner.width) {
                wrapped.push((stamp, chunk, line.style));
            }
        }

        let total = wrapped.len();
        let view = inner.height;
        let max_scroll = total.saturating_sub(view);
        let scroll = self.scroll.min(max_scroll);
        let end = total - scroll;
        let start = end.saturating_sub(view);

        for (row, (stamp, text, style)) in wrapped[start..end].iter().enumerate() {
            let y = inner.y as isize + row as isize;
            let mut x = inner.x as isize;

            if let Some(stamp) = stamp {
                let label = format!("[{}] ", stamp);

                canvas.print(x, y, &label, hint_style());
                x += label.chars().count() as isize;
            }

            canvas.print(x, y, text, *style);
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
            canvas.print(inner.x as isize, data.y as isize, "(no finite values)", hint_style());
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

        if rect.height < 8 {
            if self.focus == Focus::Variables {
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
        canvas.draw_box(
            rect.x as isize,
            rect.y as isize,
            rect.width,
            rect.height,
            border_style(),
            Some("Variables"),
        );

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
        let offset = if selection >= view { selection + 1 - view } else { 0 };

        for (row, name) in names.iter().skip(offset).take(view).enumerate() {
            let index = offset + row;
            let value = self.environment.values().get(name).copied().unwrap_or(0.0);
            let plotted = self
                .curves
                .iter()
                .any(|curve| curve.is_variable && curve.source == name.to_string());
            let marker = if plotted { "* " } else { "  " };
            let text = format!("{}{} = {}", marker, name, format_number(value));
            let style = if self.focus == Focus::Variables && index == selection {
                selected_style()
            } else if plotted {
                Style::fg(Color::BrightWhite)
            } else {
                value_style()
            };

            canvas.print_clipped(
                inner.x as isize,
                inner.y as isize + row as isize,
                inner.width,
                &text,
                style,
            );
        }
    }

    fn draw_functions(&self, canvas: &mut Canvas, rect: Rect) {
        canvas.draw_box(
            rect.x as isize,
            rect.y as isize,
            rect.width,
            rect.height,
            border_style(),
            Some("Functions"),
        );

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
        let offset = if selection >= view { selection + 1 - view } else { 0 };
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
            let style = if self.focus == Focus::Functions && index == selection {
                selected_style()
            } else if plotted {
                Style::fg(Color::BrightWhite)
            } else {
                value_style()
            };

            let fits = base.chars().count() + preview.chars().count() + 2 <= inner.width;

            if preview.is_empty() || fits {
                let text = if preview.is_empty() {
                    base.clone()
                } else {
                    format!("{}  {}", base, preview)
                };

                canvas.print_clipped(
                    inner.x as isize,
                    inner.y as isize + row as isize,
                    inner.width,
                    &text,
                    style,
                );
                row += 1;
            } else {
                canvas.print_clipped(
                    inner.x as isize,
                    inner.y as isize + row as isize,
                    inner.width,
                    &base,
                    style,
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

    fn handle_key(&mut self, key: Key) {
        if self.help_visible {
            if key != Key::Unknown {
                self.help_visible = false;
            }

            return;
        }

        match self.focus {
            Focus::Input => self.handle_input_key(key),
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
            Key::Tab => {
                let next = self.first_side_focus();

                self.focus = next;

                if next == Focus::Input {
                    self.input_mode = InputMode::Insert;
                }
            }
            Key::CtrlL => {}
            Key::CtrlC => {
                self.editor.clear();
                self.input_mode = InputMode::Insert;
                self.status = "input cleared".to_string();
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
            Key::Unknown => {}
        }
    }

    fn handle_normal_key(&mut self, key: Key) {
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
            Key::Tab => {
                let next = self.first_side_focus();
                self.focus = next;

                if next == Focus::Input {
                    self.input_mode = InputMode::Insert;
                }
            }
            Key::CtrlC => {
                self.editor.clear();
                self.input_mode = InputMode::Insert;
                self.status = "input cleared".to_string();
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

    fn handle_side_key(&mut self, key: Key) {
        let total = match self.focus {
            Focus::Variables => self.variable_names().len(),
            Focus::Functions => self.function_names().len(),
            Focus::Input => 0,
        };

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
            Key::Char(' ') | Key::Enter => self.toggle_selected(),
            Key::Char('d') | Key::Delete => self.delete_selected(),
            Key::Tab => {
                let next = self.next_side_focus();
                self.focus = next;

                if next == Focus::Input {
                    self.input_mode = InputMode::Insert;
                }
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

    fn current_selection_mut(&mut self) -> &mut usize {
        match self.focus {
            Focus::Variables => &mut self.var_selection,
            _ => &mut self.side_selection,
        }
    }

    fn first_side_focus(&self) -> Focus {
        if self.variables_present() {
            Focus::Variables
        } else if self.functions_present() {
            Focus::Functions
        } else {
            Focus::Input
        }
    }

    fn next_side_focus(&self) -> Focus {
        match self.focus {
            Focus::Variables if self.functions_present() => Focus::Functions,
            _ => Focus::Input,
        }
    }

    fn variables_present(&self) -> bool {
        !self.environment.values().is_empty()
    }

    fn functions_present(&self) -> bool {
        !self.environment.functions().is_empty()
    }

    fn toggle_selected(&mut self) {
        match self.focus {
            Focus::Variables => self.toggle_variable(),
            Focus::Functions => self.toggle_function(),
            Focus::Input => {}
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
            self.status = format!("removed plot {}", source);
            return;
        }

        let value = match self.environment.values().get(&name) {
            Some(value) => *value,
            None => return,
        };
        let color = PALETTE[self.curves.len() % PALETTE.len()];

        self.curves.push(Curve {
            source: source.clone(),
            node: Node::FloatNumber(value),
            color,
            is_variable: true,
        });
        self.status = format!("plotted {} = {}", name, format_number(value));
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
            self.status = format!("{} needs exactly one parameter to plot", name);
            return;
        }

        let source = format!("{}(x)", name);

        if let Some(position) = self.curves.iter().position(|curve| curve.source == source) {
            self.curves.remove(position);
            self.status = format!("removed plot {}", source);
            return;
        }

        let color = PALETTE[self.curves.len() % PALETTE.len()];

        self.curves.push(Curve {
            source: source.clone(),
            node: Node::Call(name.clone(), vec![Node::Variable('x')]),
            color,
            is_variable: false,
        });
        self.status = format!("plotted {}", source);
    }

    fn delete_selected(&mut self) {
        match self.focus {
            Focus::Variables => self.delete_variable(),
            Focus::Functions => self.delete_function(),
            Focus::Input => {}
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
            self.status = format!("{} deleted", name);

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
            self.curves.retain(|curve| curve.source != format!("{}(x)", name));
            self.status = format!("{} deleted", name);

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
            return;
        }

        self.push_output(hint_style(), format!("> {}", trimmed));

        if trimmed == "quit" || trimmed == "exit" {
            self.persist();
            self.should_quit = true;
            return;
        }

        if let Some(rest) = plot_target(&trimmed) {
            self.command_plot(rest);
            self.persist();
            return;
        }

        if trimmed == "scales" || trimmed == "scales on" || trimmed == "scales off" {
            self.command_scales(&trimmed);
            self.persist();
            return;
        }

        if trimmed == "vars clear" {
            self.command_clear_variables();
            self.persist();
            return;
        }

        if trimmed == "funcs clear" {
            self.command_clear_functions();
            self.persist();
            return;
        }

        if trimmed == "clear all" {
            self.command_clear_all();
            return;
        }

        match self.environment.execute(&trimmed) {
            Ok(outcome) => self.push_output(result_style(), format_outcome(outcome)),
            Err(error) => self.push_output(error_style(), format!("{}", error)),
        }

        self.persist();
    }

    fn command_scales(&mut self, command: &str) {
        self.scales_visible = match command {
            "scales on" => true,
            "scales off" => false,
            _ => !self.scales_visible,
        };

        let state = if self.scales_visible { "on" } else { "off" };

        self.status = format!("scales {}", state);
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

        self.status = "cleared all".to_string();
        self.push_output(result_style(), "cleared all".to_string());
    }

    fn command_clear_variables(&mut self) {
        self.environment.clear_variables();
        self.curves.retain(|curve| !curve.is_variable);
        self.var_selection = 0;
        self.status = "variables cleared".to_string();
        self.push_output(result_style(), "variables cleared".to_string());
    }

    fn command_clear_functions(&mut self) {
        self.environment.clear_functions();
        self.curves.retain(|curve| curve.is_variable);
        self.side_selection = 0;
        self.status = "functions cleared".to_string();
        self.push_output(result_style(), "functions cleared".to_string());
    }

    fn command_plot(&mut self, rest: &str) {
        let rest = rest.trim();

        if rest.is_empty() {
            self.push_output(error_style(), "usage: plot <expression>[, <expression>]".to_string());
            return;
        }

        if rest == "clear" {
            self.curves.clear();
            self.status = "plot cleared".to_string();
            return;
        }

        let mut curves = Vec::new();
        let mut errors = Vec::new();

        for (index, part) in rest
            .split(',')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .enumerate()
        {
            let color = PALETTE[index % PALETTE.len()];

            // A lone defined variable plots as a horizontal line
            if let Some(name) = single_variable(part) {
                if let Some(value) = self.environment.values().get(&name).copied() {
                    curves.push(Curve {
                        source: name.to_string(),
                        node: Node::FloatNumber(value),
                        color,
                        is_variable: true,
                    });
                    continue;
                }
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
                Ok(_) => curves.push(Curve {
                    source: format!("{}(x)", name),
                    node: Node::Call(name, vec![Node::Variable('x')]),
                    color,
                    is_variable: false,
                }),
                Err(error) => errors.push(format!("{}: {}", part, error)),
            }
        }

        for error in &errors {
            self.push_output(error_style(), error.clone());
        }

        if curves.is_empty() {
            return;
        }

        self.curves = curves;
        self.status = format!("plotting {} curve(s)", self.curves.len());
    }

    // Some("f") when the node is exactly f(x) for an existing user function
    fn function_reference(&self, node: &Node) -> Option<String> {
        if let Node::Call(name, args) = node {
            if args.len() == 1 && matches!(args[0], Node::Variable('x')) {
                if self.environment.functions().contains_key(name) {
                    return Some(name.clone());
                }
            }
        }

        None
    }

    fn push_output(&mut self, style: Style, text: String) {
        self.output.push(OutputLine {
            stamp: Some(now_hms()),
            text,
            style,
        });

        self.trim_output();
    }

    fn push_notice(&mut self, style: Style, text: String) {
        self.output.push(OutputLine {
            stamp: None,
            text,
            style,
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

fn wrap_with_stamp(stamp: Option<&str>, text: &str, width: usize) -> Vec<(Option<String>, String)> {
    let prefix = stamp.map(|stamp| stamp.chars().count() + 3).unwrap_or(0);

    let mut result = Vec::new();
    let mut chars: Vec<char> = text.chars().collect();
    let mut first = true;

    loop {
        let available = if first {
            width.saturating_sub(prefix).max(1)
        } else {
            width.max(1)
        };

        let take = chars.len().min(available);
        let chunk: String = chars.drain(..take).collect();
        let stamp_here = if first { stamp.map(|stamp| stamp.to_string()) } else { None };

        result.push((stamp_here, chunk));
        first = false;

        if chars.is_empty() {
            break;
        }
    }

    result
}

fn border_style() -> Style {
    Style::fg(Color::BrightBlack)
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
    Style::fg(Color::Black).on(Color::BrightCyan)
}

fn cursor_style() -> Style {
    Style::fg(Color::Black).on(Color::BrightCyan)
}
