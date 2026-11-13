# Class Diagram Feature Plan

## Goal

Add a simplified `class` subcommand to `ascii-uml` that renders ASCII class
diagrams from text. The feature targets Rust-specific relationships and uses a
user-supplied row layout instead of automatic graph layout.

## Scope

### In scope

- Classes rendered by name only (no fields, types, parameters, or methods)
- Manual layout: the user assigns classes to rows and controls left-to-right order
- Four Rust-specific relationships:
  - generalization (`trait Child: Parent`)
  - realization (`impl Trait for Struct`)
  - composition (owned field)
  - association (borrowed reference)
- Orthogonal edge routing for same-row and one-row-up connections
- Deterministic output and line-numbered errors, matching the `sequence` command

### Out of scope

- Automatic layout (no Sugiyama, ranking, barycenter ordering, or cycle handling)
- Edges spanning more than one row
- Outer-margin routing, dummy nodes, or pathfinding around boxes
- Members, types, methods, multiplicities, labels, or stereotypes
- `struct`/`trait` declarations and node-kind validation
- Packages or namespaces
- Width caps and edge-list fallback

## Input format

```text
# rows, top to bottom
layout {
  Entity
  Animal, Pet
  Collar, Dog, Owner
}

Animal --|> Entity
Dog ..|> Animal
Dog ..|> Pet
Dog *-- Collar
Dog --> Owner
```

### Layout block

- `layout {}` is mandatory and defines the full class set.
- Rows are top-to-bottom; names are comma-separated and left-to-right.
- A trailing comma is allowed:

  ```text
  layout {
    Entity,
    Animal, Pet,
    Collar, Dog, Owner,
  }
  ```

- Blank lines and `#` comments are ignored.
- Class names must not contain control characters, `,`, `{`, `}`, `#`, or
  relation operators.
- Every class must appear exactly once.

### Relations

Left-hand side is the source; right-hand side is the target/head side. For
composition, the diamond stays on the source.

| Syntax   | Relation       | Rust meaning                | Line style | Head             |
| -------- | -------------- | --------------------------- | ---------- | ---------------- |
| `--\|>`  | Generalization | `trait Child: Parent`       | solid      | `^` at target    |
| `..\|>`  | Realization    | `impl Trait for Struct`     | dotted     | `^` at target    |
| `*--`    | Composition    | `struct S { f: T }`         | solid      | `*` at source    |
| `-->`    | Association    | `struct S<'a> { f: &'a T }` | solid      | `^`/`<`/`>` at target |

## Edge placement rules (locked)

For every relation `A op B`, `B` is the placement target. The same rule applies
to all four relation kinds, including composition.

`B` is allowed only if it is:

1. in the **same row** and **immediately adjacent** to `A`, or
2. exactly **one row above** `A`.

Everything else is an error:

- `B` more than one row above `A`
- `B` in the same row but not adjacent
- `B` in a row below `A` (downward edges are unsupported)

These rules guarantee that no edge ever passes through a class box and remove
the need for general pathfinding.

## Rendering

### Boxes

- Width: `(name.len() + 4) | 1` (odd width gives every box a center column)
- Height: 3 rows
- Shape:

  ```text
  +-------+
  |  Dog  |
  +-------+
  ```

### Row placement

- Pack boxes left-to-right with a fixed gap.
- Left-align each row at column 0.
- Row order is authoritative; horizontal order within a row is authoritative.

### Same-row edges

Draw a direct horizontal line between the two adjacent boxes in the middle row
of the boxes.

```text
+----------+      +----------+      +---------+
|  Collar  |-----*|   Dog    |----->|  Owner  |
+----------+      +----------+      +---------+
```

MVP restriction: at most one relation per adjacent pair in the same row. A
second relation between the same pair is an error.

### Upward edges

1. Reserve a channel between the two rows.
2. Assign each edge a horizontal lane using greedy interval packing.
3. Draw vertically from the source box top into the channel.
4. Draw horizontally across the channel in the assigned lane.
5. Draw vertically up to the target box bottom.
6. Place the head glyph on the row immediately below the target box.

### Glyphs

| Purpose             | Glyph |
| ------------------- | ----- |
| Solid vertical      | `\|`  |
| Solid horizontal    | `-`   |
| Dotted vertical     | `:`   |
| Dotted horizontal   | `.`   |
| Corner              | `+`   |
| Upward head         | `^`   |
| Left/right head     | `<` / `>` |
| Composition diamond | `*`   |

### Ports

- Multiple edges on the same box side receive distinct x offsets.
- If a box has fewer available columns than needed ports, widen the box.
- Ports must stay inside the box border.

## Example output

Input:

```text
layout {
  Entity
  Animal, Pet
  Collar, Dog, Owner
}

Animal --|> Entity
Dog ..|> Animal
Dog ..|> Pet
Dog *-- Collar
Dog --> Owner
```

Output (schematic; the `Dog` → `Pet` edge uses a second lane in the channel):

```text
          +----------+
          |  Entity  |
          +----------+
               ^
               |
     +----------+      +----------+
     |  Animal  |      |   Pet    |
     +----------+      +----------+
            ^                ^
            :                :
            :                :
   +----------+      +----------+      +---------+
   |  Collar  |-----*|   Dog    |----->|  Owner  |
   +----------+      +----------+      +---------+
```

## Validation

Return line-numbered errors in the style of the `sequence` command:

- missing or empty layout block
- duplicate class name in layout
- relation endpoint missing from layout
- self-relation
- same-row target not immediately adjacent
- target not in the same row or exactly one row above
- invalid relation operator
- invalid class name (control characters or reserved characters)
- multiple same-row relations between the same pair
- input larger than 4 KiB (reuse the existing file-size guard)

## Implementation checklist

1. Add `src/class.rs` exposing `run(&str) -> Result<String, String>`.
2. Parse the `layout {}` block and relation lines.
3. Validate names, adjacency, and direction.
4. Compute box widths and row positions; render boxes.
5. Render same-row adjacent edges.
6. Implement channel routing for one-row-up edges.
7. Implement port and lane assignment.
8. Register the `class` subcommand in `SUBCOMMANDS` and the usage text in
   `src/cli.rs`.
9. Add tests:
   - parser errors
   - duplicate and unknown class names
   - same-row non-adjacent rejection
   - downward-edge rejection
   - multi-row-edge rejection
   - same-row rendering
   - upward routing
   - golden output for the full example

## Notes

- The `sequence` command and format are unchanged.
- The class renderer should reject control characters consistently with the
  sequence renderer.
- Determinism matters: identical input must produce byte-identical output.
