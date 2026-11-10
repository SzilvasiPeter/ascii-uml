use std::collections::{HashMap, HashSet};

const GAP: usize = 5;

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
}

#[derive(Debug, Clone, Copy)]
struct Position {
    row: usize,
    col: usize,
    x: usize,
    width: usize,
}

pub fn run(input: &str) -> Result<String, String> {
    Diagram::parse(input).map(|diagram| diagram.render())
}

impl Diagram {
    fn parse(input: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        let mut relations = Vec::new();
        let mut seen_layout = false;
        let mut in_layout = false;

        for (index, raw_line) in input.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if !seen_layout && let Some(rest) = line.strip_prefix("layout") {
                let rest = rest.trim_start();
                let rest = rest
                    .strip_prefix('{')
                    .ok_or_else(|| format!("line {line_number}: expected '{{' after layout"))?;
                seen_layout = true;
                let (content, closed) = split_layout(rest);
                add_row(content, &mut rows)?;
                in_layout = !closed;
                continue;
            }

            if in_layout {
                let (content, closed) = split_layout(line);
                add_row(content, &mut rows)?;
                if closed {
                    in_layout = false;
                }
                continue;
            }

            relations.push(parse_relation(line_number, line)?);
        }

        if in_layout {
            return Err("unclosed layout block".to_owned());
        }
        if !seen_layout {
            return Err("missing layout block".to_owned());
        }
        rows.retain(|row| !row.is_empty());
        if rows.is_empty() {
            return Err("layout block is empty".to_owned());
        }

        {
            let mut positions = HashMap::new();
            for (row_index, row) in rows.iter().enumerate() {
                for (column_index, name) in row.iter().enumerate() {
                    if positions.insert(name.as_str(), (row_index, column_index)).is_some() {
                        return Err(format!("duplicate class '{name}' in layout"));
                    }
                }
            }

            let mut same_row_pairs = HashSet::new();
            for relation in &relations {
                let source = positions.get(relation.source.as_str()).copied().ok_or_else(|| {
                    format!("line {}: unknown class '{}'", relation.line, relation.source)
                })?;
                let target = positions.get(relation.target.as_str()).copied().ok_or_else(|| {
                    format!("line {}: unknown class '{}'", relation.line, relation.target)
                })?;

                if relation.source == relation.target {
                    return Err(format!("line {}: a class cannot relate to itself", relation.line));
                }

                if source.0 == target.0 {
                    if source.1.abs_diff(target.1) != 1 {
                        return Err(format!(
                            "line {}: target '{}' is not adjacent to '{}' in the same row",
                            relation.line, relation.target, relation.source
                        ));
                    }

                    let pair = if source.1 < target.1 {
                        (relation.source.as_str(), relation.target.as_str())
                    } else {
                        (relation.target.as_str(), relation.source.as_str())
                    };
                    if !same_row_pairs.insert(pair) {
                        return Err(format!(
                            "line {}: multiple relations between '{}' and '{}'",
                            relation.line, relation.source, relation.target
                        ));
                    }
                } else if source.0 != target.0 + 1 {
                    return Err(format!(
                        "line {}: target '{}' must be in the same row or one row above '{}'",
                        relation.line, relation.target, relation.source
                    ));
                }
            }
        }

