# Architecture Diagram Feature Plan

## Goal

Add an `architecture` subcommand that renders ASCII architecture diagrams from a
text file. Use the existing conventions: std-only, deterministic output,
line-numbered errors, the 4 KiB input limit, and
`run(&str) -> Result<String, String>` in `src/diagrams/architecture.rs`.

## Input

```text
group Sources {
    Source A; Source B; Source C
}

group Storage {
    DB One; DB Two; DB Three
}

group Output {
    Brief; Analyst; Delivery
}

Source A --> DB One
Source B --> DB Two
Source C --> DB Three
DB Two --> Brief
Brief --> Analyst
Analyst --> Delivery
```

### Rules

- `group Name {` starts a row; `}` closes it. Groups render top to bottom in
  declaration order.
- A group is one row. Services render left to right in declaration order,
  regardless of line breaks. Separate them with `;` or a newline.
- A service is a bare name. Connections use `A --> B` and have no labels.
- Connections may appear before, between, or after groups. Resolve endpoints
  after parsing.
- Groups and services share one namespace; names must be unique.
- Empty services are errors: `A;;B`, `A;`, and `;A`.
- `#` starts a whole-line comment. Blank lines are ignored.
- `}` must be on its own line. `group` is reserved at the start of a line.

Names are trimmed; internal spaces are preserved. Names must not be empty and
must not contain control characters, `{`, `}`, `,`, `;`, `#`, `|`, `<`, or `>`.
Other printable characters are allowed.

Connections:

- Exactly one `-->`; both endpoints must be declared services.
- No self-connections or duplicate unordered pairs.
- Same group: adjacent columns only (index difference 1).
- Different groups: adjacent groups only (index difference 1). Any column
  distance is allowed.
- Upward, downward, and cyclic connections are allowed.

Sides are inferred from position:

- Same group: horizontal. Right when the target column is larger, left when
  smaller.
- Adjacent groups: vertical. Down when the target group is lower, up when
  higher.
- Different columns add one horizontal jog in the gap.

## Layout

Group index = row; service index = column. All groups share global columns and
start at x = 0.

Definitions (`chars(x) = x.chars().count()`):

```text
service_w(s)  = (chars(s.name) + 4) | 1
COL_GAP       = 5
C             = max services in any group
col_w[j]      = max service_w(s) for services s at column j
col_x[0]      = 2
col_x[j+1]    = col_x[j] + col_w[j] + COL_GAP
col_center[j] = col_x[j] + col_w[j] / 2
content_w     = sum(col_w) + COL_GAP * (C - 1)
group_w       = max(content_w + 4, max_group_name_len + 5)
service_x(s)  = col_x[s.si] + (col_w[s.si] - service_w(s)) / 2
```

- `max_group_name_len` is the longest group name across all groups.
- Every group uses `group_w`; missing trailing columns are blank.

Each group is seven rows:

```text
0  top border; title starts at column 1
1  padding; arrowhead row for downward connections
2  service box top
3  service box text
4  service box bottom
5  padding; arrowhead row for upward connections
6  bottom border
```

Vertical placement:

```text
top[0]   = 0
top[i+1] = top[i] + 7 + gap_h[i]
gap_h[i] = 1 + number of cross-group connections in gap i with different columns
```

Same-column cross-group connections are straight and need no lane. Each
different-column connection gets one lane row in declaration order. The extra
row is the target-side stem.

## Rendering

### Titles

- Write `" Name "` at column 1 of the top border.
- Draw titles last, so they overwrite border and edge glyphs.
- An edge crossing the title is interrupted at that row. A downward edge
  resumes at its arrowhead row below the title; an upward edge resumes in the
  gap above the title.

### Same-group connections

- Horizontal on `group_top + 3`, from the left box's right edge to the right
  box's left edge.
- `>` for left-to-right, `<` for right-to-left. Fill the gap with `-` and put
  the arrow at the target-side end.

### Cross-group connections

- Ports are `col_center[src.si]` and `col_center[tgt.si]`.
- Downward: `src_top + 5` to `tgt_top + 1`; arrow `v`.
- Upward: `src_top + 1` to `tgt_top + 5`; arrow `^`.
- Same column: straight vertical line.
- Different columns: source stem, one horizontal lane row, target stem.
  Corners are `+`.
- Crossing horizontal/vertical lines become `+`.

### Drawing order and output

1. Group borders.
2. Service boxes.
3. Same-group connections.
4. Cross-group stems and lanes.
5. Arrowheads.
6. Group titles.

Collision rules: blank plus line -> line; `-` plus `|` -> `+`; `+` is never
downgraded; text wins over lines; same-direction glyphs keep the existing
glyph.

Trim trailing whitespace, join rows with `\n`, and do not emit a trailing
newline.

## Example

For the input above:

```text
+ Sources ------------------------------------------+
|                                                   |
| +-----------+     +-----------+     +-----------+ |
| | Source A  |     | Source B  |     | Source C  | |
| +-----------+     +-----------+     +-----------+ |
|       |                 |                 |       |
+-------+-----------------+-----------------+-------+
        |                 |                 |
+ Storage ----------------+-----------------+-------+
|       v                 v                 v       |
|  +---------+       +---------+      +-----------+ |
|  | DB One  |       | DB Two  |      | DB Three  | |
|  +---------+       +---------+      +-----------+ |
|                         |                         |
+-------------------------+-------------------------+
        +-----------------+
        |
+ Output -------------------------------------------+
|       v                                           |
|   +-------+        +---------+      +-----------+ |
|   | Brief |------->| Analyst |----->| Delivery  | |
|   +-------+        +---------+      +-----------+ |
|                                                   |
+---------------------------------------------------+
```

`DB Two` jogs left to `Brief`. The `Output` title interrupts the entering
arrow for one row.

## Validation

Return `line N: ...` errors for:

- no groups, malformed/unclosed/empty groups, nested groups, connections inside
  groups, unexpected `}`
- empty services, duplicate names, invalid names
- malformed connections (not exactly one `-->`), empty sides, unknown services,
  self or duplicate pairs
- non-adjacent same-group services, non-adjacent groups

Key messages:

```text
line 12: expected exactly one '-->'
line 15: unknown service 'DB Four'
line 18: services 'Source A' and 'Source C' must be adjacent within group 'Sources'
line 21: groups 'Sources' and 'Output' are not adjacent
```

Reject control characters in names and use `escape_debug` in diagnostics.

## Implementation

1. Add `src/diagrams/architecture.rs` with `run`, parsing, validation, layout,
   rendering, and unit tests. Parse groups/services and raw connections first,
   then resolve endpoints.
2. Add `pub mod architecture;` to `src/diagrams/mod.rs`.
3. Update `src/cli.rs`: add the subcommand, change `{:<10}` to `{:<13}`, add
   the architecture format block, and update tests for four subcommands,
   usage, and running the command.
4. Update `README.md` with usage, the commands list, and an architecture input
   format section.
5. Add `exmaples/architecture.txt` with the example input.
6. Add tests: golden output; semicolon/newline separators; empty items;
   global columns; same-group connections in both directions; cross-group
   downward/upward straight and jog connections; multiple jogs; title
   overwrite; comments; Unicode names; the validation matrix; determinism.
7. Run `cargo fmt --all -- --check`, clippy, `cargo test`, and `just coverage`
   (at least 80% line coverage).

## Notes

- Use declaration-order vectors for layout; use maps only for name lookup, so
  output is deterministic.
- Keep drawing helpers private to `architecture.rs`.
