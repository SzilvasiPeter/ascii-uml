# Use Case Diagram Feature Plan

## Goal

Add a `usecase` subcommand that renders an ASCII use case diagram from a small
text file. Output is deterministic, uses no new dependencies, and reports
line-numbered errors like `sequence` and `class`.

## Scope

In:

- One named system boundary containing the use cases
- Actors on the left and/or right
- Actor to use case associations
- Include and extend dependencies (rendered `include` / `extend`)
- Automatic columnar layout

Out:

- Actor and use case generalization
- Notes, multiplicities, packages, multiple systems
- User-supplied layout blocks
- Automatic graph layout, crossing minimization, or path finding

## Input

```text
system Online Shop {
  Browse catalog
  Place order
  Pay
}

actor Customer
actor Admin
actor(left) Auditor
actor(right) Payment Gateway

Customer --> Browse catalog
Customer --> Place order
Payment Gateway --> Pay

Browse catalog ..> Pay: include
Place order ..> Pay: extend
```

### Rules

- Exactly one `system <name> {` block is required. `{` is on the same line.
  Each non-empty line inside is one use case; `}` closes the block.
- Blank lines and `#` comments are ignored.
- `actor <name>`, and `actor(left) <name>` place an actor on
  the left. `actor(right) <name>` places it on the right. Names may contain
  spaces.
- `A --> B` is an association. Exactly one endpoint must be an actor and the
  other a use case. Either direction is accepted; no arrowhead is drawn.
- `A ..> B: include` and `A ..> B: extend` are dependencies. Both endpoints
  must be use cases. The label is exactly `include` or `extend`. The arrow
  points from `A` to `B`.
- A dependency label sits two rows below its source when the target is lower
  (row `4*j + 4` for source index `j`), or two rows above it when the target
  is higher (row `4*j`). Two dependencies may not share a label row, so a use
  case can have at most one outgoing dependency in each direction.
- Self relations and duplicate relations (the same actor/use case pair, or the
  same ordered use case pair) are errors.
- The system, actor, and use case names are trimmed. They may contain spaces,
  `-`, and `.`. They may not be empty or contain control characters or
  `{ } # , : | < > ( )`.
- Actors and use cases share one namespace. Every name must be unique.

Return `line N: ...` errors for: missing, duplicate, unclosed, or empty
`system` block; empty system name; no use cases; malformed `actor` line; duplicate
name; unknown endpoint; association between two actors or two use cases;
dependency involving an actor; self or duplicate relation; missing or invalid
dependency label; overlapping dependency label rows; invalid name.

The CLI already enforces the 4 KiB input limit.

## Rendering

### Geometry

All coordinates are non-negative. The boundary starts at row 0.

```text
left actor column | left gutter | system boundary | right gutter | right actor column
```

Use cases are one line each, with three blank rows between consecutive use
cases. The three rows hold a port stem, a label, and an arrow or continuation:

```text
+-------- System --------+
|                        |
|  ( First use case )    |
|                        |
|                        |
|                        |
|  ( Second use case )   |
|                        |
+------------------------+
```

Definitions:

```text
uc_w           = max(name.chars().count() + 4)
n_uc           = number of use cases
n_l, n_r       = number of left/right actors
L, R           = max actor block width on each side (0 if no actors)
left_gutter    = 2 * n_l + 1 if n_l > 0 else 0
boundary_left  = L + left_gutter
uc_left        = boundary_left + 2
text_w         = max(uc_w, 11 if any dependency else 0)
n_lanes        = number of dependencies between non-adjacent use cases
inner_w        = text_w + 2 + 2 * n_lanes
inner_w        = max(inner_w, title.chars().count() + 4)
boundary_w     = inner_w + 2
boundary_right = boundary_left + boundary_w - 1
right_gutter   = 2 * n_r + 1 if n_r > 0 else 0
right_col      = boundary_right + 1 + right_gutter
boundary_h     = 4 * n_uc + 1
actor_h        = 0 if max(n_l, n_r) == 0 else 5 * max(n_l, n_r) - 1
canvas_w       = max(boundary_right + 1, right_col + R if n_r > 0 else 0)
canvas_h       = max(boundary_h, actor_h)
```

Use case `j` is at row `2 + 4 * j`, column `uc_left`. Its width is
`name.chars().count() + 4`, rendered `( name )`.

The title block `" " + title + " "` is centered in the top border:

```text
title_start = boundary_left + 1 + (inner_w - title.chars().count() - 2) / 2
```

`inner_w` is at least `title.chars().count() + 4`, so the title block has at
least one `-` on each side.

### Actor blocks

Each actor is a four-row block, stacked in declaration order per side with one
blank row between blocks. Actors with no associations are still rendered.
Block `i` starts at row `5 * i`. Block width is
`max(3, name.chars().count())`.

```text
   o
  /|\
  / \
Customer
```

The figure is centered in the block: `figure_left = (block_w - 3) / 2`. The
name is centered on the last row. Left blocks start at column 0; right blocks
start at `right_col`.

### Dependency ports and lanes

A downward dependency uses port column `down_x = uc_left + 2`; an upward
dependency uses `up_x = down_x + 2`. The two columns are far enough apart that
a use case can have both an incoming and an outgoing dependency on the same
side.

