use std::collections::{HashMap, HashSet};

use super::block::{Block, arrow_h, canvas, draw_hline, draw_vline, render_canvas};
use super::{parse_document, single_block};

super::run!(Diagram);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Generalization,
    Realization,
    Composition,
    Association,
}

impl Kind {
    fn line_chars(self) -> (char, char) {
        if self == Self::Realization { (':', '.') } else { ('|', '-') }
    }
}

#[derive(Debug)]
struct Relation {
    source: String,
    target: String,
    kind: Kind,
    line: usize,
}

#[derive(Debug)]
struct Diagram {
    rows: Vec<Vec<String>>,
    relations: Vec<Relation>,
    layout_name: String,
}

#[derive(Debug, Clone, Copy)]
struct Coord {
    row: usize,
    col: usize,
    x: usize,
    width: usize,
}

impl Diagram {
    fn parse(input: &str) -> Result<Self, String> {
        let document = parse_document(input, "layout")?;
        let layout = single_block(&document, "layout")?;
        if !layout.name.is_empty() {
            validate_name(&layout.name)?;
        }
        let rows: Vec<Vec<String>> = layout
            .rows
            .iter()
            .map(|r| r.iter().map(|(n, _)| validate_name(n).map(|()| n.clone())).collect())
            .collect::<Result<_, String>>()?;
        let relations = document
            .statements
            .iter()
            .map(|(l, t)| parse_relation(*l, t))
            .collect::<Result<Vec<_>, _>>()?;
        if rows.is_empty() {
            return Err("layout block is empty".into());
        }

        let mut coords = HashMap::new();
        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, name) in row.iter().enumerate() {
                if coords.insert(name.as_str(), (row_index, column_index)).is_some() {
                    return Err("duplicate class".into());
                }
            }
        }

        let mut same_row_pairs = HashSet::new();
        for rel in &relations {
            let (line, source, target) = (rel.line, &rel.source, &rel.target);
            let (Some(&src), Some(&dst)) =
                (coords.get(source.as_str()), coords.get(target.as_str()))
            else {
                let name = if coords.contains_key(source.as_str()) { target } else { source };
                return Err(format!("line {line}: unknown class '{name}'"));
            };
            if source == target {
                return Err(format!("line {line}: a class cannot relate to itself"));
            } else if src.0 == dst.0 && src.1.abs_diff(dst.1) != 1 {
                let msg = format!("target '{target}' not adjacent to '{source}' in same row");
                return Err(format!("line {line}: {msg}"));
            } else if src.0 == dst.0 {
                let pair = if src.1 < dst.1 {
                    (source.as_str(), target.as_str())
                } else {
                    (target.as_str(), source.as_str())
                };
                if !same_row_pairs.insert(pair) {
                    let msg = format!("multiple relations between '{source}' and '{target}'");
                    return Err(format!("line {line}: {msg}"));
                }
            } else if src.0 != dst.0 + 1 {
                let msg = format!("target '{target}' must be in same/previous row as '{source}'");
                return Err(format!("line {line}: {msg}"));
            }
        }

        Ok(Self { rows, relations, layout_name: layout.name.clone() })
    }

    fn render(&self) -> String {
        const GAP: usize = 5;
        let mut positions = HashMap::new();
        let mut canvas_width = 0;
        for (row_index, row) in self.rows.iter().enumerate() {
            let widths: Vec<_> = row.iter().map(|n| Block::boxed(n.as_str()).width()).collect();
            let mut x = 0;
            for (column_index, (name, &width)) in row.iter().zip(&widths).enumerate() {
                let position = Coord { row: row_index, col: column_index, x, width };
                positions.insert(name.as_str(), position);
                x += width + GAP;
            }
            canvas_width = canvas_width.max(x.saturating_sub(GAP));
        }

        let mut same_row_relations = vec![Vec::new(); self.rows.len()];
        let mut gap_relations = vec![Vec::new(); self.rows.len().saturating_sub(1)];
        for relation in &self.relations {
            let source = positions[relation.source.as_str()];
            let target = positions[relation.target.as_str()];
            if source.row == target.row {
                same_row_relations[source.row].push(relation);
            } else {
                gap_relations[target.row].push(relation);
            }
        }

        let mut lines = Vec::new();
        for row_index in 0..self.rows.len() {
            let mut band = canvas(canvas_width, 3);
            for (column_index, name) in self.rows[row_index].iter().enumerate() {
                let position = positions[name.as_str()];
                Block::boxed(name.as_str()).draw_on(&mut band, position.x, 0);
                debug_assert_eq!(position.col, column_index);
            }
            for relation in &same_row_relations[row_index] {
                draw_same_row(&mut band, relation, &positions);
            }
            lines.extend(band.into_iter().map(String::from_iter));

            if row_index + 1 < self.rows.len() {
                lines.extend(render_channel(canvas_width, &gap_relations[row_index], &positions));
            }
        }

        let canvas: Vec<Vec<char>> = lines.iter().map(|line| line.chars().collect()).collect();
        let rendered = render_canvas(&canvas);
        if self.layout_name.is_empty() {
            rendered
        } else {
            super::block::frame(&self.layout_name, &rendered)
        }
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    super::validate_name(name, "class ", &[',', '{', '}', '#', '-', '>', '<', '*', '.', '|'])
}

