use super::block::{Block, canvas, render_canvas_padded};
use super::parse_document;

super::run!(Diagram);

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
struct Diagram {
    actors: Vec<String>,
    events: Vec<Event>,
}

impl Diagram {
    fn parse(input: &str) -> Result<Self, String> {
        let document = parse_document(input, "")?;
        if !document.blocks.is_empty() {
            return Err("no messages found in input".to_owned());
        }
        let mut participants = Vec::new();
        let mut events = Vec::new();
        for (ln, statement) in document.statements {
            let line = statement.as_str();
            let (connection, message) = line
                .split_once(':')
                .ok_or_else(|| format!("line {ln}: expected ':' before the message"))?;
            let message = message.trim();
            let mut splits = Arrow::splits(connection);
            let Some((left, (arrow, points_left), right)) = splits.next() else {
                return Err(format!("line {ln}: expected one of: --> <-- ..> <.. --- ..."));
            };
            if splits.next().is_some() {
                return Err(format!("line {ln}: expected exactly one arrow"));
            } else if left.is_empty() || right.is_empty() {
                return Err(format!("line {ln}: arrow cannot have empty sides"));
            } else if left == right {
                return Err(format!("line {ln}: a sender cannot message itself"));
            } else if message.is_empty() {
                return Err(format!("line {ln}: message must not be empty"));
            } else if [left, right, message].iter().any(|str| str.chars().any(char::is_control)) {
                return Err(format!("line {ln}: control characters are not allowed"));
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

        if events.is_empty() {
            return Err("no messages found in input".to_owned());
        }
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

        let mut canvas = canvas(row_width, 4 + 3 * self.events.len());
        for (i, name) in self.actors.iter().enumerate() {
            Block::boxed(name.as_str()).draw_on(&mut canvas, lefts[i], 0);
        }
        for row in canvas.iter_mut().skip(3) {
            for &x in &centers {
                row[x] = '|';
            }
        }
        for (index, event) in self.events.iter().enumerate() {
            let y = 4 + 3 * index;
            let start = centers[event.from.min(event.to)] + 2;
            for (slot, ch) in canvas[y][start..].iter_mut().zip(event.message.chars()) {
                *slot = ch;
            }
            draw_connection(
                &mut canvas[y + 1],
                centers[event.from],
                centers[event.to],
                event.arrow,
            );
        }

        render_canvas_padded(&canvas)
    }
}

// +---+ +---+
// | a | | b |
// +---+ +---+
//   |     |
//   |---->|
fn draw_connection(row: &mut [char], from: usize, to: usize, kind: Arrow) {
    let (left, right) = (from.min(to), from.max(to));
    let (Arrow::Head(line) | Arrow::Plain(line)) = kind;
    row[left + 1..right].fill(line.fill());
    if matches!(kind, Arrow::Head(_)) {
        let (idx, ch) = if from < to { (right - 1, '>') } else { (left + 1, '<') };
        row[idx] = ch;
    }
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
        let diagram = Diagram::parse(input).unwrap();
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
        let diagram = Diagram::parse("# comment\n\nA-->B:hi\n").unwrap();
        assert_eq!(diagram.events.len(), 1);
    }

    #[test]
    fn renders_boxes() {
        let output = Diagram::parse("A --> B: hi\n").unwrap().render();
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
            let output = Diagram::parse(input).unwrap().render();
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
        assert_eq!(Diagram::parse(input).unwrap().render(), expected);
    }

    #[test]
    fn widens_only_columns_that_need_more_room() {
        let output = Diagram::parse("aa --> bb: a\nc1 --> c2: aaaaaaa\n").unwrap().render();
        let top = output.lines().next().unwrap();
        assert!(top.starts_with("+-----+ +-----+"));
        assert!(top.trim_end().ends_with("+-----+   +-----+"));
    }

    #[test]
    fn allows_printable_unicode() {
        let diagram = Diagram::parse("café --> 世界: 🙂\n").unwrap();
        assert_eq!(diagram.actors, ["café", "世界"]);
        let message = &diagram.events[0];
        assert_eq!((message.from, message.to), (0, 1));
        assert_eq!(message.arrow, Arrow::Head(Line::Solid));
        assert_eq!(message.message, "🙂");
    }

    #[test]
    fn reports_all_parse_errors() {
        let cases = [
            ("}\n", "line 1: unexpected '}'"),
            ("a --> b\n", "line 1: expected ':' before the message"),
            ("a --> b: \n", "line 1: message must not be empty"),
            ("a b: hi\n", "line 1: expected one of: --> <-- ..> <.. --- ..."),
            (" --> b: hi\n", "line 1: arrow cannot have empty sides"),
            ("a --> : hi\n", "line 1: arrow cannot have empty sides"),
            ("a --> a: hi\n", "line 1: a sender cannot message itself"),
            ("\n \n", "no messages found in input"),
            ("{ A }\n", "no messages found in input"),
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
            assert_eq!(Diagram::parse(input).unwrap_err(), expected);
        }
    }
}