        Ok(Self { rows, relations })
    }

    fn render(&self) -> String {
        let widths: Vec<Vec<usize>> =
            self.rows.iter().map(|row| row.iter().map(|name| box_width(name)).collect()).collect();
        let row_widths: Vec<usize> = widths
            .iter()
            .map(|row| row.iter().sum::<usize>() + GAP * row.len().saturating_sub(1))
            .collect();
        let canvas_width = row_widths.iter().copied().max().unwrap_or(0);

        let mut positions = HashMap::new();
        for (row_index, row) in self.rows.iter().enumerate() {
            let mut x = 0;
            for (column_index, name) in row.iter().enumerate() {
                let width = widths[row_index][column_index];
                positions.insert(
                    name.as_str(),
                    Position { row: row_index, col: column_index, x, width },
                );
                x += width + GAP;
            }
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
            let mut band = vec![vec![' '; canvas_width]; 3];
            for (column_index, name) in self.rows[row_index].iter().enumerate() {
                let position = positions[name.as_str()];
                draw_box(&mut band, position, name);
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

        lines.iter().map(|line| line.trim_end()).collect::<Vec<_>>().join("\n")
    }
}

fn split_layout(line: &str) -> (&str, bool) {
    line.find('}').map_or((line, false), |position| (&line[..position], true))
}

fn add_row(content: &str, rows: &mut Vec<Vec<String>>) -> Result<(), String> {
    let mut row = Vec::new();
    for part in content.split(',') {
        let name = part.trim();
        if name.is_empty() {
            continue;
        }
        validate_name(name)?;
        row.push(name.to_owned());
    }
    if !row.is_empty() {
        rows.push(row);
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.chars().any(|character| {
        character.is_control()
            || matches!(character, ',' | '{' | '}' | '#' | '-' | '>' | '<' | '*' | '.' | '|')
    }) {
        return Err(format!("invalid class name '{name}'"));
    }
    Ok(())
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
            if source.is_empty() {
                return Err(format!("line {line_number}: relation source must not be empty"));
            }
            if target.is_empty() {
                return Err(format!("line {line_number}: relation target must not be empty"));
            }
            validate_name(source)?;
            validate_name(target)?;
            return Ok(Relation {
                source: source.to_owned(),
                target: target.to_owned(),
                kind,
                line: line_number,
            });
        }
    }

    Err(format!("line {line_number}: expected a relation operator ('--|>', '..|>', '*--', '-->')"))
}

fn box_width(name: &str) -> usize {
    (name.chars().count() + 4) | 1
}

fn draw_box(band: &mut [Vec<char>], position: Position, name: &str) {
    let right = position.x + position.width - 1;
    for row in [0, 2] {
        band[row][position.x] = '+';
        band[row][right] = '+';
        band[row][position.x + 1..right].fill('-');
    }

    band[1][position.x] = '|';
    band[1][right] = '|';
    let inner = position.width - 2;
    let start = position.x + 1 + (inner - name.chars().count()) / 2;
    for (x, character) in (start..).zip(name.chars()) {
        band[1][x] = character;
    }
}

fn draw_same_row(band: &mut [Vec<char>], relation: &Relation, positions: &HashMap<&str, Position>) {
    let source = positions[relation.source.as_str()];
    let target = positions[relation.target.as_str()];
    let (left, right) = if source.col < target.col { (source, target) } else { (target, source) };
    let start = left.x + left.width;
    let end = right.x;
    let (_, horizontal) = relation.kind.line_chars();

    band[1][start..end].fill(horizontal);
    if relation.kind == Kind::Composition {
        if source.col < target.col {
            band[1][start] = '*';
        } else {
            band[1][end - 1] = '*';
        }
    } else if target.col > source.col {
        band[1][end - 1] = '>';
    } else {
        band[1][start] = '<';
    }
}

fn render_channel(
    width: usize,
    relations: &[&Relation],
    positions: &HashMap<&str, Position>,
) -> Vec<String> {
    let height = 2 + relations.len().max(1);
    let last = height - 1;
    let mut canvas = vec![vec![' '; width]; height];

    for (index, relation) in relations.iter().enumerate() {
        let source = positions[relation.source.as_str()];
        let target = positions[relation.target.as_str()];
        let source_x = source.x + source.width / 2;
        let target_x = target.x + target.width / 2;
        let lane = index + 1;
        let (vertical, horizontal) = relation.kind.line_chars();

        canvas[0][target_x] = if relation.kind == Kind::Composition { vertical } else { '^' };
        for row in canvas.iter_mut().take(lane + 1).skip(1) {
            row[target_x] = vertical;
        }
        for row in canvas.iter_mut().take(last).skip(lane) {
            row[source_x] = vertical;
        }
        canvas[last][source_x] = if relation.kind == Kind::Composition { '*' } else { vertical };

        if source_x == target_x {
            canvas[lane][source_x] = vertical;
        } else {
            let (left, right) = (source_x.min(target_x), source_x.max(target_x));
            canvas[lane][source_x] = '+';
            canvas[lane][target_x] = '+';
            canvas[lane][left + 1..right].fill(horizontal);
        }
    }

    canvas.into_iter().map(String::from_iter).collect()
}

