# Use case diagrams

Implemented in `src/diagrams/usecase.rs`.
See `examples/usecase.txt` and `examples/usecase.out` for a sample.

## Input

```text
system Online Shop {
    Browse catalog
    Place order
}
actor Customer
actor(right) Payment Gateway
Customer --> Browse catalog
Browse catalog ..> Place order: include
```

Exactly one non-empty `system` block is required; its name is optional.
Each row defines one use case. Semicolons or newlines start rows; use case rows do not support columns.
Actors default to the left.
Use `actor(left)` or `actor(right)` to choose a side.
`-->` associates an actor with a use case.
`..>` connects use cases and requires the label `include` or `extend`.
Blank lines and whole-line `#` comments are ignored.
System, actor, and use case names share one unique name space.
Names cannot contain control characters or `{ } # , : | < > ( )`.

## Constraints and rendering

Self-relations and duplicate relations are invalid.
Associations must connect one actor and one use case.
Dependencies must connect two use cases.
Dependencies point from source to target.
They reserve a label row above or below their source.
Two labels cannot share a row.
One system boundary contains all use cases.
Actors render outside the boundary on either side.
Layout and routing are deterministic and implemented in `usecase.rs`.
