use std::fmt;

#[derive(Debug)]
enum ParseError {
    EmptyInput,
    MissingColon { line: usize },
    MissingOperator { line: usize },
    MultipleOperators { line: usize },
    EmptyOperatorSide { line: usize },
    EmptyDescription { line: usize },
    SelfMessage { line: usize },
    ControlCharacter { line: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (line, message) = match *self {
            Self::EmptyInput => (None, "no messages found in input"),
            Self::MissingColon { line } => (Some(line), "expected ':' before the description"),
            Self::MissingOperator { line } => (Some(line), "expected '->', '..>', '--', '..'"),
            Self::MultipleOperators { line } => (Some(line), "expected exactly one operator"),
            Self::EmptyOperatorSide { line } => (Some(line), "operator cannot have empty sides"),
            Self::EmptyDescription { line } => (Some(line), "description must not be empty"),
            Self::SelfMessage { line } => (Some(line), "a sender cannot message itself"),
            Self::ControlCharacter { line } => (Some(line), "control characters are not allowed"),
        };

        match line {
            Some(line) => write!(formatter, "line {line}: {message}"),
            None => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    SolidArrow,
    DottedArrow,
    Solid,
    Dotted,
}

impl LineKind {
    const fn is_dotted(self) -> bool {
        matches!(self, Self::DottedArrow | Self::Dotted)
    }

    const fn has_arrow(self) -> bool {
        matches!(self, Self::SolidArrow | Self::DottedArrow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Event {
    from: usize,
    to: usize,
    text: String,
    kind: LineKind,
}

#[derive(Debug, Default)]
struct Sequence {
    participants: Vec<String>,
    events: Vec<Event>,
}

impl Sequence {
    fn participant_index(&mut self, name: &str) -> usize {
        self.participants.iter().position(|p| p == name).unwrap_or_else(|| {
            self.participants.push(name.to_owned());
            self.participants.len() - 1
        })
    }

    fn parse(input: &str) -> Result<Self, ParseError> {
        let mut diagram = Self::default();
        for (index, raw_line) in input.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            diagram.parse_line(index + 1, line)?;
        }

        if diagram.events.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        Ok(diagram)
    }

    fn parse_line(&mut self, line: usize, line_content: &str) -> Result<(), ParseError> {
        const OPERATORS: [(&str, LineKind, bool); 6] = [
            ("..>", LineKind::DottedArrow, true),
            ("<..", LineKind::DottedArrow, false),
            ("->", LineKind::SolidArrow, true),
            ("<-", LineKind::SolidArrow, false),
            ("--", LineKind::Solid, true),
            ("..", LineKind::Dotted, true),
        ];

        let (header, message) =
            line_content.split_once(':').ok_or(ParseError::MissingColon { line })?;
        let Some((arrow_index, operator, kind, points_right)) = find_operator(header, &OPERATORS)
        else {
            return Err(ParseError::MissingOperator { line });
        };
        if find_operator(&header[arrow_index + operator.len()..], &OPERATORS).is_some() {
            return Err(ParseError::MultipleOperators { line });
        }

        let message = message.trim();
        let left = header[..arrow_index].trim();
        let right = header[arrow_index + operator.len()..].trim();
        if left.is_empty() || right.is_empty() {
            return Err(ParseError::EmptyOperatorSide { line });
        }
        if [left, right].iter().any(|value| value.contains(['<', '>'])) {
            return Err(ParseError::MultipleOperators { line });
        }
        if message.is_empty() {
            return Err(ParseError::EmptyDescription { line });
        }
        if left == right {
            return Err(ParseError::SelfMessage { line });
        }
        if [left, right, message].iter().any(|value| value.chars().any(char::is_control)) {
            return Err(ParseError::ControlCharacter { line });
        }

        let left = self.participant_index(left);
        let right = self.participant_index(right);
        let (from, to) = if points_right { (left, right) } else { (right, left) };
        self.events.push(Event { from, to, text: message.to_owned(), kind });
        Ok(())
    }

    fn render(&self) -> String {
        debug_assert!(self.participants.len() >= 2);
        debug_assert!(!self.events.is_empty(), "diagram must contain at least one event");

        // `| <name> |` needs 4 border char; `| 1` ensures odd width for a midpoint.
        let box_widths: Vec<usize> =
            self.participants.iter().map(|name| (name.chars().count() + 4) | 1).collect();
        let mut spacings: Vec<usize> =
            box_widths.windows(2).map(|width| width[0] / 2 + width[1] / 2 + 2).collect();
        for event in &self.events {
            let left = event.from.min(event.to);
            let right = event.from.max(event.to);

            let current: usize = spacings[left..right].iter().sum();
            let required = event.text.chars().count() + 3;
            if current < required {
                spacings[left] += required - current;
            }
        }

        let first_column = box_widths[0] / 2;
        let mut box_centers = vec![first_column];
        for spacing in &spacings {
            box_centers.push(box_centers[box_centers.len() - 1] + spacing);
        }

        let last_center = box_centers[box_centers.len() - 1];
        let last_box = box_widths[box_widths.len() - 1];
        let header_width = (last_center + 1) + (last_box / 2 + 1);

        let mut header_rows: [Vec<char>; 3] = core::array::from_fn(|_| vec![' '; header_width]);
        for (index, name) in self.participants.iter().enumerate() {
            let left = box_centers[index] - box_widths[index] / 2;
            header_rows = draw_box(header_rows, left, box_widths[index], name);
        }

        let mut lines: Vec<String> = header_rows.map(String::from_iter).into_iter().collect();
        let empty_row = || lifeline_row(&box_centers, header_width);
        lines.push(String::from_iter(empty_row()));

        for event in &self.events {
            let text_start = box_centers[event.from.min(event.to)] + 2;
            let mut message_row = empty_row();
            for (offset, character) in event.text.chars().enumerate() {
                message_row[text_start + offset] = character;
            }

            let empty_row = empty_row();
            let arrow_row = draw_connection(
                &empty_row,
                box_centers[event.from],
                box_centers[event.to],
                event.kind,
            );

            lines.push(String::from_iter(message_row));
            lines.push(String::from_iter(arrow_row));
            lines.push(String::from_iter(empty_row));
        }

        lines.join("\n")
    }
}

pub fn run(input: &str) -> Result<String, String> {
    Sequence::parse(input).map(|diagram| diagram.render()).map_err(|error| error.to_string())
}

fn find_operator<'a>(
    header: &str,
    operators: &'a [(&'a str, LineKind, bool)],
) -> Option<(usize, &'a str, LineKind, bool)> {
    for (index, _) in header.char_indices() {
        for &(operator, kind, points_right) in operators {
            if header[index..].starts_with(operator) {
                return Some((index, operator, kind, points_right));
            }
        }
    }
    None
}

// +---+
// | a |
// +---+
fn draw_box(mut header: [Vec<char>; 3], left: usize, width: usize, name: &str) -> [Vec<char>; 3] {
    let right = left + width - 1;

    for row_index in [0, 2] {
        let row = &mut header[row_index];
        row[left] = '+';
        row[right] = '+';
        row[left + 1..right].fill('-');
    }

    let row = &mut header[1];
    row[left] = '|';
    row[right] = '|';
    for (offset, character) in name.chars().enumerate() {
        row[left + 2 + offset] = character;
    }

    header
}

// +---+ +---+
// | a | | b |
// +---+ +---+
//   |     |
fn lifeline_row(column_positions: &[usize], width: usize) -> Vec<char> {
    let mut row = vec![' '; width];
    for &x in column_positions {
        row[x] = '|';
    }
    row
}

// +---+ +---+
// | a | | b |
// +---+ +---+
//   |     |
//   |---->|
fn draw_connection(row: &[char], from_x: usize, to_x: usize, kind: LineKind) -> Vec<char> {
    let mut res = row.to_vec();
    let (left, right) = (from_x.min(to_x), from_x.max(to_x));
    res[left + 1..right].fill(if kind.is_dotted() { '.' } else { '-' });
    if kind.has_arrow() {
        if from_x < to_x {
            res[right - 1] = '>';
        } else {
            res[left + 1] = '<';
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parsed_line_adds_two_participants() {
        let diagram = Sequence::parse("A -> B: hi\n").unwrap();
        assert_eq!(diagram.participants.len(), 2);
    }

    #[test]
    fn parses_both_arrow_directions() {
        let diagram = Sequence::parse(
            r"column1 -> column2: do 1
column2 <- column3: do 2
column1 <- column2: do 3
",
        )
        .unwrap();

        assert_eq!(diagram.participants, ["column1", "column2", "column3"]);
        assert_eq!(
            diagram.events,
            [
                Event { from: 0, to: 1, text: "do 1".to_owned(), kind: LineKind::SolidArrow },
                Event { from: 2, to: 1, text: "do 2".to_owned(), kind: LineKind::SolidArrow },
                Event { from: 1, to: 0, text: "do 3".to_owned(), kind: LineKind::SolidArrow },
            ]
        );
    }

    #[test]
    fn parses_dotted_and_arrowless_lines() {
        let diagram = Sequence::parse(
            r"a ..> b: dotted
b <.. c: dotted back
a -- b: solid
b .. c: dotted plain
",
        )
        .unwrap();

        assert_eq!(diagram.participants, ["a", "b", "c"]);
        assert_eq!(
            diagram.events,
            [
                Event { from: 0, to: 1, text: "dotted".to_owned(), kind: LineKind::DottedArrow },
                Event {
                    from: 2,
                    to: 1,
                    text: "dotted back".to_owned(),
                    kind: LineKind::DottedArrow
                },
                Event { from: 0, to: 1, text: "solid".to_owned(), kind: LineKind::Solid },
                Event { from: 1, to: 2, text: "dotted plain".to_owned(), kind: LineKind::Dotted },
            ]
        );
    }

    #[test]
    fn ignores_empty_line_and_comments() {
        let diagram = Sequence::parse("# comment\n\nA->B:hi\n").unwrap();
        assert_eq!(diagram.participants.len(), 2);
    }

    #[test]
    fn rejects_multiple_operators() {
        let cases = [
            "a -><- b: hi\n",
            "a <--> b: hi\n",
            "a -> b -> c: hi\n",
            "a ..> b ..> c: hi\n",
            "a --> b: hi\n",
        ];

        for input in cases {
            let actual = Sequence::parse(input).unwrap_err().to_string();
            assert_eq!(actual, "line 1: expected exactly one operator");
        }
    }

    #[test]
    fn renders_boxes_and_arrows() {
        let diagram = Sequence::parse("A -> B: hi\nA <- B: back\n").unwrap();
        let output = diagram.render();
        let lines: Vec<&str> = output.lines().collect();

        assert!(lines[0].starts_with("+---+"));
        assert!(lines[1].contains("| A |"));
        assert!(lines[3].contains('|'));
        assert!(lines[5].contains('>'));
        assert!(lines[8].contains('<'));
    }

    #[test]
    fn renders_dotted_and_arrowless_lines() {
        let output =
            Sequence::parse("A ..> B: dotted\nA -- B: solid\nA .. B: plain\n").unwrap().render();
        let lines: Vec<&str> = output.lines().collect();

        assert!(lines[5].contains("....>"));
        assert!(lines[8].contains("-----") && !lines[8].contains('>'));
        assert!(lines[11].contains(".....") && !lines[11].contains('>'));
    }

    #[test]
    fn keeps_short_column_gaps_compact() {
        let diagram = Sequence::parse("aa -> bb: a\nc1 -> c2: aaaaaaa\n").unwrap();
        let output = diagram.render();
        let top = output.lines().next().unwrap();

        assert!(top.starts_with("+-----+ +-----+"));
    }

    #[test]
    fn widens_only_columns_that_need_more_room() {
        let diagram = Sequence::parse("aa -> bb: a\nc1 -> c2: aaaaaaa\n").unwrap();
        let output = diagram.render();
        let top = output.lines().next().unwrap().trim_end();
        assert!(top.ends_with("+-----+   +-----+"));
    }

    #[test]
    fn rejects_control_characters() {
        let cases = [
            "a -> b: \u{1b}[31mred\n",
            "a\u{1b} -> b: hi\n",
            "a -> b\u{7f}: hi\n",
            "a -> b: \u{9b}31mred\n",
            "a -> b: line\u{0}break\n",
        ];

        for input in cases {
            assert_eq!(
                Sequence::parse(input).unwrap_err().to_string(),
                "line 1: control characters are not allowed"
            );
        }
    }

    #[test]
    fn allows_printable_unicode() {
        let diagram = Sequence::parse("café -> 世界: 🙂\n").unwrap();
        assert_eq!(diagram.participants, ["café", "世界"]);
        assert_eq!(diagram.events[0].text, "🙂");
    }

    #[test]
    fn reports_all_parse_errors() {
        let cases = [
            ("a -> b\n", "line 1: expected ':' before the description"),
            ("a -> b: \n", "line 1: description must not be empty"),
            ("a b: hi\n", "line 1: expected '->', '..>', '--', '..'"),
            (" -> b: hi\n", "line 1: operator cannot have empty sides"),
            ("a -> : hi\n", "line 1: operator cannot have empty sides"),
            ("a -> a: hi\n", "line 1: a sender cannot message itself"),
            ("\n \n", "no messages found in input"),
        ];

        for (input, expected) in cases {
            assert_eq!(Sequence::parse(input).unwrap_err().to_string(), expected);
        }
    }
}
