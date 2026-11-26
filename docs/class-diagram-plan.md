# Class diagrams

Implemented in `src/diagrams/class.rs`.
See `examples/class.txt` and `examples/class.out` for a sample.

## Input

```text
Child --|> Parent
Struct ..|> Trait
Owner *-- Part
User --> Service

layout {
    Parent
    Child, Trait
    Part, Owner, User, Service
}
```

The required `layout` block defines every class exactly once.
Rows run top to bottom.
Comma-separated names run left to right.
Blank lines and whole-line `#` comments are ignored.
Relations are generalization (`--|>`), realization (`..|>`),
composition (`*--`), and association (`-->`).
Names cannot be empty.
Names cannot contain control characters or `, { } # - > < * . |`.

## Constraints and rendering

For `A op B`, `B` must be adjacent to `A` in the same row,
or in the row directly above.
Downward and longer edges are unsupported.
Output is deterministic.
Relation routing and glyphs are implemented in `class.rs`.