#[cfg(test)]
mod tests {
    use super::run;

    fn render(input: &str) -> String {
        run(input).unwrap_or_else(|error| panic!("expected success, got error: {error}"))
    }

    #[test]
    fn renders_every_relation_in_one_row() {
        let cases = [
            (
                "A --|> B",
                "\
+---+     +---+
| A |---->| B |
+---+     +---+",
            ),
            (
                "A ..|> B",
                "\
+---+     +---+
| A |....>| B |
+---+     +---+",
            ),
            (
                "A *-- B",
                "\
+---+     +---+
| A |*----| B |
+---+     +---+",
            ),
            (
                "B *-- A",
                "\
+---+     +---+
| A |----*| B |
+---+     +---+",
            ),
            (
                "A --> B",
                "\
+---+     +---+
| A |---->| B |
+---+     +---+",
            ),
            (
                "B --> A",
                "\
+---+     +---+
| A |<----| B |
+---+     +---+",
            ),
        ];

        for (relation, expected) in cases {
            let input = format!("layout {{ A, B }}\n{relation}\n");
            assert_eq!(render(&input), expected, "relation: {relation}");
        }
    }

    #[test]
    fn renders_every_relation_between_adjacent_rows() {
        let cases = [
            (
                "B --|> A",
                "\
+---+
| A |
+---+
  ^
  |
  |
+---+
| B |
+---+",
            ),
            (
                "B ..|> A",
                "\
+---+
| A |
+---+
  ^
  :
  :
+---+
| B |
+---+",
            ),
            (
                "B *-- A",
                "\
+---+
| A |
+---+
  |
  |
  *
+---+
| B |
+---+",
            ),
            (
                "B --> A",
                "\
+---+
| A |
+---+
  ^
  |
  |
+---+
| B |
+---+",
            ),
        ];

        for (relation, expected) in cases {
            let input = format!("layout {{ A\nB }}\n{relation}\n");
            assert_eq!(render(&input), expected, "relation: {relation}");
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
            "            ^\n",
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
        // The first relation has target 'A' on the left and source 'B' on the
        // right, so the pair is stored as (target, source).
        let error = run("layout { A, B }\nB --> A\nA --|> B\n").unwrap_err();

        assert!(
            error.contains("multiple relations between 'A' and 'B'"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_invalid_input() {
        let cases = [
            ("", "missing layout block"),
            ("layout {\n}\n", "layout block is empty"),
            ("layout A, B\n", "expected '{' after layout"),
            ("layout {\nA, B\n", "unclosed layout block"),
            ("layout { A, B, A }\n", "duplicate class 'A'"),
            ("layout { A, B }\nA --> C\n", "unknown class 'C'"),
            ("layout { A, B }\nC --> B\n", "unknown class 'C'"),
            ("layout { A }\nA --> A\n", "cannot relate to itself"),
            ("layout { A, B, C }\nA --> C\n", "is not adjacent"),
            ("layout { A\nB }\nA --> B\n", "must be in the same row or one row above"),
            ("layout { A\nB\nC }\nC --> A\n", "must be in the same row or one row above"),
            ("layout { A, B }\nA --> B\nA --|> B\n", "multiple relations between 'A' and 'B'"),
            ("layout { A, B }\nA => B\n", "expected a relation operator"),
            ("layout { A, B }\nA -->\n", "relation target must not be empty"),
            ("layout { A, B }\n--> B\n", "relation source must not be empty"),
            ("layout { A-B }\n", "invalid class name 'A-B'"),
            ("layout { A.B }\n", "invalid class name 'A.B'"),
            ("layout { A|B }\n", "invalid class name 'A|B'"),
            ("layout { A\u{7} }\n", "invalid class name"),
            ("layout { A, B }\nA-B --> B\n", "invalid class name 'A-B'"),
            ("layout { A, B }\nA --> B-C\n", "invalid class name 'B-C'"),
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
