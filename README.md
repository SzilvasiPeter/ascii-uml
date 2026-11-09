# ascii-uml

[![crates.io](https://img.shields.io/crates/v/ascii-uml)](https://crates.io/crates/ascii-uml)
[![coverage](https://img.shields.io/endpoint?url=https://szilvasipeter.github.io/ascii-uml/badge.json)](https://szilvasipeter.github.io/ascii-uml/html/index.html)
![forbids-unsafe](https://img.shields.io/badge/forbids-unsafe-blue)
![ci](https://github.com/SzilvasiPeter/ascii-uml/actions/workflows/ci.yml/badge.svg)
![cd](https://github.com/SzilvasiPeter/ascii-uml/actions/workflows/cd.yml/badge.svg)

A small command-line tool that turns simple text into ASCII UML diagrams.

Inspired by:
- https://github.com/rust-lang/bors/blob/main/docs/design.md
- https://github.com/pylover/adia

## Install

```sh
cargo install ascii-uml
```

## Usage

```sh
ascii-uml sequence sequence.txt
```

The input must be a regular file no larger than 4 KiB.

Run `ascii-uml --help` for all options and the input format.

### Commands

- `sequence` - create a sequence diagram from a simple text description.

#### Sequence input format

Each non-empty line describes one message:

```text
sender -> receiver: description
sender <- receiver: description
```

The `->` left arrow sends from the left participant to the right participant. The `<-` right arrow sends from
the right participant to the left participant.

For example:

```text
student -> master: ask the way
student <- master: show the way
```

produces:

```text
+---------+    +---------+
| student |    | master  |
+---------+    +---------+
     |              |
     | ask the way  |
     |------------->|
     |              |
     | show the way |
     |<-------------|
     |              |
```
