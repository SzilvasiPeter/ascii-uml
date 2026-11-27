use std::collections::{HashMap, HashSet};

use super::block::{Block, draw_frame, draw_text, put, render_canvas};
use super::parse_document;

pub fn run(input: &str) -> Result<String, String> {
    Diagram::parse(input).map(|diagram| diagram.render())
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pos {
    group: usize,
    row: usize,
    service: usize,
}

struct Link {
    from: Pos,
    to: Pos,
}

type Group = super::Block;

struct Diagram {
    groups: Vec<Group>,
    links: Vec<Link>,
}

struct RenderLayout<'a> {
    groups: &'a [Group],
    links: &'a [Link],
    columns: &'a [usize],
    widths: &'a [usize],
    centers: &'a [usize],
    tops: &'a [usize],
    heights: &'a [usize],
}

impl RenderLayout<'_> {
    fn geometry(&self, name: &str, service: usize) -> (usize, usize) {
        let width = Block::boxed(name).width();
        (self.columns[service] + (self.widths[service] - width) / 2, width)
    }
}

impl Diagram {
    fn parse(input: &str) -> Result<Self, String> {
        let document = parse_document(input, "group")?;
        let mut groups = Vec::new();
        for block in document.blocks {
            if !block.name.is_empty() {
                validate_name(&block.name, "group", block.line)?;
            }
            if block.rows.is_empty() {
                return Err(format!("line {}: empty group", block.line));
            }
            for row in &block.rows {
                for (service, line) in row {
                    if service.contains("-->") {
                        return Err(format!("line {line}: connections must be outside groups"));
                    }
                    validate_name(service, "service", *line)?;
                }
            }
            groups.push(block);
        }
        if groups.is_empty() {
            return Err("no groups".to_owned());
        }

        let mut seen = HashMap::new();
        let mut positions = HashMap::new();
        for (group_index, group) in groups.iter().enumerate() {
            if !group.name.is_empty() {
                insert_name(&mut seen, &group.name, group.line)?;
            }
            for (row_index, row) in group.rows.iter().enumerate() {
                for (service_index, (name, line)) in row.iter().enumerate() {
                    insert_name(&mut seen, name, *line)?;
                    positions.insert(name.clone(), (group_index, row_index, service_index));
                }
            }
        }

        let mut links = Vec::new();
        let mut pairs = HashSet::new();
        for (line, text) in document.statements {
            let (from, to) = parse_connection(&text, line)?;
            let from_pos = resolve(&positions, &from, line)?;
            let to_pos = resolve(&positions, &to, line)?;
            if from_pos == to_pos {
                return Err(format!("line {line}: cannot connect to itself"));
            }
            let pair = if from <= to { (from, to) } else { (to, from) };
            if !pairs.insert(pair.clone()) {
                return Err(format!(
                    "line {line}: duplicate connection between '{}' and '{}'",
                    pair.0.escape_debug(),
                    pair.1.escape_debug()
                ));
            }
            validate_adjacency(from_pos, to_pos, line)?;
            links.push(Link { from: from_pos, to: to_pos });
        }

        Ok(Self { groups, links })
    }

    fn render(&self) -> String {
        let group_count = self.groups.len();
        let column_count =
            self.groups.iter().flat_map(|group| &group.rows).map(Vec::len).max().unwrap_or(0);
        let mut widths = vec![0; column_count];
        for group in &self.groups {
            for row in &group.rows {
                for (index, (name, _)) in row.iter().enumerate() {
                    widths[index] = widths[index].max(Block::boxed(name.as_str()).width());
                }
            }
        }
        let mut columns = vec![2];
        for index in 1..column_count {
            columns.push(columns[index - 1] + widths[index - 1] + 5);
        }
        let centers: Vec<usize> = columns.iter().zip(&widths).map(|(&x, &w)| x + w / 2).collect();
        let content_width = widths.iter().sum::<usize>() + 5 * column_count.saturating_sub(1);
        let name_width =
            self.groups.iter().map(|group| group.name.chars().count()).max().unwrap_or(0);
        let width = (content_width + 4).max(name_width + 5);

        let mut gap_counts = vec![0; group_count.saturating_sub(1)];
        for link in &self.links {
            if link.from.group != link.to.group && link.from.service != link.to.service {
                gap_counts[link.from.group.min(link.to.group)] += 1;
            }
        }
        let mut tops = vec![0; group_count];
        let heights: Vec<_> = self.groups.iter().map(|group| 4 * group.rows.len() + 3).collect();
        for index in 0..group_count.saturating_sub(1) {
            tops[index + 1] = tops[index] + heights[index] + 1 + gap_counts[index];
        }
        let height = tops.last().unwrap() + heights[group_count - 1];
        let mut grid = vec![vec![' '; width]; height];
        let layout = RenderLayout {
            groups: &self.groups,
            links: &self.links,
            columns: &columns,
            widths: &widths,
            centers: &centers,
            tops: &tops,
            heights: &heights,
        };
        draw_borders(&mut grid, &layout);
        draw_boxes(&mut grid, &layout);
        draw_same_group(&mut grid, &layout);
        draw_cross_group(&mut grid, &layout);
        draw_titles(&mut grid, &layout);
        render_canvas(&grid)
    }
}