fn parse_relation(line_number: usize, line: &str) -> Result<Relation, String> {
    let operators = [
        ("--|>", Kind::Generalization),
        ("..|>", Kind::Realization),
        ("*--", Kind::Composition),
        ("-->", Kind::Association),
    ];

    for (operator, kind) in operators {
        if let Some((source, target)) = line.split_once(operator) {
            let source = source.trim();
            let target = target.trim();
            if source.is_empty() || target.is_empty() {
                return Err(format!("line {line_number}: empty relation side"));
            }
            validate_name(source)?;
            validate_name(target)?;
            let (source, target) = (source.to_owned(), target.to_owned());
            return Ok(Relation { source, target, kind, line: line_number });
        }
    }

    Err(format!("line {line_number}: expected a relation operator"))
}

fn draw_same_row(band: &mut [Vec<char>], relation: &Relation, coords: &HashMap<&str, Coord>) {
    let source = coords[relation.source.as_str()];
    let target = coords[relation.target.as_str()];
    let (left, right) = if source.col < target.col { (source, target) } else { (target, source) };
    let (start, end) = (left.x + left.width, right.x);
    let (_, horizontal) = relation.kind.line_chars();
    let ltr = source.col < target.col;
    let (from, to, head) = match (relation.kind, ltr) {
        (Kind::Composition, true) => (end - 1, start, "*"),
        (Kind::Composition, false) => (start, end - 1, "*"),
        (Kind::Generalization | Kind::Realization, true) => (start, end - 1, "|>"),
        (Kind::Generalization | Kind::Realization, false) => (end - 1, start, "<|"),
        (Kind::Association, true) => (start, end - 1, ">"),
        (Kind::Association, false) => (end - 1, start, "<"),
    };
    arrow_h(band, 1, from, to, horizontal, head);
}

fn render_channel(width: usize, rels: &[&Relation], coords: &HashMap<&str, Coord>) -> Vec<String> {
    let height = 2 + rels.len().max(1);
    let last = height - 1;
    let mut canvas = canvas(width, height);

    for (index, relation) in rels.iter().enumerate() {
        let source = coords[relation.source.as_str()];
        let target = coords[relation.target.as_str()];
        let source_x = source.x + source.width / 2;
        let target_x = target.x + target.width / 2;
        let lane = index + 1;
        let (vertical, horizontal) = relation.kind.line_chars();

        draw_vline(&mut canvas, target_x, 0, lane, vertical);
        canvas[0][target_x] = match relation.kind {
            Kind::Composition => vertical,
            Kind::Generalization | Kind::Realization => 'A',
            Kind::Association => '^',
        };
        draw_vline(&mut canvas, source_x, lane, last, vertical);
        if source_x != target_x {
            draw_hline(&mut canvas, lane, source_x, target_x, horizontal);
        }
        canvas[last][source_x] = if relation.kind == Kind::Composition { '*' } else { vertical };
    }

    canvas.into_iter().map(String::from_iter).collect()
}

#[cfg(test)]
mod tests {
    use super::run;

    fn render(input: &str) -> String {
        run(input).unwrap()
    }

    fn same_row(arrow: &str) -> String {
        format!("+---+     +---+\n| A |{arrow}| B |\n+---+     +---+")
    }

    fn between_rows(head: char, vertical: char, last: char) -> String {
        format!("+---+\n| A |\n+---+\n  {head}\n  {vertical}\n  {last}\n+---+\n| B |\n+---+")
    }

    #[test]
    fn renders_every_relation_in_one_row() {
        let cases = [
            ("A --|> B", "---|>"),
            ("A ..|> B", "...|>"),
            ("B --|> A", "<|---"),
            ("B ..|> A", "<|..."),
            ("A *-- B", "*----"),
            ("B *-- A", "----*"),
            ("A --> B", "---->"),
            ("B --> A", "<----"),
        ];

        for (relation, arrow) in cases {
            let input = format!("layout {{ A, B }}\n{relation}\n");
            assert_eq!(render(&input), same_row(arrow), "relation: {relation}");
        }
    }

