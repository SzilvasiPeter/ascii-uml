# Block Abstraction Plan

## Goal

Extract the box drawing duplicated across the diagram renderers into a small
`Block` abstraction, then rebuild the sequence renderer on top of it. A block
is a rectangle of text lines that composes horizontally and vertically.

No rendered output changes: parsing, layout math, and diagrams stay
byte-identical. The abstraction stays independent of any diagram grammar.

## Scope

In scope:

- A private `src/diagrams/block.rs` module with independent unit tests.
- `text`, `boxed` (the shared three-row box), `horizontal`, `vertical`,
  `spacer`, `render`.
- Migrating `sequence.rs` to compose its whole output from blocks.

Out of scope:

- A shared canvas type, connectors, arrowheads, lifeline abstractions.
- Parser helpers (`significant_lines`, line-error consolidation).
- The class, usecase, and architecture renderers.
- Cosmetic output cleanup such as trailing-whitespace trimming.
- Misuse hardening of the `Block` API. The module is internal only; input
  validation is the parsers' job. Making `Block` itself reject bad input is
  deferred until — and only if — the crate grows a library API.

## API

`Block` lives in a private module (`mod block;`) and is never exported from
the crate, so its constructors can be infallible and trust their callers.

```rust
pub struct Block {
    // Private. Always rectangular: every line padded to the block width.
    lines: Vec<String>,
}

impl Block {
    pub fn text(source: impl Into<String>) -> Self;
    pub fn boxed(label: impl Into<String>) -> Self;
    pub fn horizontal(blocks: impl IntoIterator<Item = Block>, gap: usize) -> Self;
    pub fn vertical(blocks: impl IntoIterator<Item = Block>, gap: usize) -> Self;
    pub fn spacer(width: usize, height: usize) -> Self;
    pub fn render(&self) -> String;
}
```

## Rules

1. **Char widths.** Sizes are `char` counts, never byte lengths: `世界`
   measures 2 and `🙂` measures 1, matching the existing renderers.
2. **Rectangular lines.** Every block's lines are padded with spaces to the
   block width. Gaps, spacers, and alignment live only in these padded lines.
3. **No trimming, anywhere.** `render` emits the lines verbatim: joined with
   `\n`, no trailing newline. Whitespace is layout — it encodes gaps and
   offsets (a sequence lifeline row is almost all spaces) — so the abstraction
   never removes leading, interior, or trailing spaces. Cosmetic
   trailing-whitespace trimming, if ever wanted, belongs to the final consumer
   (the CLI), not here.
4. **Single-line content by contract.** `text` and `boxed` take exactly one
   row with no control characters. The parsers own that rule — they reject
   `\n`, `\r`, `\t`, and friends before any block is built — so the internal
   constructors stay infallible. Height comes only from composition
   (`vertical`, `boxed`); width comes only from content and `horizontal`.
5. **Compose, then render.** Build the diagram from blocks; call `render` once
   at the end.

## Semantics

### `text`

A single row, kept verbatim, including leading and trailing spaces. `""` is
one empty row. By contract (rule 4) the source is one row without control
characters; the parsers enforce that before any block is built, so the
constructor does not. Multi-row content is composed explicitly:
`Block::vertical([Block::text("ab"), Block::text("cd")], 0)`.

### `boxed`

```text
+---+
| A |
+---+
```

- Width `(label.chars().count() + 4) | 1`, height `3`, borders `+ - |`.
- The label starts at column 2 — one space after the left border — which
  reproduces the current sequence, class, and architecture boxes exactly.
- The label follows the same one-row contract as `text` (rule 4).

### `horizontal`

Places blocks left to right in declaration order with `gap` spaces between
neighbors. Blocks of different heights are top-aligned; missing bottom rows
become spaces. Use `gap = 0` with explicit `spacer`s for per-pair gaps.

### `vertical`

Places blocks top to bottom in declaration order with `gap` blank rows between
neighbors. Blocks of different widths are left-aligned; rule 2 restores the
right padding.

### `spacer`

A blank block of the given size. Its height must match the neighbors it
separates, because `horizontal` top-aligns rows and a shorter block
contributes spaces only to the rows it has:

```rust
Block::horizontal([Block::boxed("A"), Block::spacer(3, 3), Block::boxed("B")], 0)
```

```text
+---+   +---+
| A |   | B |
+---+   +---+
```

### `render`

Joins rows with `\n` and appends no trailing newline (rule 3).

## Sequence migration

`Sequence::render` keeps its layout math (widths, gaps, centers) and its
diagram-specific row builders (`lifeline_row`, `draw_connection`). Changes:

- The local `draw_box` is deleted.
- The header is `Block::horizontal` over the participants with a
  `spacer(edge, 3)` between neighbors, where
  `edge = lefts[i] - lefts[i - 1] - widths[i - 1]` is the current space
  between adjacent box edges. Per-pair gaps vary with message widths, hence
  spacers instead of one uniform gap. The spacer height matches the boxed
  height (3) so it contributes spaces to every header row.
- Every body row (lifeline, message, arrow, blank) is built exactly as today —
  a `Vec<char>` row joined into a `String` — and wrapped in `Block::text`.
- The diagram is `Block::vertical([header, body rows...], 0).render()`. The
  header is one column narrower than the body rows; vertical left-alignment
  re-pads it, preserving today's trailing spaces.
- `render` returns `String` end to end. Block construction is infallible:
  `block` is an internal module and the parsers are the validation boundary —
  control characters never reach a block — so no unwrapping or error plumbing
  appears in renderer code.

## Tests

In `block.rs`:

1. `boxed("A")` renders the exact three-row box, and an even-length label too.
2. Unicode labels measure by chars (rule 1).
3. `horizontal(..., 1)` places two boxes one space apart; mixed heights
   top-align.
4. `vertical(..., 1)` stacks two boxes one blank row apart; mixed widths
   left-align and re-pad.
5. A `vertical` of two `horizontal` rows renders a rows-and-columns grid.
6. `spacer` carries a variable gap through composition.
7. `render` preserves leading, interior, and trailing spaces verbatim.

In `sequence.rs`: existing tests pass unchanged, plus one new test asserting
the complete output of a fixed input (locks byte-identical rendering). The
parsers stay the validation boundary: `reports_all_parse_errors` covers
control-character rejection, and the `Block` constructors are infallible by
contract (rule 4).

## Implementation checklist

1. Add `src/diagrams/block.rs` and a private `mod block;` in
   `src/diagrams/mod.rs`.
2. Implement the six methods with the tests above.
3. Migrate `Sequence::render`; delete the local `draw_box`; add the
   exact-output test.
4. Run `cargo test` and `just check`; diff the CLI output of every
   `examples/*.txt` file against the pre-refactor output (must be identical).

## Follow-ups

1. Migrate `class.rs` (rows of boxes, vertical stacking), then
   `architecture.rs` and `usecase.rs`, adding `framed` for group frames and
   the system boundary: width `max(body_width + 4, title_chars + 5)`, height
   `body_height + 4`, title written as `" title "` from column 1 on the top
   border, body left-aligned with one cell of padding.
2. Share connector and arrow drawing once the static layout is settled.
3. Extract the shared parsing layer (see the TODO in `diagrams/mod.rs`):
   control-character rejection, empty-name checks, blank-line/comment
   filtering, and `line N: ...` error formatting.
