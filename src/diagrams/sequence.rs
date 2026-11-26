use std::fmt;

use super::block::{BLOCK_HEIGHT, Block};

#[derive(Debug)]
enum ParseError {
    EmptyInput,
    MissingColon { ln: usize },
    MissingArrow { ln: usize },
    MultipleArrows { ln: usize },
    EmptySide { ln: usize },
    EmptyMessage { ln: usize },
    SelfMessage { ln: usize },
    ControlCharacter { ln: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ln, message) = match *self {
            Self::EmptyInput => (None, "no messages found in input"),
            Self::MissingColon { ln } => (Some(ln), "expected ':' before the message"),
            Self::MissingArrow { ln } => (Some(ln), "expected one of: --> <-- ..> <.. --- ..."),
            Self::MultipleArrows { ln } => (Some(ln), "expected exactly one arrow"),
            Self::EmptySide { ln } => (Some(ln), "arrow cannot have empty sides"),
            Self::EmptyMessage { ln } => (Some(ln), "message must not be empty"),
            Self::SelfMessage { ln } => (Some(ln), "a sender cannot message itself"),
            Self::ControlCharacter { ln } => (Some(ln), "control characters are not allowed"),
        };

        match ln {
            Some(ln) => write!(formatter, "line {ln}: {message}"),
            None => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    Solid,
    Dotted,
}

impl Line {
    const fn fill(self) -> char {
        match self {
            Self::Solid => '-',
            Self::Dotted => '.',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arrow {
    Head(Line),
    Plain(Line),
}

impl Arrow {
    // Finds overlapping arrow tokens so ambiguous links can be rejected.
    fn splits(link: &str) -> impl Iterator<Item = (&str, (Self, bool), &str)> {
        let ops = [
            ("-->", (Self::Head(Line::Solid), false)),
            ("<--", (Self::Head(Line::Solid), true)),
            ("---", (Self::Plain(Line::Solid), false)),
            ("...", (Self::Plain(Line::Dotted), false)),
            ("..>", (Self::Head(Line::Dotted), false)),
            ("<..", (Self::Head(Line::Dotted), true)),
        ];
        link.char_indices().filter_map(move |(idx, _)| {
            ops.iter()
                .find_map(|&(pat, op)| link[idx..].strip_prefix(pat).map(|rest| (op, rest)))
                .map(|(op, rest)| (link[..idx].trim(), op, rest.trim()))
        })
    }
}

#[derive(Debug)]
struct Event {
    from: usize,
    arrow: Arrow,
    to: usize,
    message: String,
}

#[derive(Debug)]
struct Sequence {
    actors: Vec<String>,
    events: Vec<Event>,
}

impl Sequence {
    fn parse(input: &str) -> Result<Self, ParseError> {
        let mut participants = Vec::new();
        let mut events = Vec::new();
        for (ln, line) in input.lines().enumerate().filter_map(|(index, line)| {
            let line = line.trim();
            (!line.is_empty() && !line.starts_with('#')).then_some((index + 1, line))
        }) {
            let (link, message) = line.split_once(':').ok_or(ParseError::MissingColon { ln })?;
            let message = message.trim();
            let mut splits = Arrow::splits(link);
            let Some((left, (arrow, points_left), right)) = splits.next() else {
                return Err(ParseError::MissingArrow { ln });
            };
            if splits.next().is_some() {
                return Err(ParseError::MultipleArrows { ln });
            } else if left.is_empty() || right.is_empty() {
                return Err(ParseError::EmptySide { ln });
            } else if left == right {
                return Err(ParseError::SelfMessage { ln });
            } else if message.is_empty() {
                return Err(ParseError::EmptyMessage { ln });
            } else if [left, right, message].iter().any(|str| str.chars().any(char::is_control)) {
                return Err(ParseError::ControlCharacter { ln });
            }

            let [left, right] = [left, right].map(|name| {
                participants.iter().position(|p| *p == name).unwrap_or_else(|| {
                    participants.push(name.to_owned());
                    participants.len() - 1
                })
            });
            let (from, to) = if points_left { (right, left) } else { (left, right) };
            events.push(Event { from, to, arrow, message: message.to_owned() });
        }

        events.first().ok_or(ParseError::EmptyInput)?;
        Ok(Self { actors: participants, events })
    }

    fn render(&self) -> String {
        // Box padding adds four columns; odd widths give the lifeline a center column.
        let widths: Vec<_> = self.actors.iter().map(|p| (p.chars().count() + 4) | 1).collect();
        let mut gaps: Vec<_> = widths.windows(2).map(|w| w[0] / 2 + w[1] / 2 + 2).collect();
        for event in &self.events {
            let (left, right) = (event.from.min(event.to), event.from.max(event.to));
            let required = event.message.chars().count() + 3;
            gaps[left] += required.saturating_sub(gaps[left..right].iter().sum());
        }

        let row_width = widths[0] / 2 + gaps.iter().sum::<usize>() + widths.last().unwrap() / 2 + 2;
        let mut centers = vec![widths[0] / 2];
        for gap in gaps {
            centers.push(*centers.last().unwrap() + gap);
        }
        let lefts: Vec<_> = centers.iter().zip(&widths).map(|(c, w)| c - w / 2).collect();

        let mut header = Vec::new();
        for (i, name) in self.actors.iter().enumerate() {
            if i > 0 {
                header.push(Block::spacer(lefts[i] - lefts[i - 1] - widths[i - 1], BLOCK_HEIGHT));
            }
            header.push(Block::boxed(name.as_str()));
        }

        let empty_row = || lifeline_row(&centers, row_width);
        let text_row = |row: Vec<char>| Block::text(String::from_iter(row));
        let mut blocks = vec![Block::horizontal(header, 0), text_row(empty_row())];
        for event in &self.events {
            let (from, to) = (event.from, event.to);
            let message_start = centers[from.min(to)] + 2;
            let mut message_row = empty_row();
            for (slot, ch) in message_row[message_start..].iter_mut().zip(event.message.chars()) {
                *slot = ch;
            }
            blocks.push(text_row(message_row));
            blocks.push(text_row(draw_connection(
                &empty_row(),
                centers[from],
                centers[to],
                event.arrow,
            )));
            blocks.push(text_row(empty_row()));
        }

        Block::vertical(blocks, 0).render()
    }
}

pub fn run(input: &str) -> Result<String, String> {
    Sequence::parse(input).map(|diagram| diagram.render()).map_err(|error| error.to_string())
}

// +---+ +---+
// | a | | b |
// +---+ +---+
//   |     |
fn lifeline_row(columns: &[usize], width: usize) -> Vec<char> {
    let mut row = vec![' '; width];
    for &x in columns {
        row[x] = '|';
    }
    row
}

// +---+ +---+
// | a | | b |
// +---+ +---+
//   |     |
//   |---->|
fn draw_connection(row: &[char], from: usize, to: usize, kind: Arrow) -> Vec<char> {
    let mut result = row.to_vec();
    let (left, right) = (from.min(to), from.max(to));
    let (Arrow::Head(line) | Arrow::Plain(line)) = kind;
    result[left + 1..right].fill(line.fill());
    if matches!(kind, Arrow::Head(_)) {
        let (idx, ch) = if from < to { (right - 1, '>') } else { (left + 1, '<') };
        result[idx] = ch;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_arrow_kind() {
        let input = r"a --> b: solid right
                      b <-- a: solid left
                      a ..> b: dotted right
                      b <.. c: dotted left
                      a --- b: solid
                      b ... c: dotted";
        let diagram = Sequence::parse(input).unwrap();
        let messages: Vec<_> = diagram
            .events
            .iter()
            .map(|message| (message.from, message.to, message.arrow, message.message.as_str()))
            .collect();
        assert_eq!(diagram.actors, ["a", "b", "c"]);
        assert_eq!(
            messages,
            [
                (0, 1, Arrow::Head(Line::Solid), "solid right"),
                (0, 1, Arrow::Head(Line::Solid), "solid left"),
                (0, 1, Arrow::Head(Line::Dotted), "dotted right"),
                (2, 1, Arrow::Head(Line::Dotted), "dotted left"),
                (0, 1, Arrow::Plain(Line::Solid), "solid"),
                (1, 2, Arrow::Plain(Line::Dotted), "dotted"),
            ]
        );
    }

    #[test]
    fn ignores_empty_line_and_comments() {
        let diagram = Sequence::parse("# comment\n\nA-->B:hi\n").unwrap();
        assert_eq!(diagram.events.len(), 1);
    }

    #[test]
    fn renders_boxes() {
        let output = Sequence::parse("A --> B: hi\n").unwrap().render();
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with("+---+"));
        assert!(lines[1].contains("| A |"));
        assert!(lines[2].contains("+---+"));
        assert!(lines[3].contains('|'));
    }

    #[test]
    fn renders_each_arrow_kind() {
        let cases = [
            ("A --> B: x\n", "|---->|"),
            ("A <-- B: x\n", "|<----|"),
            ("A ..> B: x\n", "|....>|"),
            ("A <.. B: x\n", "|<....|"),
            ("A --- B: x\n", "|-----|"),
            ("A ... B: x\n", "|.....|"),
        ];
        for (input, expected) in cases {
            let output = Sequence::parse(input).unwrap().render();
            assert!(output.lines().nth(5).unwrap().contains(expected), "{input}");
        }
    }

    #[test]
    fn renders_the_documented_example_exactly() {
        let input = "student --- master: enter room\n\
                     student --> master: ask the way\n\
                     student <.. master: show the way\n\
                     student ... master: student knows the way\n";
        let expected = concat!(
            "+---------+             +---------+ \n",
            "| student |             | master  | \n",
            "+---------+             +---------+ \n",
            "     |                       |      \n",
            "     | enter room            |      \n",
            "     |-----------------------|      \n",
            "     |                       |      \n",
            "     | ask the way           |      \n",
            "     |---------------------->|      \n",
            "     |                       |      \n",
            "     | show the way          |      \n",
            "     |<......................|      \n",
            "     |                       |      \n",
            "     | student knows the way |      \n",
            "     |.......................|      \n",
            "     |                       |      ",
        );
        assert_eq!(Sequence::parse(input).unwrap().render(), expected);
    }

    #[test]
    fn widens_only_columns_that_need_more_room() {
        let output = Sequence::parse("aa --> bb: a\nc1 --> c2: aaaaaaa\n").unwrap().render();
        let top = output.lines().next().unwrap();
        assert!(top.starts_with("+-----+ +-----+"));
        assert!(top.trim_end().ends_with("+-----+   +-----+"));
    }

    #[test]
    fn allows_printable_unicode() {
        let diagram = Sequence::parse("café --> 世界: 🙂\n").unwrap();
        assert_eq!(diagram.actors, ["café", "世界"]);
        let message = &diagram.events[0];
        assert_eq!((message.from, message.to), (0, 1));
        assert_eq!(message.arrow, Arrow::Head(Line::Solid));
        assert_eq!(message.message, "🙂");
    }

    #[test]
    fn reports_all_parse_errors() {
        let cases = [
            ("a --> b\n", "line 1: expected ':' before the message"),
            ("a --> b: \n", "line 1: message must not be empty"),
            ("a b: hi\n", "line 1: expected one of: --> <-- ..> <.. --- ..."),
            (" --> b: hi\n", "line 1: arrow cannot have empty sides"),
            ("a --> : hi\n", "line 1: arrow cannot have empty sides"),
            ("a --> a: hi\n", "line 1: a sender cannot message itself"),
            ("\n \n", "no messages found in input"),
            ("a --><-- b: hi\n", "line 1: expected exactly one arrow"),
            ("a <--> b: hi\n", "line 1: expected exactly one arrow"),
            ("a --> b --> c: hi\n", "line 1: expected exactly one arrow"),
            ("a ..> b ..> c: hi\n", "line 1: expected exactly one arrow"),
            ("a ---> b: hi\n", "line 1: expected exactly one arrow"),
            ("a --> b: \u{1b}[31mred\n", "line 1: control characters are not allowed"),
            ("a\u{1b} --> b: hi\n", "line 1: control characters are not allowed"),
            ("a --> b\u{7f}: hi\n", "line 1: control characters are not allowed"),
            ("a --> b: \u{9b}31mred\n", "line 1: control characters are not allowed"),
            ("a --> b: line\u{0}break\n", "line 1: control characters are not allowed"),
        ];
        for (input, expected) in cases {
            assert_eq!(Sequence::parse(input).unwrap_err().to_string(), expected);
        }
    }
}
