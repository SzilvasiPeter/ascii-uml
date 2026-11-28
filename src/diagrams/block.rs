//! Boxed labels and the shared canvas primitives used to draw diagrams.

/// Canvas helpers let routed edges share and join one drawing surface.
pub(super) fn canvas(width: usize, height: usize) -> Vec<Vec<char>> {
    vec![vec![' '; width]; height]
}

/// Draws a line glyph; crossings become `+` so routes stay connected.
pub(super) fn put(canvas: &mut [Vec<char>], x: usize, y: usize, glyph: char) {
    let current = canvas[y][x];
    let horizontal = |character| matches!(character, '-' | '.');
    let vertical = |character| matches!(character, '|' | ':');
    canvas[y][x] = if current == ' ' {
        glyph
    } else if (current == '+' || glyph == '+')
        || (horizontal(current) && vertical(glyph))
        || (vertical(current) && horizontal(glyph))
    {
        '+'
    } else if current == glyph {
        current
    } else {
        glyph
    };
}

pub(super) fn draw_text(canvas: &mut [Vec<char>], row: usize, col: usize, text: &str) {
    for (i, c) in text.chars().enumerate() {
        canvas[row][col + i] = c;
    }
}

pub(super) fn draw_frame(
    canvas: &mut [Vec<char>],
    (left, top, right, bottom): (usize, usize, usize, usize),
    title: &str,
) {
    canvas[top][left..=right].fill('-');
    canvas[bottom][left..=right].fill('-');
    for row in canvas.iter_mut().take(bottom).skip(top + 1) {
        row[left] = '|';
        row[right] = '|';
    }
    for (x, y) in [(left, top), (left, bottom), (right, top), (right, bottom)] {
        canvas[y][x] = '+';
    }
    draw_text(canvas, top, left + 1, &format!(" {title} "));
}

pub(super) fn draw_hline(canvas: &mut [Vec<char>], row: usize, a: usize, b: usize, glyph: char) {
    for col in a.min(b)..=a.max(b) {
        put(canvas, col, row, glyph);
    }
}

pub(super) fn draw_vline(canvas: &mut [Vec<char>], col: usize, a: usize, b: usize, glyph: char) {
    for row in a.min(b)..=a.max(b) {
        put(canvas, col, row, glyph);
    }
}

/// Draws a horizontal line and writes `head` so it ends at `to`.
pub(super) fn arrow_h(
    canvas: &mut [Vec<char>],
    row: usize,
    from: usize,
    to: usize,
    glyph: char,
    head: &str,
) {
    draw_hline(canvas, row, from, to, glyph);
    let start = if to >= from { to + 1 - head.chars().count() } else { to };
    draw_text(canvas, row, start, head);
}

/// Draws a vertical arrow between `from` and `to`, ending in `^` or `v`.
pub(super) fn arrow_v(canvas: &mut [Vec<char>], col: usize, from: usize, to: usize, glyph: char) {
    draw_vline(canvas, col, from, to, glyph);
    canvas[to][col] = if to > from { 'v' } else { '^' };
}

/// Joins canvas rows and trims their ends to avoid trailing output spaces.
pub(super) fn render_canvas(canvas: &[Vec<char>]) -> String {
    canvas
        .iter()
        .map(|row| row.iter().copied().collect::<String>().trim_end().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Joins canvas rows without trimming so padded layouts stay intact.
pub(super) fn render_canvas_padded(canvas: &[Vec<char>]) -> String {
    canvas.iter().map(|row| row.iter().collect::<String>()).collect::<Vec<_>>().join("\n")
}

pub(super) fn frame(title: &str, content: &str) -> String {
    let lines: Vec<_> = content.lines().collect();
    let width = lines.iter().chain([&title]).map(|s| s.chars().count()).max().unwrap_or(0) + 4;
    let mut canvas = canvas(width, lines.len() + 2);
    draw_frame(&mut canvas, (0, 0, width - 1, lines.len() + 1), title);
    for (row, line) in lines.iter().enumerate() {
        draw_text(&mut canvas, row + 1, 1, &format!(" {line}"));
    }
    render_canvas(&canvas)
}

/// A rectangle with equal-width rows for reliable composition.
#[derive(Debug)]
pub(super) struct Block {
    lines: Vec<String>,
}

impl Block {
    /// Wraps a label in the standard three-row class and participant box.
    #[must_use]
    pub(super) fn boxed(label: impl Into<String>) -> Self {
        let label = label.into();
        let width = (label.chars().count() + 4) | 1;
        let border = format!("+{}+", "-".repeat(width - 2));
        let padding = " ".repeat(width - 3 - label.chars().count());
        Self { lines: vec![border.clone(), format!("| {label}{padding}|"), border] }
    }

    /// Copies the block at `(x, y)`; spaces clear cells for correct layout.
    pub(super) fn draw_on(&self, canvas: &mut [Vec<char>], x: usize, y: usize) {
        for (row, line) in self.lines.iter().enumerate() {
            for (column, character) in line.chars().enumerate() {
                canvas[y + row][x + column] = character;
            }
        }
    }

    pub(super) fn width(&self) -> usize {
        self.lines.first().map_or(0, |line| line.chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(block: &Block) -> String {
        block.lines.join("\n")
    }

    #[test]
    fn boxes_a_short_label() {
        assert_eq!(render(&Block::boxed("A")), "+---+\n| A |\n+---+");
    }

    #[test]
    fn boxes_an_even_label_with_odd_width() {
        assert_eq!(render(&Block::boxed("ab")), "+-----+\n| ab  |\n+-----+");
    }

    #[test]
    fn measures_labels_in_characters() {
        assert_eq!(render(&Block::boxed("🙂")), "+---+\n| 🙂 |\n+---+");
        assert_eq!(render(&Block::boxed("世界")), "+-----+\n| 世界  |\n+-----+");
    }
}
