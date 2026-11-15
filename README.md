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
ascii-uml architecture architecture.txt
```

The input must be a regular file no larger than 4 KiB.

Run `ascii-uml --help` for all options and the input format.

### Commands

- `sequence` - create a sequence diagram from a simple text description.
- `class` - create a class diagram from a manual row layout.
- `usecase` - create a use case diagram with one system boundary.
- `architecture` - create an architecture diagram with grouped services.

#### Sequence input format

Each non-empty line describes one message:

```text
sender -> receiver: description
sender <- receiver: description
sender ..> receiver: dotted description
sender <.. receiver: dotted description
sender -- receiver: description without an arrow
sender .. receiver: dotted description without an arrow
```

The `->` and `<-` arrows send a request while `..>` and `<..` arrows response with a message.
The `--` solid line is a direct connection while the `..` dotted line is a loose link or info message.

For example:

```text
student -- master: enter room
student -> master: ask the way
student <.. master: show the way
student .. master: student knows the way
```

produces:

```text
+---------+             +---------+
| student |             | master  |
+---------+             +---------+
     |                       |
     | enter room            |
     |-----------------------|
     |                       |
     | ask the way           |
     |---------------------->|
     |                       |
     | show the way          |
     |<......................|
     |                       |
     | student knows the way |
     |.......................|
     |                       |
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
     A
     |
     |
+---------+     +---------------+
| Master  |*----| SecretScroll  |
+---------+     +---------------+
     ^
     |
     |
+---------+     +---------+
| Student |...|>| Trainee |
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
    o        + Karate Dojo --------------------+        o
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

#### Architecture input format

Each `group` is rendered as one row, top to bottom. Services are listed left to
right and may be separated by commas, semicolons, or newlines. Connections use
`-->` between two declared services and may appear before, between, or after
groups. Groups and services share one namespace.

For example:

```text
group Sources {
    Source A, Source B, Source C
}

group Storage {
    DB One; DB Two; DB Three
}

group Output {
    Brief, Analyst, Delivery
}

Source A --> DB One
Source B --> DB Two
Source C --> DB Three
DB Two --> Brief
Brief --> Analyst
Analyst --> Delivery
```

produces:

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

Services in the same group connect horizontally to adjacent columns.
Services in different groups connect vertically, and only adjacent groups may be connected.
A connection between different columns adds a horizontal jog in the gap.
Blank lines and whole-line `#` comments are ignored.
