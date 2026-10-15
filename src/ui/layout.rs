// Height of the input box row block
pub const INPUT_HEIGHT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl Rect {
    pub fn right(&self) -> usize {
        self.x + self.width
    }

    pub fn bottom(&self) -> usize {
        self.y + self.height
    }
}

pub struct Layout {
    pub main: Rect,
    pub side: Option<Rect>,
    pub input: Rect,
}

// Distributes the available space; two columns when there is enough room
pub fn compute(columns: usize, rows: usize) -> Layout {
    compute_with_side(columns, rows, true)
}

// Like compute, but can keep the whole content area for the main pane
pub fn compute_with_side(columns: usize, rows: usize, want_side: bool) -> Layout {
    let input_y = rows.saturating_sub(INPUT_HEIGHT);
    let input = Rect {
        x: 0,
        y: input_y,
        width: columns,
        height: rows.saturating_sub(input_y),
    };

    let content_height = input_y;

    let (main, side) = if want_side {
        split_content(columns, content_height)
    } else {
        (
            Rect {
                x: 0,
                y: 0,
                width: columns,
                height: content_height,
            },
            None,
        )
    };

    Layout { main, side, input }
}

// Splits the content area above the input into main and optional side
fn split_content(columns: usize, height: usize) -> (Rect, Option<Rect>) {
    let two_column = columns >= 72 && height >= 8;

    if two_column {
        let side_width = (columns / 3).clamp(24, 44);
        let main_width = columns - side_width;

        let main = Rect {
            x: 0,
            y: 0,
            width: main_width,
            height,
        };
        let side = Rect {
            x: main_width,
            y: 0,
            width: side_width,
            height,
        };

        return (main, Some(side));
    }

    if height >= 6 {
        let side_height = (height / 3).clamp(3, 8);

        let main = Rect {
            x: 0,
            y: 0,
            width: columns,
            height: height - side_height,
        };
        let side = Rect {
            x: 0,
            y: height - side_height,
            width: columns,
            height: side_height,
        };

        return (main, Some(side));
    }

    let main = Rect {
        x: 0,
        y: 0,
        width: columns,
        height,
    };

    (main, None)
}
