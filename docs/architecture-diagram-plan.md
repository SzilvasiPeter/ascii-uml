# Architecture diagrams

Implemented in `src/diagrams/architecture.rs`.
See `examples/architecture.txt` and `examples/architecture.out` for a sample.

## Input

```text
group Sources { Source A; Source B }
group Storage { DB One; DB Two }
Source A --> DB One
DB Two --> Source B
```

Each group is a row.
Services are columns in declaration order.
Separate services with semicolons or newlines.
Groups and services share one unique name space.
Connections use exactly one `-->` and must connect declared services.
Connections may appear anywhere in the input.
Blank lines and whole-line `#` comments are ignored.

Names are trimmed and non-empty.
Names cannot contain control characters or `{ } , ; # | < >`.
Empty service items are invalid.
Self-links and duplicate unordered pairs are invalid.
Same-group links must connect adjacent columns.
Cross-group links must connect adjacent groups.
Links may point either way.

## Layout and rendering

Groups render top to bottom.
Services render left to right.
All groups share column widths based on their widest service.
Same-group links are horizontal.
Links between adjacent groups are vertical.
Links in different columns use a horizontal jog.
Each jog gets a lane in declaration order.
Output is deterministic and widths use character counts.
