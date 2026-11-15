use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
enum ParseError {
    EmptyInput,
    MissingColon { ln: usize },
    MissingOperator { ln: usize },
    MultipleOperators { ln: usize },
    EmptyOperatorSide { ln: usize },
    EmptyDescription { ln: usize },
    SelfMessage { ln: usize },
    ControlCharacter { ln: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ln, message) = match *self {
            Self::EmptyInput => (None, "no messages found in input"),
            Self::MissingColon { ln } => (Some(ln), "expected ':' before the description"),
            Self::MissingOperator { ln } => (Some(ln), "expected one of: --> <-- ..> <.. --- ..."),
            Self::MultipleOperators { ln } => (Some(ln), "expected exactly one operator"),
            Self::EmptyOperatorSide { ln } => (Some(ln), "operator cannot have empty sides"),
            Self::EmptyDescription { ln } => (Some(ln), "description must not be empty"),
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
enum OperatorKind {
    SolidArrowRight,
    SolidArrowLeft,
    DottedArrowRight,
    DottedArrowLeft,
    Solid,
    Dotted,
}

impl OperatorKind {
    const fn is_dotted(self) -> bool {
        matches!(self, Self::DottedArrowRight | Self::DottedArrowLeft | Self::Dotted)
    }

    const fn has_arrow(self) -> bool {
        !matches!(self, Self::Solid | Self::Dotted)
    }

    const fn points_right(self) -> bool {
        !matches!(self, Self::SolidArrowLeft | Self::DottedArrowLeft)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Event {
    left: String,
    operator: OperatorKind,
    right: String,
    text: String,
}

#[derive(Debug, Default)]
struct Sequence {
    events: Vec<Event>,
}

impl Sequence {
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

    fn parse_line(&mut self, ln: usize, line: &str) -> Result<(), ParseError> {
        let (event, description) = line.split_once(':').ok_or(ParseError::MissingColon { ln })?;
        let operators = collect_operators(event);
        if operators.is_empty() {
            return Err(ParseError::MissingOperator { ln });
        }
        if operators.len() != 1 {
            return Err(ParseError::MultipleOperators { ln });
        }
        let (arrow_index, operator_len, operator) = operators[0];

        let left = event[..arrow_index].trim();
        let right = event[arrow_index + operator_len..].trim();
        let description = description.trim();
        if left.is_empty() || right.is_empty() {
            return Err(ParseError::EmptyOperatorSide { ln });
        }
        if left == right {
            return Err(ParseError::SelfMessage { ln });
        }
        if description.is_empty() {
            return Err(ParseError::EmptyDescription { ln });
        }
        if [left, right, description].iter().any(|value| value.chars().any(char::is_control)) {
            return Err(ParseError::ControlCharacter { ln });
        }

        let (left, right, text) = (left.to_owned(), right.to_owned(), description.to_owned());
        self.events.push(Event { left, operator, right, text });
        Ok(())
    }

    fn render(&self) -> String {
        let mut participants: Vec<&str> = Vec::new();
        for name in self.events.iter().flat_map(|event| [event.left.as_str(), event.right.as_str()])
        {
            if !participants.contains(&name) {
                participants.push(name);
            }
        }

        debug_assert!(participants.len() >= 2);
        debug_assert!(!self.events.is_empty(), "diagram must contain at least one event");

        let positions: HashMap<&str, usize> =
            participants.iter().copied().enumerate().map(|(index, name)| (name, index)).collect();
        let endpoints = |event: &Event| {
            let left = positions[event.left.as_str()];
            let right = positions[event.right.as_str()];
            if event.operator.points_right() { (left, right) } else { (right, left) }
        };

        // `| <name> |` needs 4 border char; `| 1` ensures odd width for a midpoint.
        let box_widths: Vec<usize> =
            participants.iter().map(|name| (name.chars().count() + 4) | 1).collect();
        let mut spacings: Vec<usize> =
            box_widths.windows(2).map(|width| width[0] / 2 + width[1] / 2 + 2).collect();
        for event in &self.events {
            let (from, to) = endpoints(event);
            let left = from.min(to);
            let right = from.max(to);

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
        for (index, name) in participants.iter().copied().enumerate() {
            let left = box_centers[index] - box_widths[index] / 2;
            header_rows = draw_box(header_rows, left, box_widths[index], name);
        }

        let mut lines: Vec<String> = header_rows.map(String::from_iter).into_iter().collect();
        let empty_row = || lifeline_row(&box_centers, header_width);
        lines.push(String::from_iter(empty_row()));

        for event in &self.events {
            let (from, to) = endpoints(event);
            let text_start = box_centers[from.min(to)] + 2;
            let mut message_row = empty_row();
            for (offset, character) in event.text.chars().enumerate() {
                message_row[text_start + offset] = character;
            }

            let empty_row = empty_row();
            let arrow_row =
                draw_connection(&empty_row, box_centers[from], box_centers[to], event.operator);

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

fn collect_operators(header: &str) -> Vec<(usize, usize, OperatorKind)> {
    const OPERATORS: [(&str, OperatorKind); 6] = [
        ("..>", OperatorKind::DottedArrowRight),
        ("<..", OperatorKind::DottedArrowLeft),
        ("-->", OperatorKind::SolidArrowRight),
        ("<--", OperatorKind::SolidArrowLeft),
        ("---", OperatorKind::Solid),
        ("...", OperatorKind::Dotted),
    ];

    OPERATORS
        .iter()
        .flat_map(|&(operator, kind)| {
            header.match_indices(operator).map(move |(index, _)| (index, operator.len(), kind))
        })
        .collect()
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
fn draw_connection(row: &[char], from_x: usize, to_x: usize, kind: OperatorKind) -> Vec<char> {
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

    fn event(left: &str, operator: OperatorKind, right: &str, text: &str) -> Event {
        let (left, right, text) = (left.to_owned(), right.to_owned(), text.to_owned());
        Event { left, operator, right, text }
    }

    #[test]
    fn parses_each_operator_kind() {
        let input = r"a --> b: solid right
                      b <-- a: solid left
                      a ..> b: dotted right
                      b <.. c: dotted left
                      a --- b: solid
                      b ... c: dotted";
        let diagram = Sequence::parse(input).unwrap();
        assert_eq!(
            diagram.events,
            [
                event("a", OperatorKind::SolidArrowRight, "b", "solid right"),
                event("b", OperatorKind::SolidArrowLeft, "a", "solid left"),
                event("a", OperatorKind::DottedArrowRight, "b", "dotted right"),
                event("b", OperatorKind::DottedArrowLeft, "c", "dotted left"),
                event("a", OperatorKind::Solid, "b", "solid"),
                event("b", OperatorKind::Dotted, "c", "dotted"),
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
    fn widens_only_columns_that_need_more_room() {
        let output = Sequence::parse("aa --> bb: a\nc1 --> c2: aaaaaaa\n").unwrap().render();
        let top = output.lines().next().unwrap();
        assert!(top.starts_with("+-----+ +-----+"));
        assert!(top.trim_end().ends_with("+-----+   +-----+"));
    }

    #[test]
    fn allows_printable_unicode() {
        let diagram = Sequence::parse("café --> 世界: 🙂\n").unwrap();
        assert_eq!(diagram.events[0].left, "café");
        assert_eq!(diagram.events[0].right, "世界");
        assert_eq!(diagram.events[0].text, "🙂");
    }

    #[test]
    fn reports_all_parse_errors() {
        let cases = [
            ("a --> b\n", "line 1: expected ':' before the description"),
            ("a --> b: \n", "line 1: description must not be empty"),
            ("a b: hi\n", "line 1: expected one of: --> <-- ..> <.. --- ..."),
            (" --> b: hi\n", "line 1: operator cannot have empty sides"),
            ("a --> : hi\n", "line 1: operator cannot have empty sides"),
            ("a --> a: hi\n", "line 1: a sender cannot message itself"),
            ("\n \n", "no messages found in input"),
            ("a --><-- b: hi\n", "line 1: expected exactly one operator"),
            ("a <--> b: hi\n", "line 1: expected exactly one operator"),
            ("a --> b --> c: hi\n", "line 1: expected exactly one operator"),
            ("a ..> b ..> c: hi\n", "line 1: expected exactly one operator"),
            ("a ---> b: hi\n", "line 1: expected exactly one operator"),
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