fn validate_name(name: &str, kind: &str, line: usize) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!("line {line}: empty {kind} name"));
    }
    super::validate_name(name, &format!("{kind} "), &['{', '}', ';', ',', '#', '|', '<', '>'])
        .map_err(|e| format!("line {line}: {e}"))
}

fn insert_name(seen: &mut HashMap<String, usize>, name: &str, line: usize) -> Result<(), String> {
    if seen.insert(name.to_owned(), line).is_some() {
        return Err(format!("line {line}: duplicate name"));
    }
    Ok(())
}

fn parse_connection(text: &str, line: usize) -> Result<(String, String), String> {
    if text.matches("-->").count() != 1 {
        return Err(format!("line {line}: expected exactly one '-->'"));
    }
    let (from, to) = text.split_once("-->").unwrap();
    let (from, to) = (from.trim(), to.trim());
    if from.is_empty() || to.is_empty() {
        return Err(format!("line {line}: empty connection side"));
    }
    Ok((from.to_owned(), to.to_owned()))
}

fn resolve(
    positions: &HashMap<String, (usize, usize, usize)>,
    name: &str,
    line: usize,
) -> Result<Pos, String> {
    positions
        .get(name)
        .map(|&(group, row, service)| Pos { group, row, service })
        .ok_or_else(|| format!("line {line}: unknown service"))
}

fn validate_adjacency(from: Pos, to: Pos, line: usize) -> Result<(), String> {
    if from.group == to.group {
        if from.row.abs_diff(to.row) + from.service.abs_diff(to.service) != 1 {
            return Err(format!("line {line}: not adjacent in group"));
        }
    } else if from.group.abs_diff(to.group) != 1 {
        return Err(format!("line {line}: groups not adjacent"));
    }
    Ok(())
}

fn draw_borders(grid: &mut [Vec<char>], layout: &RenderLayout<'_>) {
    let right = grid[0].len() - 1;
    for ((group, &top), &height) in layout.groups.iter().zip(layout.tops).zip(layout.heights) {
        draw_frame(grid, (0, top, right, top + height - 1), &group.name);
    }
}

fn draw_boxes(grid: &mut [Vec<char>], layout: &RenderLayout<'_>) {
    for (group_index, group) in layout.groups.iter().enumerate() {
        for (row_index, row) in group.rows.iter().enumerate() {
            for (service_index, (name, _)) in row.iter().enumerate() {
                let (x, _) = layout.geometry(name, service_index);
                Block::boxed(name.as_str()).draw_on(
                    grid,
                    x,
                    layout.tops[group_index] + 2 + 4 * row_index,
                );
            }
        }
    }
}

fn draw_titles(grid: &mut [Vec<char>], layout: &RenderLayout<'_>) {
    for (group, &top) in layout.groups.iter().zip(layout.tops) {
        draw_text(grid, top, 1, &format!(" {} ", group.name));
    }
}

