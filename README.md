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
ascii-uml class class.txt
ascii-uml usecase usecase.txt
```

The input must be a regular file no larger than 4 KiB.

Run `ascii-uml --help` for all options and the input format.

### Commands

- `sequence` - create a sequence diagram from a simple text description.
- `class` - create a class diagram from a manual row layout.
- `usecase` - create a use case diagram with one system boundary.

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

#### Class input format

Relations may connect classes in the same row if they are adjacent, or
a class to one in the row directly above. The relation operators are:

```text
--|>  generalization
..|>  realization
*--   composition
-->   association
```

The `layout` block assigns classes to rows, top to bottom. Names in a row are comma-separated and rendered left to right:

```text
Master --|> Person
Student ..|> Trainee
Master *-- SecretScroll
Student --> Master

layout {
    Person
    Master, SecretScroll
    Student, Trainee
}
```

produces:

```text
+---------+
| Person  |
+---------+
     ^
     |
     |
+---------+     +---------------+
| Master  |*----| SecretScroll  |
+---------+     +---------------+
     ^
     |
     |
+---------+     +---------+
| Student |....>| Trainee |
+---------+     +---------+
```

#### Use case input format

A use case diagram has exactly one `system` block, actors on either side, and
associations or include/extend dependencies:

```text
system Karate Dojo {
    Practice Basic Kata
    Bow & Observe Etiquette
    Perform Controlled Sparring
    Correct Stance & Form
}

actor White Belt
actor(right) Yellow Belt

White Belt --> Practice Basic Kata
Yellow Belt --> Perform Controlled Sparring
Practice Basic Kata ..> Bow & Observe Etiquette: include
Correct Stance & Form ..> Perform Controlled Sparring: extend
```

`actor <name>`, `actor(left) <name>`, and `actor(right) <name>` place an actor on
the left or right. Colons are not accepted in actor declarations.

Associations (`-->`) connect an actor to a use case. Dependencies (`..>`) connect
two use cases and must end with either `: include` or `: extend`.

produces:

```text
    o        +---------- Karate Dojo ----------+        o
   /|\-----+ |                                 | +-----/|\
   / \     +-+-( Practice Basic Kata )         | |     / \
White Belt   |   |                             | | Yellow Belt
             |   include                       | |
             |   v                             | |
             | ( Bow & Observe Etiquette )     | |
             |                                 | |
             |                                 | |
             |                                 | |
             | ( Perform Controlled Sparring )-+-+
             |     ^                           |
             |     extend                      |
             |     |                           |
             | ( Correct Stance & Form )       |
             |                                 |
             +---------------------------------+
```
