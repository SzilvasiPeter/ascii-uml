//! Blocks hold diagram text as rows for aligned composition.
//!
//! Parsers reject control characters before creating blocks, since they break layout.
//! Each text block has one row. Spaces carry layout, so composition preserves them.

/// Number of rows in a boxed block.
pub(super) const BLOCK_HEIGHT: usize = 3;

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
    for x in [left, right] {
        for y in [top, bottom] {
            canvas[y][x] = '+';
        }
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
    /// Pads rows to the widest row so the block stays rectangular.
    fn new(mut lines: Vec<String>) -> Self {
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        for line in &mut lines {
            line.push_str(&" ".repeat(width - line.chars().count()));
        }
        Self { lines }
    }

    /// Keeps one row unchanged because spaces carry layout.
    /// Callers must reject control characters because they break rendering.
    #[must_use]
    pub(super) fn text(source: impl Into<String>) -> Self {
        Self { lines: vec![source.into()] }
    }

    /// Wraps a label in the standard three-row class and participant box.
    #[must_use]
    pub(super) fn boxed(label: impl Into<String>) -> Self {
        let label = label.into();
        let width = (label.chars().count() + 4) | 1;
        let border = format!("+{}+", "-".repeat(width - 2));
        let padding = " ".repeat(width - 3 - label.chars().count());
        Self::new(vec![border.clone(), format!("| {label}{padding}|"), border])
    }

    /// Copies the block at `(x, y)`; spaces clear cells for correct layout.
    pub(super) fn draw_on(&self, canvas: &mut [Vec<char>], x: usize, y: usize) {
        for (row, line) in self.lines.iter().enumerate() {
            for (column, character) in line.chars().enumerate() {
                canvas[y + row][x + column] = character;
            }
        }
    }

    /// Places blocks side by side with `gap` spaces.
    /// Shorter blocks are padded underneath to keep rows aligned.
    #[must_use]
    pub(super) fn horizontal(blocks: impl IntoIterator<Item = Self>, gap: usize) -> Self {
        let blocks: Vec<_> = blocks.into_iter().collect();
        let height = blocks.iter().map(|b| b.lines.len()).max().unwrap_or(0);
        let spacer = " ".repeat(gap);
        let mut lines = vec![String::new(); height];
        for (i, block) in blocks.into_iter().enumerate() {
            let width = block.width();
            for (row, line) in lines.iter_mut().enumerate() {
                if i > 0 {
                    line.push_str(&spacer);
                }
                match block.lines.get(row) {
                    Some(text) => line.push_str(text),
                    None => line.push_str(&" ".repeat(width)),
                }
            }
        }
        Self { lines }
    }

    /// Stacks blocks with `gap` blank rows between them.
    /// Rows share a left edge and width for consistent alignment.
    #[must_use]
    pub(super) fn vertical(blocks: impl IntoIterator<Item = Self>, gap: usize) -> Self {
        let mut rows = Vec::new();
        for (i, block) in blocks.into_iter().enumerate() {
            if i > 0 {
                rows.extend(std::iter::repeat_n(String::new(), gap));
            }
            rows.extend(block.lines);
        }
        Self::new(rows)
    }

    /// Creates `width` by `height` blank space for diagram gaps.
    #[must_use]
    pub(super) fn spacer(width: usize, height: usize) -> Self {
        let line = " ".repeat(width);
        Self { lines: vec![line; height] }
    }

    /// Joins rows without a final newline so callers control line endings.
    #[must_use]
    pub(super) fn render(&self) -> String {
        self.lines.join("\n")
    }

    pub(super) fn width(&self) -> usize {
        self.lines.first().map_or(0, |line| line.chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxes_a_short_label() {
        assert_eq!(Block::boxed("A").render(), "+---+\n| A |\n+---+");
    }

    #[test]
    fn boxes_an_even_label_with_odd_width() {
        assert_eq!(Block::boxed("ab").render(), "+-----+\n| ab  |\n+-----+");
    }

    #[test]
    fn measures_labels_in_characters() {
        assert_eq!(Block::boxed("🙂").render(), "+---+\n| 🙂 |\n+---+");
        assert_eq!(Block::boxed("世界").render(), "+-----+\n| 世界  |\n+-----+");
    }

    #[test]
    fn joins_blocks_horizontally_with_a_gap() {
        let block = Block::horizontal([Block::boxed("A"), Block::boxed("B")], 1);
        assert_eq!(block.render(), "+---+ +---+\n| A | | B |\n+---+ +---+");
    }

    #[test]
    fn top_aligns_blocks_of_different_heights() {
        let text = Block::vertical([Block::text("ab"), Block::text("cd")], 0);
        let block = Block::horizontal([text, Block::boxed("A")], 1);
        assert_eq!(block.render(), "ab +---+\ncd | A |\n   +---+");
    }

    #[test]
    fn stacks_blocks_vertically_with_a_gap() {
        let block = Block::vertical([Block::boxed("A"), Block::boxed("BC")], 1);
        assert_eq!(block.render(), "+---+  \n| A |  \n+---+  \n       \n+-----+\n| BC  |\n+-----+");
    }

    #[test]
    fn left_aligns_blocks_of_different_widths() {
        let block = Block::vertical([Block::text("abc"), Block::text("de")], 0);
        assert_eq!(block.render(), "abc\nde ");
    }

    #[test]
    fn composes_rows_and_columns() {
        let grid = Block::vertical(
            [
                Block::horizontal([Block::boxed("A"), Block::boxed("B")], 1),
                Block::horizontal([Block::boxed("C"), Block::boxed("D")], 1),
            ],
            0,
        );
        assert_eq!(
            grid.render(),
            "+---+ +---+\n| A | | B |\n+---+ +---+\n+---+ +---+\n| C | | D |\n+---+ +---+"
        );
    }

    #[test]
    fn spacers_override_gap() {
        let block = |text: &str| Block::boxed(text);
        let blocks = Block::horizontal(
            [block("A"), Block::spacer(3, BLOCK_HEIGHT), block("B"), block("C")],
            0,
        );
        assert_eq!(blocks.render(), "+---+   +---++---+\n| A |   | B || C |\n+---+   +---++---+");
        let blocks = Block::vertical(
            [block("A"), Block::spacer(0, BLOCK_HEIGHT), block("B"), block("C")],
            0,
        );
        assert_eq!(
            blocks.render(),
            "+---+\n| A |\n+---+\n     \n     \n     \n+---+\n| B |\n+---+\n+---+\n| C |\n+---+"
        );
    }

    #[test]
    fn renders_whitespace_verbatim() {
        assert_eq!(Block::text("  a  ").render(), "  a  ");
        assert_eq!(Block::text(" b ").render(), " b ");
    }

    #[test]
    fn renders_empty_compositions_as_empty_strings() {
        assert_eq!(Block::text("").render(), "");
        assert_eq!(Block::horizontal([], 1).render(), "");
        assert_eq!(Block::vertical([], 1).render(), "");
    }
}
