# ascii-uml

[![crates.io](https://img.shields.io/crates/v/ascii-uml)](https://crates.io/crates/ascii-uml)
[![coverage](https://img.shields.io/endpoint?url=https://szilvasipeter.github.io/ascii-uml/badge.json)](https://szilvasipeter.github.io/ascii-uml/html/index.html)
![forbids-unsafe](https://img.shields.io/badge/forbids-unsafe-blue)
![ci](https://github.com/SzilvasiPeter/ascii-uml/actions/workflows/ci.yml/badge.svg)
![cd](https://github.com/SzilvasiPeter/ascii-uml/actions/workflows/cd.yml/badge.svg)

A small CLI that turns text files into ASCII UML diagrams.

## Install and use

```sh
cargo install ascii-uml
ascii-uml sequence examples/sequence.txt
ascii-uml class examples/class.txt
ascii-uml usecase examples/usecase.txt
ascii-uml architecture examples/architecture.txt
```

Input must be a regular file no larger than 4 KiB.
Run `ascii-uml --help` for options and format examples.
See [examples/README.md](examples/README.md) for inputs and rendered diagrams.

## Commands

- `sequence`: messages between participants use `-->`, `<--`, `..>`, `<..`, `---`, or `...`.
- `class`: relations use `--|>`, `..|>`, `*--`, or `-->`.
  A `layout` block sets row and column order.
- `usecase`: one `system` block contains the use cases.
  Actors connect with `-->`; dependencies use `..>` with `include` or `extend`.
- `architecture`: `group` blocks list services in rows.
  Connect services with `-->`.

Blank lines and whole-line `#` comments are ignored.
Names may contain spaces.
Command-specific validation and layout rules are shown by `--help`.
