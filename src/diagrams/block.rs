//! A rectangular block of text lines that composes horizontally and vertically.
//!
//! This module is internal to the diagram renderers (`mod block`, never
//! exported), so its constructors are infallible and trust their callers: the
//! parsers reject control characters before any block is built, and every
//! `text` input is exactly one row. Misuse hardening is deferred until — and
//! only if — this crate grows a library API (see the TODO in
//! `diagrams/mod.rs`).
//!
//! Whitespace is layout: gaps and offsets are carried by padded lines, so
//! nothing is ever trimmed. Dimensions come from composition only: `text` is
//! one verbatim row, and height grows only through `vertical` and `boxed`.

/// The row count of the [`Block::boxed`] shape.
pub(super) const BLOCK_HEIGHT: usize = 3;

/// A rectangle of text lines, each padded to the block width.
#[derive(Debug)]
pub struct Block {
    lines: Vec<String>,
}

impl Block {
    /// Builds a block from raw rows, padding every row to the widest one.
    fn new(rows: Vec<String>) -> Self {
        let width = rows.iter().map(|row| row.chars().count()).max().unwrap_or(0);
        let lines = rows.into_iter().map(|row| padded(row, width)).collect();
        Self { lines }
    }

    /// A single row of drawable text, kept verbatim: leading and trailing
    /// spaces are layout and are preserved.
    ///
    /// Internal contract: exactly one row without control characters. The
    /// parsers guarantee both before any block is built.
    #[must_use]
    pub fn text(source: impl Into<String>) -> Self {
        Self { lines: vec![source.into()] }
    }

    /// The three-row box shared by the diagram renderers.
    ///
    /// ```text
    /// +---+
    /// | A |
    /// +---+
    /// ```
    ///
    /// Internal contract: the label is one row without control characters.
    #[must_use]
    pub fn boxed(label: impl Into<String>) -> Self {
        let label = label.into();
        let width = (label.chars().count() + 4) | 1;
        let border = format!("+{}+", "-".repeat(width - 2));
        let padding = " ".repeat(width - 3 - label.chars().count());
        Self::new(vec![border.clone(), format!("| {label}{padding}|"), border])
    }

    /// Places blocks left to right with `gap` spaces between neighbors.
    /// Blocks of different heights are top-aligned; missing rows become spaces.
    #[must_use]
    pub fn horizontal(blocks: impl IntoIterator<Item = Self>, gap: usize) -> Self {
        let blocks: Vec<_> = blocks.into_iter().collect();
        let height = blocks.iter().map(|block| block.lines.len()).max().unwrap_or(0);
        let spacer = " ".repeat(gap);
        let mut lines = vec![String::new(); height];
        for (index, block) in blocks.into_iter().enumerate() {
            if index > 0 {
                for row in &mut lines {
                    row.push_str(&spacer);
                }
            }
            let width = block.width();
            for (row, line) in lines.iter_mut().enumerate() {
                match block.lines.get(row) {
                    Some(text) => line.push_str(text),
                    None => line.push_str(&" ".repeat(width)),
                }
            }
        }
        Self { lines }
    }

    /// Places blocks top to bottom with `gap` blank rows between neighbors.
    /// Blocks of different widths are left-aligned; rows are re-padded.
    #[must_use]
    pub fn vertical(blocks: impl IntoIterator<Item = Self>, gap: usize) -> Self {
        let mut rows = Vec::new();
        for (index, block) in blocks.into_iter().enumerate() {
            if index > 0 {
                rows.extend(std::iter::repeat_n(String::new(), gap));
            }
            rows.extend(block.lines);
        }
        Self::new(rows)
    }

    /// A blank block of the given size.
    #[must_use]
    pub fn spacer(width: usize, height: usize) -> Self {
        let line = " ".repeat(width);
        Self { lines: vec![line; height] }
    }

    /// Joins the rows with `\n` and appends no trailing newline.
    #[must_use]
    pub fn render(&self) -> String {
        self.lines.join("\n")
    }

    fn width(&self) -> usize {
        self.lines.first().map_or(0, |line| line.chars().count())
    }
}