fn draw_same_group(grid: &mut [Vec<char>], layout: &RenderLayout<'_>) {
    for link in layout.links {
        if link.from.group != link.to.group {
            continue;
        }
        let group = &layout.groups[link.from.group];
        let (source_x, source_width) =
            layout.geometry(&group.rows[link.from.row][link.from.service].0, link.from.service);
        let (target_x, target_width) =
            layout.geometry(&group.rows[link.to.row][link.to.service].0, link.to.service);
        if link.from.row == link.to.row {
            let y = layout.tops[link.from.group] + 3 + 4 * link.from.row;
            if link.from.service < link.to.service {
                let left = source_x + source_width;
                let right = target_x - 1;
                grid[y][left..right].fill('-');
                grid[y][right] = '>';
            } else {
                let left = target_x + target_width;
                let right = source_x - 1;
                grid[y][left] = '<';
                grid[y][left + 1..=right].fill('-');
            }
        } else {
            let x = source_x + source_width / 2;
            let source_y = layout.tops[link.from.group] + 4 + 4 * link.from.row;
            let target_y = layout.tops[link.to.group] + 2 + 4 * link.to.row;
            for y in source_y.min(target_y)..=source_y.max(target_y) {
                put(grid, x, y, '|');
            }
            grid[target_y][x] = if link.to.row > link.from.row { 'v' } else { '^' };
        }
    }
}