`text_w` reserves room for the longest label at either port. A dependency
between non-adjacent use cases gets a lane to the right of the text area:

```text
lane(k) = boundary_left + 3 + text_w + 2 * k   // k = 0, 1, ...
```

Here `k` is the dependency's index among non-adjacent dependencies in
declaration order. Lanes stay inside the boundary; the `inner_w` formula above
leaves one padding column after the last lane.

## Routing

Associations use the outer gutters and the left/right sides of use cases.
Dependencies use the inner right corridor and the top/bottom sides. The two
lanes and port sets are independent.

### Associations

Actors are not sorted or aligned with use cases. Actor `i` has:

```text
arms_row   = 5 * i + 1
left_lane  = boundary_left - 2 - 2 * i
right_lane = boundary_right + 2 + 2 * i
```

For each actor:

1. Start at the arms row immediately beside the figure: right of `/|\` for a
   left actor, left of `/|\` for a right actor.
2. Draw a solid horizontal line to the actor's lane.
3. For each connected use case, draw a solid vertical line along the lane to
   the use case row, then a solid horizontal line to the use case:
   - left actor: end just left of `(`;
   - right actor: end just right of `)`.

All associations from one actor share that actor's lane. Different actors have
different lanes. A boundary border crossed by a horizontal line becomes `+`.

### Dependencies

Dependencies connect to the top or bottom of a use case, never to its left or
right side. For a dependency with source row `s` and target row `t`:

```text
downward (t > s): port row s+1, label row s+2, arrow row t-1, arrow 'v'
upward   (t < s): port row s-1, label row s-2, arrow row t+1, arrow '^'
```

If the use cases are adjacent (`abs(source index - target index) == 1`), draw
the dependency directly at its port column: `|` on the port row, the label on
the label row, and the arrow on the arrow row.

Otherwise:

1. Draw `|` at the source port column on the port row, then the label at the
   same column on the label row.
2. After the label, draw `.` to the lane and `+` at the lane.
3. Draw `|` along the lane to the arrow row; put `+` at the lane corner.
4. Draw `.` from the lane to the target port column and put the arrow there.

For example, the include from `Browse catalog` to `Pay` is non-adjacent and
uses a lane; the extend from `Place order` to `Pay` is direct:

```text
( Browse catalog )
  |
include..........+
( Place order )  |
  |              |
extend           |
  v..............+
( Pay )
```

An upward dependency is symmetric; its label starts at `up_x`:

```text
( A )
  ^
extend
  |
( B )
```

Each non-adjacent dependency has its own lane, so dependencies never share a
vertical segment. A lane may cross an association line; the crossing becomes
`+`.

### Drawing

Draw in this order: boundary and use cases, actors, associations, dependencies.

When writing a line into an occupied cell:

- space → write the glyph;
- any horizontal glyph (`-` or `.`) plus any vertical glyph (`|`) → `+`;
- same direction → keep the line;
- `+` is never downgraded.

Never overwrite use case text or parentheses, actor glyphs or names, or
boundary corners. Dependency labels and arrowheads (`v` / `^`) are written
last and win over line glyphs.

Trim trailing whitespace from every line. One `char` is one column, matching
the existing commands.

## Example

Input:

```text
system Online Shop {
  Browse catalog
  Place order
  Pay
}

actor Customer
actor(right) Payment Gateway

Customer --> Browse catalog
Customer --> Place order
Browse catalog ..> Pay: include
Place order ..> Pay: extend
Payment Gateway --> Pay
```

Expected output. Associations enter from the left/right. The include from
`Browse catalog` to `Pay` is non-adjacent and branches to a lane; the extend
from `Place order` to `Pay` is direct. Both arrows enter `Pay` from the top:

```text
   o       +---- Online Shop -----+          o
  /|\----+ |                      | +-------/|\
  / \    +-+-( Browse catalog )   | |       / \
Customer | |   |                  | | Payment Gateway
         | |   include..........+ | |
         | |                    | | |
         +-+-( Place order )    | | |
           |   |                | | |
           |   extend           | | |
           |   v................+ | |
           | ( Pay )--------------+-+
           |                      |
           +----------------------+
```

This output follows the formulas above and is the golden test target.

## Implementation

1. Add `src/diagrams/usecase.rs` with `run(&str) -> Result<String, String>`.
2. Parse the system block, actors, associations, and dependencies.
3. Validate as listed under Input.
4. Compute boundary, use case, dependency-lane, gutter, and actor positions.
5. Render the boundary, use cases, actors, associations, and dependencies.
6. Register `usecase` in `SUBCOMMANDS` and the usage text in `src/cli.rs`.
7. Add `pub mod usecase;` to `src/diagrams/mod.rs`.
8. Update the CLI test from two subcommands to three.
9. Update `README.md` with the command and input format.
10. Add tests: parser/validation errors, default/left/right actors, multiple
    associations per actor, left/right association ports, down/up dependency
    ports, direct neighbor routing, lane routing for non-neighbors, duplicate
    label-row rejection, and one golden output.
11. Run `cargo fmt --all`, `cargo clippy --all-targets`, and `cargo test`.

## Notes

- `sequence` and `class` are unchanged.
- No new dependencies.
- Identical input must produce byte-identical output.