fn padded(mut row: String, width: usize) -> String {
    row.push_str(&" ".repeat(width.saturating_sub(row.chars().count())));
    row
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
        // TODO: This is totally weird. I would expect the text "ab\ncd" will keep as a text unit so no "+---+" header inject itself between the text.
        // Why is this implemented this way?
        // ANSWER: The text does stay a unit: "ab" and "cd" are still consecutive rows in
        //         order — the `+---+` is not inserted between them, it sits to their
        //         right. `horizontal` composes row-wise: result row i = (text row i, or
        //         spaces if the text is shorter) + gap + (box row i, or spaces). The box
        //         is 3 rows and the text 2; top-alignment (plan, §horizontal) pairs the
        //         box's bottom border with space-fill for the text's missing row 3. That
        //         is why row 3 is `   +---+`. Row-wise placement is the point of
        //         `horizontal`: rows must align across blocks (the sequence header boxes
        //         share their border rows so lifelines hang underneath). Appending the
        //         box *after* the text as a unit would be `vertical`, not `horizontal`.
        // UPDATE: `Block::text` is a single row by contract, so `"ab\ncd"` can no longer
        //         smuggle a height into the layout; the parsers reject control
        //         characters before any block is built. The two text rows are composed
        //         explicitly with `vertical`.
        let text = Block::vertical([Block::text("ab"), Block::text("cd")], 0);
        let block = Block::horizontal([text, Block::boxed("A")], 1);
        assert_eq!(block.render(), "ab +---+\ncd | A |\n   +---+");
    }

    #[test]
    fn stacks_blocks_vertically_with_a_gap() {
        let block = Block::vertical([Block::boxed("A"), Block::boxed("BC")], 1);
        // TODO: test run:
        // ```
        //  left: "+---+  \n| A |  \n+---+  \n       \n+-----+\n| BC  |\n+-----+"
        // right: "+---+\n| A |\n+---+\n     \n+-----+\n| BC  |\n+-----+"
        // ```
        // Why does the render add 2 whitespace between the "A" block?
        // ANSWER: Rule 2 (rectangular lines). `vertical` left-aligns the blocks and
        //         `Self::new` re-pads every row to the widest row of the stack:
        //         `boxed("BC")` is 7 wide, `boxed("A")` is 5, so each "A" row gains 2
        //         trailing spaces and the blank gap row becomes 7 spaces. The padding is
        //         mandatory, not cosmetic: `horizontal` later pushes block rows verbatim,
        //         assuming every line is exactly the block width and never re-padding,
        //         so a ragged block would silently misalign columns. In the failing run
        //         quoted above, `left` is the actual render (correct per rule 2) and
        //         `right` is this assertion, which still expects the unpadded variant.
        // UPDATE: The assertion below is that padded actual.
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
        // TODO: the test run:
        // ```
        // left: "+---+\n| A |\n+---+\n     \n     \n     \n+---+\n| B |\n+---+\n+---+\n| C |\n+---+"
        // right: "+---+\n| A |\n+---+\n\n\n+---+\n| B |\n+---+\n+---+\n| C |\n+---+"
        // ```
        // Why does the render add 5 whitespace between the new lines?
        // ANSWER: Same rule 2. The widest row of the stack is 5 (`+---+`), and
        //         `Self::new` pads every row — including the blank gap rows and the
        //         spacer's own rows — to that width. The gap rows belong to the composed
        //         block, so the rectangle invariant applies to them like any other row;
        //         leaving them truly empty would make the block ragged. Note that after
        //         composition `spacer(0, n)` and `gap = n` are indistinguishable: both
        //         become full-width blank rows. As above, `left` in the TODO is the
        //         actual render and `right` is this unpadded assertion.
        // UPDATE: The assertion below matches that padded actual.
        assert_eq!(
            blocks.render(),
            "+---+\n| A |\n+---+\n     \n     \n     \n+---+\n| B |\n+---+\n+---+\n| C |\n+---+"
        );
    }

    #[test]
    fn renders_whitespace_verbatim() {
        // TODO: if it renders verbatim, then why does the "b" has 3 trailing whitespace instead of one?
        // ANSWER: "Verbatim" is rule 3: render never *removes* whitespace. But rule 2
        //         runs first: `text` pads every row to the longest row ("  a  ", 5
        //         chars) so the block is rectangular. " b " is 3 chars and receives 2
        //         padding spaces, so it ends as " b" + 3 spaces: the 1 original plus the
        //         2 added. Nothing was removed; 2 spaces were appended. The padding
        //         exists because every row must be equal-width for `horizontal`, which
        //         concatenates rows verbatim without re-padding (see the other answers).
        // UPDATE: `text` is a single row now, so nothing pads it at construction and
        //         what goes in renders out. Multi-row content is composed with
        //         `vertical` instead of one `\n`-joined string.
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
