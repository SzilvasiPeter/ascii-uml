use std::collections::{HashMap, HashSet};

pub fn run(input: &str) -> Result<String, String> {
    Diagram::parse(input).map(|diagram| diagram.render())
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pos {
    group: usize,
    service: usize,
}

struct Link {
    from: Pos,
    to: Pos,
}

struct Group {
    name: String,
    line: usize,
    services: Vec<(String, usize)>,
}

struct Diagram {
    groups: Vec<Group>,
    links: Vec<Link>,
}

impl Diagram {
    fn parse(input: &str) -> Result<Self, String> {
        let mut groups = Vec::new();
        let mut raw_connections = Vec::new();
        let mut current: Option<Group> = None;

        for (index, raw_line) in input.lines().enumerate() {
            let line = index + 1;
            let text = raw_line.trim();
            if text.is_empty() || text.starts_with('#') {
                continue;
            }
            if let Some(group) = current.as_mut() {
                if text == "}" {
                    let group = current.take().unwrap();
                    if group.services.is_empty() {
                        return Err(format!(
                            "line {line}: empty group '{}'",
                            group.name.escape_debug()
                        ));
                    }
                    groups.push(group);
                } else if is_group_start(text) {
                    return Err(format!("line {line}: nested group"));
                } else if text.contains("-->") {
                    return Err(format!("line {line}: connections must be outside groups"));
                } else {
                    for part in text.split(';') {
                        let name = part.trim();
                        validate_name(name, "service", line)?;
                        group.services.push((name.to_owned(), line));
                    }
                }
            } else if text == "}" {
                return Err(format!("line {line}: unexpected '}}'"));
            } else if is_group_start(text) {
                let rest = text.strip_prefix("group").unwrap().trim();
                let Some(name) = rest.strip_suffix('{') else {
                    return Err(format!("line {line}: expected '{{' after group name"));
                };
                let name = name.trim();
                validate_name(name, "group", line)?;
                current = Some(Group { name: name.to_owned(), line, services: Vec::new() });
            } else {
                raw_connections.push((line, text.to_owned()));
            }
        }

        if let Some(group) = current {
            return Err(format!(
                "line {}: unclosed group '{}'",
                group.line,
                group.name.escape_debug()
            ));
        }
        if groups.is_empty() {
            return Err("no groups".to_owned());
        }

        let mut seen = HashMap::new();
        let mut positions = HashMap::new();
        for (group_index, group) in groups.iter().enumerate() {
            insert_name(&mut seen, &group.name, group.line)?;
            for (service_index, (name, line)) in group.services.iter().enumerate() {
                insert_name(&mut seen, name, *line)?;
                positions.insert(name.clone(), (group_index, service_index));
            }
        }

        let mut links = Vec::new();
        let mut pairs = HashSet::new();
        for (line, text) in raw_connections {
            let (from, to) = parse_connection(&text, line)?;
            let from_pos = resolve(&positions, &from, line)?;
            let to_pos = resolve(&positions, &to, line)?;
            if from_pos == to_pos {
                return Err(format!("line {line}: cannot connect to itself"));
            }
            let pair =
                if from <= to { (from.clone(), to.clone()) } else { (to.clone(), from.clone()) };
            if !pairs.insert(pair.clone()) {
                return Err(format!(
                    "line {line}: duplicate connection between '{}' and '{}'",
                    pair.0.escape_debug(),
                    pair.1.escape_debug()
                ));
            }
            validate_adjacency(&groups, from_pos, to_pos, &from, &to, line)?;
            links.push(Link { from: from_pos, to: to_pos });
        }

        Ok(Self { groups, links })
    }

    fn render(&self) -> String {
        let group_count = self.groups.len();
        let column_count = self.groups.iter().map(|group| group.services.len()).max().unwrap_or(0);
        let mut widths = vec![0; column_count];
        for group in &self.groups {
            for (index, (name, _)) in group.services.iter().enumerate() {
                widths[index] = widths[index].max(service_width(name));
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
        for index in 0..group_count.saturating_sub(1) {
            tops[index + 1] = tops[index] + 7 + 1 + gap_counts[index];
        }
        let height = tops.last().unwrap() + 7;
        let mut grid = vec![vec![' '; width]; height];

        draw_borders(&mut grid, &tops);
        draw_boxes(&mut grid, &self.groups, &columns, &widths, &tops);
        draw_same_group(&mut grid, &self.groups, &self.links, &columns, &widths, &tops);
        draw_cross_group(&mut grid, &self.links, &centers, &tops, group_count);
        draw_titles(&mut grid, &self.groups, &tops);

        grid.into_iter()
            .map(|row| row.into_iter().collect::<String>().trim_end().to_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn validate_name(name: &str, kind: &str, line: usize) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!("line {line}: empty {kind} name"));
    }
    if name.chars().any(|character| {
        character.is_control() || matches!(character, '{' | '}' | ',' | ';' | '#' | '|' | '<' | '>')
    }) {
        return Err(format!("line {line}: invalid {kind} name '{}'", name.escape_debug()));
    }
    Ok(())
}

fn is_group_start(line: &str) -> bool {
    line.strip_prefix("group").is_some_and(|rest| {
        rest.is_empty() || rest.starts_with('{') || rest.starts_with(char::is_whitespace)
    })
}

fn insert_name(seen: &mut HashMap<String, usize>, name: &str, line: usize) -> Result<(), String> {
    if seen.insert(name.to_owned(), line).is_some() {
        return Err(format!("line {line}: duplicate name '{}'", name.escape_debug()));
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
    positions: &HashMap<String, (usize, usize)>,
    name: &str,
    line: usize,
) -> Result<Pos, String> {
    positions.get(name).map_or_else(
        || Err(format!("line {line}: unknown service '{}'", name.escape_debug())),
        |&(group, service)| Ok(Pos { group, service }),
    )
}

fn validate_adjacency(
    groups: &[Group],
    from: Pos,
    to: Pos,
    from_name: &str,
    to_name: &str,
    line: usize,
) -> Result<(), String> {
    if from.group == to.group {
        if from.service.abs_diff(to.service) != 1 {
            return Err(format!(
                "line {line}: services '{}' and '{}' must be adjacent within group '{}'",
                from_name.escape_debug(),
                to_name.escape_debug(),
                groups[from.group].name.escape_debug()
            ));
        }
    } else if from.group.abs_diff(to.group) != 1 {
        return Err(format!(
            "line {line}: groups '{}' and '{}' are not adjacent",
            groups[from.group].name.escape_debug(),
            groups[to.group].name.escape_debug()
        ));
    }
    Ok(())
}

fn service_width(name: &str) -> usize {
    (name.chars().count() + 4) | 1
}

fn geometry(name: &str, service: usize, columns: &[usize], widths: &[usize]) -> (usize, usize) {
    let width = service_width(name);
    (columns[service] + (widths[service] - width) / 2, width)
}

fn put(grid: &mut [Vec<char>], x: usize, y: usize, character: char) {
    grid[y][x] = match (grid[y][x], character) {
        (' ', character) => character,
        ('+', _) | (_, '+') | ('-', '|') | ('|', '-') => '+',
        (existing, character) if existing == character => existing,
        (_, character) => character,
    };
}

fn draw_borders(grid: &mut [Vec<char>], tops: &[usize]) {
    let width = grid[0].len();
    for &top in tops {
        for row in &mut grid[top..top + 7] {
            row[0] = '|';
            row[width - 1] = '|';
        }
        grid[top].fill('-');
        grid[top + 6].fill('-');
        for (x, y) in [(0, top), (width - 1, top), (0, top + 6), (width - 1, top + 6)] {
            grid[y][x] = '+';
        }
    }
}

fn draw_boxes(
    grid: &mut [Vec<char>],
    groups: &[Group],
    columns: &[usize],
    widths: &[usize],
    tops: &[usize],
) {
    for (group_index, group) in groups.iter().enumerate() {
        for (service_index, (name, _)) in group.services.iter().enumerate() {
            let (x, width) = geometry(name, service_index, columns, widths);
            let y = tops[group_index] + 2;
            for row in [y, y + 2] {
                grid[row][x] = '+';
                grid[row][x + width - 1] = '+';
                grid[row][x + 1..x + width - 1].fill('-');
            }
            grid[y + 1][x] = '|';
            grid[y + 1][x + width - 1] = '|';
            grid[y + 1][x + 1] = ' ';
            for (offset, character) in name.chars().enumerate() {
                grid[y + 1][x + 2 + offset] = character;
            }
            grid[y + 1][x + 2 + name.chars().count()] = ' ';
        }
    }
}

fn draw_same_group(
    grid: &mut [Vec<char>],
    groups: &[Group],
    links: &[Link],
    columns: &[usize],
    widths: &[usize],
    tops: &[usize],
) {
    for link in links {
        if link.from.group != link.to.group {
            continue;
        }
        let group = &groups[link.from.group];
        let (source_x, source_width) =
            geometry(&group.services[link.from.service].0, link.from.service, columns, widths);
        let (target_x, target_width) =
            geometry(&group.services[link.to.service].0, link.to.service, columns, widths);
        let y = tops[link.from.group] + 3;
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
    }
}

fn draw_cross_group(
    grid: &mut [Vec<char>],
    links: &[Link],
    centers: &[usize],
    tops: &[usize],
    group_count: usize,
) {
    let mut lanes = vec![0; group_count.saturating_sub(1)];
    for link in links {
        if link.from.group == link.to.group {
            continue;
        }
        let (source_group, source_service) = (link.from.group, link.from.service);
        let (target_group, target_service) = (link.to.group, link.to.service);
        let down = target_group > source_group;
        let source_x = centers[source_service];
        let target_x = centers[target_service];
        let (source_y, target_y) = if down {
            (tops[source_group] + 5, tops[target_group] + 1)
        } else {
            (tops[source_group] + 1, tops[target_group] + 5)
        };
        if source_service == target_service {
            for y in source_y.min(target_y)..=source_y.max(target_y) {
                put(grid, source_x, y, '|');
            }
        } else {
            let gap = source_group.min(target_group);
            let lane = tops[gap] + 7 + lanes[gap];
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

fn draw_titles(grid: &mut [Vec<char>], groups: &[Group], tops: &[usize]) {
    for (group, &top) in groups.iter().zip(tops) {
        grid[top][1] = ' ';
        for (offset, character) in group.name.chars().enumerate() {
            grid[top][offset + 2] = character;
        }
        grid[top][group.name.chars().count() + 2] = ' ';
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn error_cases() {
        let cases = [
            ("", "no groups"),
            ("A", "no groups"),
            ("A\nB", "no groups"),
            ("A --> B", "no groups"),
            ("group G\nA\n}", "line 1: expected '{'"),
            ("group  {\nA\n}", "line 1: empty group name"),
            ("group G {\nA", "unclosed group"),
            ("group G {\n}", "empty group"),
            ("group G {\ngroup H {\nA\n}\n}", "line 2: nested group"),
            ("group G {\nA; B\nA --> B\n}", "line 3: connections"),
            ("}", "line 1: unexpected '}'"),
            ("group G {\nA;;B\n}", "line 2: empty service"),
            ("group G {\nA;\n}", "line 2: empty service"),
            ("group G {\n;A\n}", "line 2: empty service"),
            ("group G {\nA; A\n}", "duplicate name 'A'"),
            ("group A {\nB\n}\ngroup G {\nA\n}", "duplicate name 'A'"),
            ("group G {\nA\n}\ngroup G {\nB\n}", "duplicate name 'G'"),
            ("group G {\nA\u{7}\n}", "line 2: invalid service name 'A\\u{7}'"),
            ("group G {\nA; B\n}\nA B", "line 4: expected exactly one '-->'"),
            ("group G {\nA; B\n}\nA --> B --> C", "line 4: expected exactly one '-->'"),
            ("group G {\nA; B\n}\nA -->", "line 4: empty connection side"),
            ("group G {\nA; B\n}\n--> B", "line 4: empty connection side"),
            ("group G {\nA; B\n}\nA --> C", "line 4: unknown service 'C'"),
            ("group G {\nA; B\n}\nC --> B", "line 4: unknown service 'C'"),
            ("group G {\nA; B\n}\nA --> A", "line 4: cannot connect to itself"),
            ("group G {\nA; B\n}\nA --> B\nA --> B", "line 5: duplicate connection"),
            ("group G {\nA; B\n}\nA --> B\nB --> A", "line 5: duplicate connection"),
            (
                "group G {\nA; B; C\n}\nA --> C",
                "line 4: services 'A' and 'C' must be adjacent within group 'G'",
            ),
            (
                "group G1 {\nA\n}\ngroup G2 {\nB\n}\ngroup G3 {\nC\n}\nA --> C",
                "line 10: groups 'G1' and 'G3' are not adjacent",
            ),
        ];
        for (input, expected) in cases {
            let error = run(input).unwrap_err();
            assert!(error.contains(expected), "input: {input:?}, error: {error:?}");
        }

        for name in ["A{B", "A}B", "A#B", "A|B", "A<B", "A>B"] {
            let input = format!("group G {{\n{name}\n}}");
            let error = run(&input).unwrap_err();
            assert!(error.contains("invalid service name"), "input: {input:?}, error: {error:?}");
        }
        for name in ["G,H", "G;H", "G#H", "G|H", "G<H", "G>H"] {
            let input = format!("group {name} {{\nA\n}}");
            let error = run(&input).unwrap_err();
            assert!(error.contains("invalid group name"), "input: {input:?}, error: {error:?}");
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
                "group G {\nA; B\n}",
                r"+ G --------------+
|                 |
| +---+     +---+ |
| | A |     | B | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "group G {\nA; B\n}\nA --> B",
                r"+ G --------------+
|                 |
| +---+     +---+ |
| | A |---->| B | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "# comment\n\ngroup G {\n# ignored\nA; B\n}\n\nA --> B\n",
                r"+ G --------------+
|                 |
| +---+     +---+ |
| | A |---->| B | |
| +---+     +---+ |
|                 |
+-----------------+",
            ),
            (
                "group G {\nA; B\n}\nB --> A",
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
                "group G1 {\nA; B\n}\ngroup G2 {\nC; D\n}",
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
                "group G1 {\nA; B\n}\ngroup G2 {\nC; D\n}\nA --> B\nB --> C",
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
                "group G1 {\nA; B; C\n}\ngroup G2 {\nD; E\n}\nA --> E\nC --> E",
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
