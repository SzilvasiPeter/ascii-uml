use std::fmt;
use std::fs;

use crate::cli;

#[derive(Debug)]
struct ParseError {
    line: usize,
    message: String,
}

impl ParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self { line, message: message.into() }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            formatter.write_str(&self.message)
        } else {
            write!(formatter, "line {}: {}", self.line, self.message)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Event {
    from: usize,
    to: usize,
    text: String,
}

#[derive(Debug, Default)]
struct Diagram {
    senders: Vec<String>,
    events: Vec<Event>,
}

impl Diagram {
    fn participant_index(&mut self, name: &str) -> usize {
        self.senders.iter().position(|p| p == name).unwrap_or_else(|| {
            self.senders.push(name.to_owned());
            self.senders.len() - 1
        })
    }

    fn parse(input: &str) -> Result<Self, ParseError> {
        let parsed_lines = input
            .lines()
            .enumerate()
            .filter_map(|(index, raw_line)| {
                let line = raw_line.trim();
                (!line.is_empty()).then_some((index + 1, line))
            })
            .map(|(line_number, line)| parse_line(line_number, line))
            .collect::<Result<Vec<_>, _>>()?;

        if parsed_lines.is_empty() {
            return Err(ParseError::new(0, "no messages found in input"));
        }

        let mut diagram = Self::default();
        for line in parsed_lines {
            let left = diagram.participant_index(&line.left);
            let right = diagram.participant_index(&line.right);
            let (from, to) = if line.points_right { (left, right) } else { (right, left) };
            diagram.events.push(Event { from, to, text: line.message });
        }

        Ok(diagram)
    }

    fn render(&self) -> String {
        // `| <name> |` needs 4 border char; `| 1` ensures odd width for a midpoint.
        let box_widths: Vec<usize> =
            self.senders.iter().map(|name| name.chars().count() + 4 | 1).collect();
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
        let box_centers: Vec<usize> = std::iter::once(first_column)
            .chain(spacings.iter().scan(first_column, |position, spacing| {
                *position += spacing;
                Some(*position)
            }))
            .collect();

        let last_center = box_centers.last().expect("column_pos must not be empty");
        let last_box = box_widths.last().expect("box_widths must not be empty");
        let header_width = (last_center + 1) + (last_box / 2 + 1);

        let mut header_rows: [Vec<char>; 3] = core::array::from_fn(|_| vec![' '; header_width]);
        for (index, name) in self.senders.iter().enumerate() {
            let left = box_centers[index] - box_widths[index] / 2;
            header_rows = draw_box(header_rows, left, box_widths[index], name);
        }

        let mut lines: Vec<String> = header_rows.map(String::from_iter).into_iter().collect();
        let empty_row = || columns_row(&box_centers, header_width);
        lines.push(String::from_iter(empty_row()));

        for event in &self.events {
            let text_start = box_centers[event.from.min(event.to)] + 2;
            let mut message_row = empty_row();
            for (offset, character) in event.text.chars().enumerate() {
                message_row[text_start + offset] = character;
            }

            let empty_row = empty_row();
            let arrow_row = draw_arrow(&empty_row, box_centers[event.from], box_centers[event.to]);

            lines.push(String::from_iter(message_row));
            lines.push(String::from_iter(arrow_row));
            lines.push(String::from_iter(empty_row));
        }

        lines.join("\n") + "\n"
    }
}

#[derive(Debug)]
struct ParsedLine {
    left: String,
    right: String,
    points_right: bool,
    message: String,
}

fn parse_line(line_number: usize, line: &str) -> Result<ParsedLine, ParseError> {
    const ARROW_LENGTH: usize = 2;

    let (header, message) = line
        .split_once(':')
        .ok_or_else(|| ParseError::new(line_number, "expected ':' before the description"))?;
    let (arrow_index, points_right) = match (header.find("->"), header.find("<-")) {
        (Some(r), Some(l)) if r < l => (r, true),
        (Some(r), Some(l)) if r > l => (l, false),
        (Some(r), None) => (r, true),
        (None, Some(l)) => (l, false),
        _ => return Err(ParseError::new(line_number, "expected an arrow ('->' or '<-')")),
    };

    let message = message.trim();
    let left = header[..arrow_index].trim();
    let right = header[arrow_index + ARROW_LENGTH..].trim();
    if left.is_empty() || right.is_empty() {
        return Err(ParseError::new(line_number, "arrow cannot have empty sides"));
    }
    if message.is_empty() {
        return Err(ParseError::new(line_number, "description must not be empty"));
    }
    if left == right {
        return Err(ParseError::new(line_number, "a sender cannot message itself"));
    }

    let (left, right, message) = (left.to_owned(), right.to_owned(), message.to_owned());
    Ok(ParsedLine { left, right, points_right, message })
}

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

fn draw_arrow(row: &[char], from_x: usize, to_x: usize) -> Vec<char> {
    let mut res = row.to_vec();
    if from_x < to_x {
        res[from_x + 1..to_x].fill('-');
        res[to_x - 1] = '>';
    } else {
        res[to_x + 1..from_x].fill('-');
        res[to_x + 1] = '<';
    }
    res
}

fn columns_row(column_positions: &[usize], width: usize) -> Vec<char> {
    let mut row = vec![' '; width];
    for &x in column_positions {
        row[x] = '|';
    }
    row
}

pub fn run(args: &[String]) {
    let Some(argument) = args.first() else {
        return cli::usage_error("missing input file");
    };

    match argument.as_str() {
        "-h" | "--help" => print!("{}", cli::usage()),
        "-V" | "--version" => {
            println!("{} sequence {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        }
        option if option.starts_with('-') => {
            cli::usage_error(&format!("unrecognized option '{option}'"));
        }
        path => {
            if args.len() > 1 {
                eprintln!("error: only one input file may be specified");
                return;
            }

            let input = match fs::read_to_string(path) {
                Ok(input) => input,
                Err(error) => {
                    eprintln!("error: failed to read '{path}': {error}");
                    return;
                }
            };

            match Diagram::parse(&input) {
                Ok(diagram) => print!("{}", diagram.render()),
                Err(error) => eprintln!("error: {error}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_arrow_directions() {
        let diagram = Diagram::parse(
            r#"column1 -> column2: do 1
column2 <- column3: do 2
column1 <- column2: do 3
"#,
        )
        .unwrap();

        assert_eq!(diagram.senders, ["column1", "column2", "column3"]);
        assert_eq!(
            diagram.events,
            [
                Event { from: 0, to: 1, text: "do 1".to_owned() },
                Event { from: 2, to: 1, text: "do 2".to_owned() },
                Event { from: 1, to: 0, text: "do 3".to_owned() },
            ]
        );
    }

    #[test]
    fn renders_boxes_and_arrows() {
        let diagram = Diagram::parse("A -> B: hi\nA <- B: back\n").unwrap();
        let output = diagram.render();
        let lines: Vec<&str> = output.lines().collect();

        assert!(lines[0].starts_with("+---+"));
        assert!(lines[1].contains("| A |"));
        assert!(lines[3].contains('|'));
        assert!(lines[5].contains('>'));
        assert!(lines[8].contains('<'));
    }

    #[test]
    fn keeps_short_column_gaps_compact() {
        let diagram = Diagram::parse("aa -> bb: a\nc1 -> c2: aaaaaaa\n").unwrap();
        let output = diagram.render();
        let top = output.lines().next().unwrap();

        assert!(top.starts_with("+-----+ +-----+"));
    }

    #[test]
    fn widens_only_columns_that_need_more_room() {
        let diagram = Diagram::parse("aa -> bb: a\nc1 -> c2: aaaaaaa\n").unwrap();
        let output = diagram.render();
        let top = output.lines().next().unwrap().trim_end();

        assert!(top.ends_with("+-----+   +-----+"));
    }

    #[test]
    fn reports_all_parse_errors() {
        let cases = [
            ("a -> b\n", "line 1: expected ':' before the description"),
            ("a -> b: \n", "line 1: description must not be empty"),
            ("a b: hi\n", "line 1: expected an arrow ('->' or '<-')"),
            (" -> b: hi\n", "line 1: arrow cannot have empty sides"),
            ("a -> : hi\n", "line 1: arrow cannot have empty sides"),
            ("a -> a: hi\n", "line 1: a sender cannot message itself"),
            ("\n \n", "no messages found in input"),
        ];

        for (input, expected) in cases {
            assert_eq!(Diagram::parse(input).unwrap_err().to_string(), expected);
        }
    }
}
