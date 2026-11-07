# ascii-uml

A small command-line tool that turns simple text descriptions into ASCII UML diagrams.

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

Run `ascii-uml --help` for all options and the input format.

### Commands

- `sequence` - create a sequence diagram from a simple text description.

## Sequence input format

Each non-empty line describes one message:

```text
sender -> receiver: description
sender <- receiver: description
```

The `->` left arrow sends from the left participant to the right participant. The `<-` right arrow sends from
the right participant to the left participant.

For example:

```text
kid -> mom: give cookie
kid <- mom: get cookie
```

produces:

```text
+-----+       +-----+
| kid |       | mom |
+-----+       +-----+
   |             |
   | give cookie |
   |------------>|
   |             |
   | get cookie  |
   |<------------|
   |             |
```
