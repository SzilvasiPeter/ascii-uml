# Diagram block renderer

Implemented in the private `src/diagrams/block.rs` module.
`Block` composes text rectangles.
It does not parse diagram syntax or draw connectors.

## API and rules

- `text` and `boxed` create one-row content and a three-row box.
- `horizontal` composes blocks left to right.
- `vertical` stacks blocks.
- `spacer` creates blank space.
- `render` joins rows without a final newline.
- Composition pads rows to rectangular dimensions.
- Horizontal composition top-aligns blocks.
- Vertical composition left-aligns blocks.
- Widths count Unicode scalar values (`char`), not bytes.
- Whitespace is layout and is preserved, including trailing spaces.

The abstraction is private.
Parsers are responsible for rejecting control characters.
Tests in `block.rs` cover dimensions, composition, and whitespace preservation.
Sequence rendering uses blocks.
Other renderers may adopt them where useful.
Connectors and shared parsing remain separate.
