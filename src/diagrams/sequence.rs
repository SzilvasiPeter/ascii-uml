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

    const fn right_to_left(self) -> bool {
        matches!(self, Self::DottedArrowLeft | Self::SolidArrowLeft)
    }

    // Yields `(left, operator, right)` for every operator occurrence in `header`.
    // The scan goes position by position so overlapping occurrences are found.
    fn splits(header: &str) -> impl Iterator<Item = (&str, Self, &str)> {
        let ops = [
            ("-->", Self::SolidArrowRight),
            ("<--", Self::SolidArrowLeft),
            ("---", Self::Solid),
            ("...", Self::Dotted),
            ("..>", Self::DottedArrowRight),
            ("<..", Self::DottedArrowLeft),
        ];
        header.char_indices().filter_map(move |(idx, _)| {
            ops.iter()
                .find_map(|&(pat, op)| header[idx..].strip_prefix(pat).map(|rest| (op, rest)))
                .map(|(op, rest)| (header[..idx].trim(), op, rest.trim()))
        })
    }
}

#[derive(Debug)]
struct Message {
    from: usize,
    to: usize,
    operator: OperatorKind,
    text: String,
}

#[derive(Debug)]
struct Sequence {
    participants: Vec<String>,
    messages: Vec<Message>,
}

impl Sequence {
    fn parse(input: &str) -> Result<Self, ParseError> {
        let mut participants = Vec::new();
        let mut messages = Vec::new();
        for (ln, line) in input.lines().enumerate().filter_map(|(index, line)| {
            let line = line.trim();
            (!line.is_empty() && !line.starts_with('#')).then_some((index + 1, line))
        }) {
            let (header, text) = line.split_once(':').ok_or(ParseError::MissingColon { ln })?;
            let text = text.trim();
            let mut splits = OperatorKind::splits(header);
            let Some((left, operator, right)) = splits.next() else {
                return Err(ParseError::MissingOperator { ln });
            };
            if splits.next().is_some() {
                return Err(ParseError::MultipleOperators { ln });
            }
            if left.is_empty() || right.is_empty() {
                return Err(ParseError::EmptyOperatorSide { ln });
            } else if left == right {
                return Err(ParseError::SelfMessage { ln });
            } else if text.is_empty() {
                return Err(ParseError::EmptyDescription { ln });
            } else if [left, right, text].iter().any(|value| value.chars().any(char::is_control)) {
                return Err(ParseError::ControlCharacter { ln });
            }

            let [left, right] = [left, right].map(|name| {
                participants.iter().position(|p| *p == name).unwrap_or_else(|| {
                    participants.push(name.to_owned());
                    participants.len() - 1
                })
            });
            let (from, to) = if operator.right_to_left() { (right, left) } else { (left, right) };
            messages.push(Message { from, to, operator, text: text.to_owned() });
        }

        messages.first().ok_or(ParseError::EmptyInput)?;
        Ok(Self { participants, messages })
    }

    fn render(&self) -> String {
        // `| <name> |` needs 4 border char; `| 1` ensures odd width for a midpoint.
        let box_widths: Vec<usize> =
            self.participants.iter().map(|name| (name.chars().count() + 4) | 1).collect();
        let mut spacings: Vec<usize> =
            box_widths.windows(2).map(|width| width[0] / 2 + width[1] / 2 + 2).collect();
        for message in &self.messages {
            let (from, to) = (message.from, message.to);
            let left = from.min(to);
            let right = from.max(to);

            let current: usize = spacings[left..right].iter().sum();
            let required = message.text.chars().count() + 3;
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

        for message in &self.messages {
            let (from, to) = (message.from, message.to);
            let text_start = box_centers[from.min(to)] + 2;
            let mut message_row = empty_row();
            for (offset, character) in message.text.chars().enumerate() {
                message_row[text_start + offset] = character;
            }

            let empty_row = empty_row();
            let arrow_row =
                draw_connection(&empty_row, box_centers[from], box_centers[to], message.operator);

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

    #[test]
    fn parses_each_operator_kind() {
        let input = r"a --> b: solid right
                      b <-- a: solid left
                      a ..> b: dotted right
                      b <.. c: dotted left
                      a --- b: solid
                      b ... c: dotted";
        let diagram = Sequence::parse(input).unwrap();
        let messages: Vec<_> = diagram
            .messages
            .iter()
            .map(|message| (message.from, message.to, message.operator, message.text.as_str()))
            .collect();
        assert_eq!(
            messages,
            [
                (0, 1, OperatorKind::SolidArrowRight, "solid right"),
                (0, 1, OperatorKind::SolidArrowLeft, "solid left"),
                (0, 1, OperatorKind::DottedArrowRight, "dotted right"),
                (2, 1, OperatorKind::DottedArrowLeft, "dotted left"),
                (0, 1, OperatorKind::Solid, "solid"),
                (1, 2, OperatorKind::Dotted, "dotted"),
            ]
        );
        assert_eq!(diagram.participants, ["a", "b", "c"]);
    }

    #[test]
    fn ignores_empty_line_and_comments() {
        let diagram = Sequence::parse("# comment\n\nA-->B:hi\n").unwrap();
        assert_eq!(diagram.messages.len(), 1);
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
        assert_eq!(diagram.participants, ["café", "世界"]);
        let message = &diagram.messages[0];
        assert_eq!((message.from, message.to), (0, 1));
        assert_eq!(message.operator, OperatorKind::SolidArrowRight);
        assert_eq!(message.text, "🙂");
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
