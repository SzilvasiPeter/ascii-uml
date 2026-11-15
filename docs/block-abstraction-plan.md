# Block Abstraction Plan

## Goal

Extract the duplicated box and frame drawing from the diagram renderers into a
small, independent `Block` abstraction. A block is a rendered rectangle of text
that knows its own dimensions and can be composed horizontally or vertically.

This step intentionally contains no canvas, connectors, arrows, lifelines, or
parser changes.

## Scope

### In scope

- A private `src/diagrams/block.rs` module.
- Raw text blocks.
- The common three-row bordered box.
- Bordered frames with a top-left title and one cell of padding.
- Horizontal and vertical stacking.
- Blank spacer blocks for variable gaps.
- Independent unit tests for the abstraction.

### Out of scope

- Public `width` / `height` accessors.
- A shared character canvas or grid.
- Connector merging, arrowheads, ports, or lanes.
- Sequence lifelines.
- Parser helpers such as `significant_lines`.
- Line-error consolidation.
- Refactoring the existing diagram modules.

## Public API

```rust
pub struct Block {
    // Internal only. Lines are rectangular and padded to the block width.
    lines: Vec<String>,
}

impl Block {
    pub fn text(text: impl Into<String>) -> Self;
    pub fn boxed(label: impl Into<String>) -> Self;
    pub fn framed(title: impl Into<String>, body: Block) -> Self;
    pub fn horizontal(blocks: impl IntoIterator<Item = Block>, gap: usize) -> Self;
    pub fn vertical(blocks: impl IntoIterator<Item = Block>, gap: usize) -> Self;
    pub fn spacer(width: usize, height: usize) -> Self;
    pub fn render(&self) -> String;
}
```

`width` and `height` are implementation details for this step. If a later
diagram needs box dimensions to place connectors, accessors can be added then
with a concrete use case.

## Semantics

### `text`

- Accepts a raw string.
- Multi-line input is split into rows on `\n`.
- Width is the longest row in characters; height is the number of rows.
- Short rows are padded internally when composed.

### `boxed`

Renders the common three-row box used by sequence participants, class boxes,
and architecture services:

```text
+---+
| A |
+---+
```

- Width: `(label.chars().count() + 4) | 1`.
- Height: `3`.
- The label starts at `x + 2`, matching the current renderers.
- This preserves current output for both odd- and even-length labels.

### `framed`

Renders a bordered panel with a top-left title and one cell of padding around
the body. The body may be text, a box, another frame, or a stack.

```rust
Block::framed("S", Block::text("( A )"))
```

```text
+ S ----+
|       |
| ( A ) |
|       |
+-------+
```

- Width: `max(body_width + 4, title_chars + 5)`.
- Height: `body_height + 4`.
- Top border: corners are `+`; the title is written as `" title "` starting at
  `x = 1`.
- The body is left-aligned with one space of padding on each side.
- This matches the existing `system` boundary and architecture `group` frames.

### `horizontal`

- Places blocks side by side in declaration order.
- Inserts `gap` spaces between adjacent blocks.
- Top-aligns blocks of different heights.
- Missing rows are filled with spaces.

### `vertical`

- Places blocks top to bottom in declaration order.
- Inserts `gap` blank rows between adjacent blocks.
- Left-aligns blocks of different widths.
- Short rows are padded with spaces.

### `spacer`

Creates a blank block with the requested width and height. It is used to model
variable gaps without adding separate `*_with_gaps` stack methods:

```rust
Block::horizontal(
    [Block::boxed("A"), Block::spacer(3, 0), Block::boxed("B")],
    0,
)
```

### `render`

- Trims trailing whitespace from each row.
- Joins rows with `\n`.
- Does not emit a trailing newline.

## Examples

### Horizontal stack

```rust
Block::horizontal([Block::boxed("A"), Block::boxed("B")], 1)
```

```text
+---+ +---+
| A | | B |
+---+ +---+
```

### Vertical stack

```rust
Block::vertical([Block::boxed("A"), Block::boxed("B")], 1)
```

```text
+---+
| A |
+---+

+---+
| B |
+---+
```

### Framed architecture group

```rust
Block::framed(
    "G",
    Block::horizontal([Block::boxed("A"), Block::boxed("B")], 5),
)
```

```text
+ G --------------+
|                 |
| +---+     +---+ |
| | A |     | B | |
| +---+     +---+ |
|                 |
+-----------------+
```

This is byte-for-byte the static shape currently produced by the architecture
renderer.

## Tests

Add focused tests inside `block.rs`:

1. `boxed("A")` renders the exact three-row box.
2. `horizontal(..., 1)` renders two boxes with one space between them.
3. `vertical(..., 1)` renders two boxes with one blank row between them.
4. `framed("G", horizontal(..., 5))` matches the architecture group shape.
5. Two framed blocks in a vertical stack with a gap verify nesting, alignment,
   and dimensions.
6. A spacer between two boxes verifies explicit variable gaps.

## Implementation Checklist

1. Add `src/diagrams/block.rs`.
2. Add `mod block;` to `src/diagrams/mod.rs`; keep the module private.
3. Implement the seven public methods and private padding/rendering helpers.
4. Add the independent unit tests.
5. Run `cargo fmt --all -- --check`, `cargo test`, and `just check`.
6. Do not modify any diagram renderer in this step.

## Follow-up Steps

1. Migrate static box and frame drawing to `Block`, starting with the
   architecture group frames and service boxes because they map directly to
   `framed(horizontal(boxed(...)))`.
2. Introduce shared parser helpers: `significant_lines`, `line_error`, and the
   `validate_name` consolidation.
3. Add connector and arrow handling separately, after the static layout
   abstraction is settled.

## Notes

- Keep the implementation std-only.
- Use character counts, not byte lengths, for width calculations.
- Preserve deterministic output.
- The abstraction must stay independent of any specific diagram grammar or
  layout.