    #[test]
    fn renders_every_relation_between_adjacent_rows() {
        let cases = [
            ("B --|> A", ('A', '|', '|')),
            ("B ..|> A", ('A', ':', ':')),
            ("B *-- A", ('|', '|', '*')),
            ("B --> A", ('^', '|', '|')),
        ];

        for (relation, (head, vertical, last)) in cases {
            let input = format!("layout {{ A\nB }}\n{relation}\n");
            assert_eq!(render(&input), between_rows(head, vertical, last), "relation: {relation}");
        }
    }

    #[test]
    fn allows_trailing_comma_in_layout() {
        let input = "layout { A, B, }\nA --> B\n";
        let expected = "\
+---+     +---+
| A |---->| B |
+---+     +---+";

        assert_eq!(render(input), expected);
    }

    #[test]
    fn semicolon_starts_a_new_layout_row() {
        let input = "layout { A, B; C }\nC --> A\n";
        assert!(render(input).contains("| C |"));
    }

    #[test]
    fn named_layout_draws_a_frame() {
        assert!(render("layout Domain { A }\n").starts_with("+ Domain"));
    }

    #[test]
    fn renders_offset_upward_edges() {
        let layout = "layout {\n  X, A\n  B\n}\n";
        let solid = concat!(
            "+---+     +---+\n",
            "| X |     | A |\n",
            "+---+     +---+\n",
            "            ^\n",
            "  +---------+\n",
            "  |\n",
            "+---+\n",
            "| B |\n",
            "+---+",
        );
        let dotted = concat!(
            "+---+     +---+\n",
            "| X |     | A |\n",
            "+---+     +---+\n",
            "            A\n",
            "  +.........+\n",
            "  :\n",
            "+---+\n",
            "| B |\n",
            "+---+",
        );

        assert_eq!(render(&format!("{layout}B --> A\n")), solid);
        assert_eq!(render(&format!("{layout}B ..|> A\n")), dotted);
    }

    #[test]
    fn ignores_empty_lines_and_comments() {
        let input = "\
# leading comment

layout {
  # inside the layout
  A, B,
}

# between layout and relations
A --> B
";
        let expected = "\
+---+     +---+
| A |---->| B |
+---+     +---+";

        assert_eq!(render(input), expected);
    }

    #[test]
    fn detects_duplicate_same_row_pairs_in_either_direction() {
        // Direction does not distinguish pairs, so reversed relations count as duplicates.
        let error = run("layout { A, B }\nB --> A\nA --|> B\n").unwrap_err();

        assert!(error.contains("multiple relations"), "unexpected error: {error}");
    }

    #[test]
    fn rejects_invalid_input() {
        let cases = [
            ("", "missing layout block"),
            ("layout {\n}\n", "layout block is empty"),
            ("layout A, B\n", "expected '{'"),
            ("layout A-B { A }\n", "invalid class name"),
            ("layout {\nA, B\n", "unclosed layout"),
            ("layout { A, B, A }\n", "duplicate class"),
            ("layout { A, B }\nA --> C\n", "unknown class"),
            ("layout { A, B }\nC --> B\n", "unknown class"),
            ("layout { A }\nA --> A\n", "cannot relate to itself"),
            ("layout { A, B, C }\nA --> C\n", "not adjacent"),
            ("layout { A\nB }\nA --> B\n", "must be in same/previous row"),
            ("layout { A\nB\nC }\nC --> A\n", "must be in same/previous row"),
            ("layout { A, B }\nA --> B\nA --|> B\n", "multiple relations"),
            ("layout { A, B }\nA => B\n", "expected a relation operator"),
            ("layout { A, B }\nA -->\n", "empty relation side"),
            ("layout { A, B }\n--> B\n", "empty relation side"),
            ("layout { A-B }\n", "invalid class name"),
            ("layout { A.B }\n", "invalid class name"),
            ("layout { A|B }\n", "invalid class name"),
            ("layout { A\u{7} }\n", "invalid class name"),
            ("layout { A, B }\nA-B --> B\n", "invalid class name"),
            ("layout { A, B }\nA --> B-C\n", "invalid class name"),
        ];

        for (input, expected_fragment) in cases {
            let error = run(input).unwrap_err();
            assert!(
                error.contains(expected_fragment),
                "expected error containing {expected_fragment:?} for input {input:?}, got {error:?}"
            );
        }
    }
}