fn draw_cross_group(grid: &mut [Vec<char>], layout: &RenderLayout<'_>) {
    let mut lanes = vec![0; layout.groups.len().saturating_sub(1)];
    for link in layout.links {
        if link.from.group == link.to.group {
            continue;
        }
        let (source_group, source_service) = (link.from.group, link.from.service);
        let (target_group, target_service) = (link.to.group, link.to.service);
        let down = target_group > source_group;
        let source_x = layout.centers[source_service];
        let target_x = layout.centers[target_service];
        let (source_y, target_y) = if down {
            (
                layout.tops[source_group] + 5 + 4 * link.from.row,
                layout.tops[target_group] + 1 + 4 * link.to.row,
            )
        } else {
            (
                layout.tops[source_group] + 1 + 4 * link.from.row,
                layout.tops[target_group] + 5 + 4 * link.to.row,
            )
        };
        if source_service == target_service {
            for y in source_y.min(target_y)..=source_y.max(target_y) {
                put(grid, source_x, y, '|');
            }
        } else {
            let gap = source_group.min(target_group);
            let lane = layout.tops[gap] + layout.heights[gap] + lanes[gap];
            lanes[gap] += 1;
            for y in source_y.min(lane)..=source_y.max(lane) {
                put(grid, source_x, y, '|');
            }
            for x in source_x.min(target_x)..=source_x.max(target_x) {
                put(grid, x, lane, '-');
            }
            put(grid, source_x, lane, '+');
            put(grid, target_x, lane, '+');
            for y in lane.min(target_y)..=lane.max(target_y) {
                put(grid, target_x, y, '|');
            }
        }
        grid[target_y][target_x] = if down { 'v' } else { '^' };
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn single_line_group() {
        let output = run("group { A; B; C; }").unwrap();
        for service in ["A", "B", "C"] {
            assert!(output.contains(&format!("| {service} |")));
        }
    }

    #[test]
    fn semicolon_starts_a_service_row() {
        let output = run("group G { A, B; C }").unwrap();
        assert!(output.find("| A |").unwrap() < output.find("| C |").unwrap());
        assert!(output.lines().any(|line| line.contains("| C |")));
    }

    #[test]
    fn error_cases() {
        let cases = [
            ("", "no groups"),
            ("A", "no groups"),
            ("A\nB", "no groups"),
            ("A --> B", "no groups"),
            ("group G\nA\n}", "line 1: expected '{'"),
            ("group G {\nA", "unclosed group"),
            ("group G {\n}", "empty group"),
            ("group G {\ngroup H {\nA\n}\n}", "line 2: nested group"),
            ("group G {\nA; B\nA --> B\n}", "line 3: connections"),
            ("}", "line 1: unexpected '}'"),
            ("group G {\nA; A\n}", "duplicate name"),
            ("group A {\nB\n}\ngroup G {\nA\n}", "duplicate name"),
            ("group G {\nA\n}\ngroup G {\nB\n}", "duplicate name"),
            ("group G {\nA\u{7}\n}", "invalid service name"),
            ("group G {\nA; B\n}\nA B", "line 4: expected exactly one '-->'"),
            ("group G {\nA; B\n}\nA --> B --> C", "line 4: expected exactly one '-->'"),
            ("group G {\nA; B\n}\nA -->", "line 4: empty connection side"),
            ("group G {\nA; B\n}\n--> B", "line 4: empty connection side"),
            ("group G {\nA; B\n}\nA --> C", "unknown service"),
            ("group G {\nA; B\n}\nC --> B", "unknown service"),
            ("group G {\nA; B\n}\nA --> A", "line 4: cannot connect to itself"),
            ("group G {\nA; B\n}\nA --> B\nA --> B", "duplicate connection"),
            ("group G {\nA; B\n}\nA --> B\nB --> A", "duplicate connection"),
            ("group G {\nA, B, C\n}\nA --> C", "not adjacent in group"),
            (
                "group G1 {\nA\n}\ngroup G2 {\nB\n}\ngroup G3 {\nC\n}\nA --> C",
                "groups not adjacent",
            ),
        ];
        for (input, expected) in cases {
            let error = run(input).unwrap_err();
            assert!(error.contains(expected), "input: {input:?}, error: {error:?}");
        }

        for name in ["A{B", "A}B", "A#B", "A|B", "A<B", "A>B"] {
            assert!(
                run(&format!("group G {{\n{name}\n}}"))
                    .unwrap_err()
                    .contains("invalid service name")
            );
        }
        for name in ["G,H", "G;H", "G#H", "G|H", "G<H", "G>H"] {
            assert!(
                run(&format!("group {name} {{\nA\n}}")).unwrap_err().contains("invalid group name")
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn positive_cases() {
        let cases = [
            (
                "group G {\nA\n}",
                r"+ G ----+
|       |
| +---+ |
| | A | |
| +---+ |
|       |
+-------+",
            ),
            (
                "group G {\nA, B\n}",
                r"+ G --------------+
|                 |
| +---+     +---+ |
| | A |     | B | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "group G {\nA, B\n}\nA --> B",
                r"+ G --------------+
|                 |
| +---+     +---+ |
| | A |---->| B | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "# comment\n\ngroup G {\n# ignored\nA, B\n}\n\nA --> B\n",
                r"+ G --------------+
|                 |
| +---+     +---+ |
| | A |---->| B | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "group G {\nA, B\n}\nB --> A",
                r"+ G --------------+
|                 |
| +---+     +---+ |
| | A |<----| B | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "group G1 {\nA\n}\ngroup G2 {\nB\n}",
                r"+ G1 ---+
|       |
| +---+ |
| | A | |
| +---+ |
|       |
+-------+

+ G2 ---+
|       |
| +---+ |
| | B | |
| +---+ |
|       |
+-------+",
            ),
            (
                "group G1 {\nA\n}\ngroup G2 {\nB\n}\nA --> B",
                r"+ G1 ---+
|       |
| +---+ |
| | A | |
| +---+ |
|   |   |
+---+---+
    |
+ G2 ---+
|   v   |
| +---+ |
| | B | |
| +---+ |
|       |
+-------+",
            ),
            (
                "group G1 {\nA\n}\ngroup G2 {\nB\n}\nB --> A",
                r"+ G1 ---+
|       |
| +---+ |
| | A | |
| +---+ |
|   ^   |
+---+---+
    |
+ G2 ---+
|   |   |
| +---+ |
| | B | |
| +---+ |
|       |
+-------+",
            ),
            (
                "group G1 {\nA, B\n}\ngroup G2 {\nC, D\n}",
                r"+ G1 -------------+
|                 |
| +---+     +---+ |
| | A |     | B | |
| +---+     +---+ |
|                 |
+-----------------+

+ G2 -------------+
|                 |
| +---+     +---+ |
| | C |     | D | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "group G1 {\nA, B\n}\ngroup G2 {\nC, D\n}\nA --> B\nB --> C",
                r"+ G1 -------------+
|                 |
| +---+     +---+ |
| | A |---->| B | |
| +---+     +---+ |
|             |   |
+-------------+---+
    +---------+
    |
+ G2 -------------+
|   v             |
| +---+     +---+ |
| | C |     | D | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "group G1 {\nA, B, C\n}\ngroup G2 {\nD, E\n}\nA --> E\nC --> E",
                r"+ G1 -----------------------+
|                           |
| +---+     +---+     +---+ |
| | A |     | B |     | C | |
| +---+     +---+     +---+ |
|   |                   |   |
+---+-------------------+---+
    +---------+         |
              +---------+
              |
+ G2 ---------+-------------+
|             v             |
| +---+     +---+           |
| | D |     | E |           |
| +---+     +---+           |
|                           |
+---------------------------+",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(run(input).unwrap(), expected, "input: {input}");
        }
    }
}
