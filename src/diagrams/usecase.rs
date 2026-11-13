use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

struct Actor {
    name: String,
    side: Side,
}

impl Actor {
    fn width(&self) -> usize {
        self.name.chars().count().max(3)
    }
}

struct Association {
    actor: String,
    use_case: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DependencyKind {
    Include,
    Extend,
}

impl DependencyKind {
    const fn label(self) -> &'static str {
        if matches!(self, Self::Include) { "include" } else { "extend" }
    }
}

struct Dependency {
    source: usize,
    target: usize,
    kind: DependencyKind,
}

struct Diagram {
    system: String,
    use_cases: Vec<String>,
    actors: Vec<Actor>,
    associations: Vec<Association>,
    dependencies: Vec<Dependency>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RelationKind {
    Association,
    Include,
    Extend,
}

struct PendingRelation {
    source: String,
    target: String,
    kind: RelationKind,
    line: usize,
}

impl Diagram {
    fn parse(input: &str) -> Result<Self, String> {
        let (mut system, mut system_line, mut open) = (None, 0, false);
        let (mut use_cases, mut actors, mut relations, mut names) =
            (vec![], vec![], vec![], HashSet::new());
        for (index, raw) in input.lines().enumerate() {
            let (line_number, line) = (index + 1, raw.trim());
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if open {
                if line == "}" {
                    open = false;
                    continue;
                }
                check_name(line_number, line)?;
                if !names.insert(line.to_owned()) {
                    return Err(format!("line {line_number}: duplicate name '{line}'"));
                }
                use_cases.push(line.to_owned());
                continue;
            }
            if line.contains("..>") || line.contains("-->") {
                relations.push(parse_relation(line_number, line)?);
                continue;
            }
            if starts_keyword(line, "system", &['{']) {
                if system.is_some() {
                    return Err(format!("line {line_number}: duplicate system block"));
                }
                let (name, still_open) = parse_system_header(line_number, line)?;
                (system, system_line, open) = (Some(name), line_number, still_open);
                continue;
            }
            if starts_keyword(line, "actor", &[':', '(']) {
                let (name, side) = parse_actor(line_number, line)?;
                if !names.insert(name.clone()) {
                    return Err(format!("line {line_number}: duplicate name '{name}'"));
                }
                actors.push(Actor { name, side });
                continue;
            }
            return Err(format!(
                "line {line_number}: expected a system block, actor declaration, association ('-->'), or dependency ('..>')"
            ));
        }

        if open {
            return Err(format!("line {system_line}: unclosed system block"));
        }
        let system = system.ok_or_else(|| "missing system block".to_owned())?;
        if use_cases.is_empty() {
            return Err(format!("line {system_line}: empty system block has no use cases"));
        }
        let (associations, dependencies) = collect_relations(&use_cases, &actors, &relations)?;
        Ok(Self { system, use_cases, actors, associations, dependencies })
    }
}

fn starts_keyword(line: &str, keyword: &str, extra: &[char]) -> bool {
    line.strip_prefix(keyword).is_some_and(|rest| {
        rest.is_empty()
            || rest.starts_with(char::is_whitespace)
            || rest.starts_with(|character| extra.contains(&character))
    })
}

fn validate_name(name: &str) -> Result<(), String> {
    super::validate_name(name, "", &['{', '}', '#', ',', ':', '|', '<', '>', '(', ')'])
}

fn check_name(line: usize, name: &str) -> Result<(), String> {
    validate_name(name).map_err(|message| format!("line {line}: {message}"))
}

fn check_required(line: usize, label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() { Err(format!("line {line}: {label} must not be empty")) } else { Ok(()) }
}

fn parse_system_header(line_number: usize, line: &str) -> Result<(String, bool), String> {
    let rest = line.strip_prefix("system").map_or("", str::trim_start);
    let brace = rest
        .find('{')
        .ok_or_else(|| format!("line {line_number}: expected '{{' after the system name"))?;
    let name = rest[..brace].trim();
    if name.is_empty() {
        return Err(format!("line {line_number}: system name must not be empty"));
    }
    check_name(line_number, name)?;
    let after = rest[brace + 1..].trim();
    if !after.is_empty() && after != "}" {
        return Err(format!("line {line_number}: system contents must start on the next line"));
    }
    Ok((name.to_owned(), after != "}"))
}

fn parse_actor(line_number: usize, line: &str) -> Result<(String, Side), String> {
    let rest = line.strip_prefix("actor").map_or("", str::trim_start);
    let malformed = || format!("line {line_number}: malformed actor line");
    let (side, name) = if let Some(after) = rest.strip_prefix('(') {
        let close = after.find(')').ok_or_else(malformed)?;
        let side = match &after[..close] {
            "left" => Side::Left,
            "right" => Side::Right,
            _ => return Err(malformed()),
        };
        let after = &after[close + 1..];
        if !after.starts_with(char::is_whitespace) {
            return Err(malformed());
        }
        (side, after.trim_start())
    } else if rest.starts_with(':') {
        return Err(malformed());
    } else {
        (Side::Left, rest)
    };
    let name = name.trim();
    if name.is_empty() {
        return Err(format!("line {line_number}: actor name must not be empty"));
    }
    check_name(line_number, name)?;
    Ok((name.to_owned(), side))
}

fn parse_relation(ln_num: usize, line: &str) -> Result<PendingRelation, String> {
    if let Some((source, rest)) = line.split_once("..>") {
        let (target, label) = rest
            .split_once(':')
            .ok_or_else(|| format!("line {ln_num}: missing dependency label"))?;
        let (source, target, label) = (source.trim(), target.trim(), label.trim());
        check_required(ln_num, "dependency source", source)?;
        check_required(ln_num, "dependency target", target)?;
        check_name(ln_num, source)?;
        check_name(ln_num, target)?;
        let kind = match label {
            "include" => RelationKind::Include,
            "extend" => RelationKind::Extend,
            "" => return Err(format!("line {ln_num}: missing dependency label")),
            _ => return Err(format!("line {ln_num}: invalid dependency label '{label}'")),
        };
        let (source, target) = (source.to_owned(), target.to_owned());
        return Ok(PendingRelation { source, target, kind, line: ln_num });
    }

    let (source, target) = line
        .split_once("-->")
        .ok_or_else(|| format!("line {ln_num}: expected an association ('-->')"))?;
    let (source, target) = (source.trim(), target.trim());
    check_required(ln_num, "association source", source)?;
    check_required(ln_num, "association target", target)?;
    check_name(ln_num, source)?;
    check_name(ln_num, target)?;
    let (source, target) = (source.to_owned(), target.to_owned());
    Ok(PendingRelation { source, target, kind: RelationKind::Association, line: ln_num })
}

#[derive(Clone, Copy)]
enum Endpoint {
    Actor(usize),
    UseCase(usize),
}

fn collect_relations(
    use_cases: &[String],
    actors: &[Actor],
    relations: &[PendingRelation],
) -> Result<(Vec<Association>, Vec<Dependency>), String> {
    let endpoint = |name: &str| {
        use_cases
            .iter()
            .position(|use_case| use_case == name)
            .map(Endpoint::UseCase)
            .or_else(|| actors.iter().position(|actor| actor.name == name).map(Endpoint::Actor))
    };
    let (mut associations, mut dependencies) = (vec![], vec![]);
    let (mut pairs, mut dependency_pairs, mut label_rows) =
        (HashSet::new(), HashSet::new(), HashSet::new());

    for relation in relations {
        let line = relation.line;
        let find = |name: &str| {
            endpoint(name).ok_or_else(|| format!("line {line}: unknown endpoint '{name}'"))
        };
        let (source, target) = (find(&relation.source)?, find(&relation.target)?);
        if relation.source == relation.target {
            return Err(format!("line {line}: a relation cannot be a self relation"));
        }
        match (relation.kind, source, target) {
            (RelationKind::Association, Endpoint::Actor(actor), Endpoint::UseCase(use_case))
            | (RelationKind::Association, Endpoint::UseCase(use_case), Endpoint::Actor(actor)) => {
                let actor = actors[actor].name.clone();
                if !pairs.insert((actor.clone(), use_case)) {
                    return Err(format!(
                        "line {line}: duplicate relation between '{}' and '{}'",
                        relation.source, relation.target
                    ));
                }
                associations.push(Association { actor, use_case });
            }
            (RelationKind::Association, Endpoint::Actor(_), Endpoint::Actor(_)) => {
                return Err(format!("line {line}: association cannot connect two actors"));
            }
            (RelationKind::Association, Endpoint::UseCase(_), Endpoint::UseCase(_)) => {
                return Err(format!("line {line}: association cannot connect two use cases"));
            }
            (_, Endpoint::Actor(_), _) | (_, _, Endpoint::Actor(_)) => {
                return Err(format!("line {line}: dependency cannot involve an actor"));
            }
            (_, Endpoint::UseCase(source), Endpoint::UseCase(target)) => {
                if !dependency_pairs.insert((source, target)) {
                    return Err(format!(
                        "line {line}: duplicate dependency between '{}' and '{}'",
                        relation.source, relation.target
                    ));
                }
                let source_row = 2 + 4 * source;
                let label_row = if target > source { source_row + 2 } else { source_row - 2 };
                if !label_rows.insert(label_row) {
                    return Err(format!(
                        "line {line}: dependency label row overlaps another dependency"
                    ));
                }
                let kind = if relation.kind == RelationKind::Include {
                    DependencyKind::Include
                } else {
                    DependencyKind::Extend
                };
                dependencies.push(Dependency { source, target, kind });
            }
        }
    }
    Ok((associations, dependencies))
}

struct Layout {
    boundary_left: usize,
    boundary_right: usize,
    boundary_h: usize,
    inner_w: usize,
    right_col: usize,
    uc_left: usize,
    text_w: usize,
    canvas_w: usize,
    canvas_h: usize,
}

impl Layout {
    fn new(diagram: &Diagram) -> Self {
        let count = |side| diagram.actors.iter().filter(|actor| actor.side == side).count();
        let max = |side| {
            diagram
                .actors
                .iter()
                .filter(|actor| actor.side == side)
                .map(Actor::width)
                .max()
                .unwrap_or(0)
        };
        let (n_l, n_r) = (count(Side::Left), count(Side::Right));
        let (left_max, right_max) = (max(Side::Left), max(Side::Right));
        let uc_w =
            diagram.use_cases.iter().map(|name| name.chars().count() + 4).max().expect("use case");
        let n_lanes = diagram
            .dependencies
            .iter()
            .filter(|dependency| dependency.source.abs_diff(dependency.target) != 1)
            .count();
        let boundary_left = left_max + if n_l > 0 { 2 * n_l + 1 } else { 0 };
        let uc_left = boundary_left + 2;
        let text_w = if diagram.dependencies.is_empty() { uc_w } else { uc_w.max(11) };
        let inner_w = (text_w + 2 + 2 * n_lanes).max(diagram.system.chars().count() + 4);
        let boundary_right = boundary_left + inner_w + 1;
        let right_col = boundary_right + 1 + if n_r > 0 { 2 * n_r + 1 } else { 0 };
        let boundary_h = 4 * diagram.use_cases.len() + 1;
        let actor_h = if n_l.max(n_r) == 0 { 0 } else { 5 * n_l.max(n_r) - 1 };
        let canvas_w = if n_r > 0 {
            (boundary_right + 1).max(right_col + right_max)
        } else {
            boundary_right + 1
        };

        Self {
            boundary_left,
            boundary_right,
            boundary_h,
            inner_w,
            right_col,
            uc_left,
            text_w,
            canvas_w,
            canvas_h: boundary_h.max(actor_h),
        }
    }
}

impl Diagram {
    fn render(&self) -> String {
        let layout = Layout::new(self);
        let mut canvas = vec![vec![' '; layout.canvas_w]; layout.canvas_h];
        self.draw_boundary(&layout, &mut canvas);
        self.draw_use_cases(&layout, &mut canvas);
        self.draw_actors(&layout, &mut canvas);
        self.draw_associations(&layout, &mut canvas);
        self.draw_dependencies(&layout, &mut canvas);
        canvas
            .into_iter()
            .map(|row| String::from_iter(row).trim_end().to_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn draw_boundary(&self, layout: &Layout, canvas: &mut [Vec<char>]) {
        let (left, right, bottom) =
            (layout.boundary_left, layout.boundary_right, layout.boundary_h - 1);
        canvas[0][left..=right].fill('-');
        canvas[bottom][left..=right].fill('-');
        for (row, col) in [(0, left), (0, right), (bottom, left), (bottom, right)] {
            canvas[row][col] = '+';
        }
        for row in canvas.iter_mut().take(bottom).skip(1) {
            row[left] = '|';
            row[right] = '|';
        }
        let start = left + 1 + (layout.inner_w - self.system.chars().count() - 2) / 2;
        draw_text(canvas, 0, start, &format!(" {} ", self.system));
    }

    fn draw_use_cases(&self, layout: &Layout, canvas: &mut [Vec<char>]) {
        for (index, name) in self.use_cases.iter().enumerate() {
            draw_text(canvas, 2 + 4 * index, layout.uc_left, &format!("( {name} )"));
        }
    }

    fn draw_actors(&self, layout: &Layout, canvas: &mut [Vec<char>]) {
        for (actor, index) in self.actors.iter().zip(self.actor_indices()) {
            let start = if actor.side == Side::Left { 0 } else { layout.right_col };
            let (width, top) = (actor.width(), 5 * index);
            let left = start + (width - 3) / 2;
            draw_text(canvas, top, left, " o ");
            draw_text(canvas, top + 1, left, "/|\\");
            draw_text(canvas, top + 2, left, "/ \\");
            draw_text(
                canvas,
                top + 3,
                start + (width - actor.name.chars().count()) / 2,
                &actor.name,
            );
        }
    }

    fn draw_associations(&self, layout: &Layout, canvas: &mut [Vec<char>]) {
        for (actor, index) in self.actors.iter().zip(self.actor_indices()) {
            let mut linked =
                self.associations.iter().filter(|association| association.actor == actor.name);
            let Some(first) = linked.next() else { continue };
            let figure_left = (actor.width() - 3) / 2;
            let arms_row = 5 * index + 1;
            let (lane, start) = match actor.side {
                Side::Left => (layout.boundary_left - 2 - 2 * index, figure_left + 3),
                Side::Right => {
                    (layout.boundary_right + 2 + 2 * index, layout.right_col + figure_left - 1)
                }
            };
            draw_hline(canvas, arms_row, start, lane, '-');
            canvas[arms_row][lane] = '+';
            for association in std::iter::once(first).chain(linked) {
                let row = 2 + 4 * association.use_case;
                draw_vline(canvas, lane, arms_row, row, '|');
                let right =
                    layout.uc_left + self.use_cases[association.use_case].chars().count() + 3;
                match actor.side {
                    Side::Left => draw_hline(canvas, row, lane, layout.uc_left - 1, '-'),
                    Side::Right => draw_hline(canvas, row, right + 1, lane, '-'),
                }
            }
        }
    }

    fn draw_dependencies(&self, layout: &Layout, canvas: &mut [Vec<char>]) {
        let (down_x, up_x) = (layout.uc_left + 2, layout.uc_left + 4);
        let mut next_lane = 0;
        for dependency in &self.dependencies {
            let (source_row, target_row) = (2 + 4 * dependency.source, 2 + 4 * dependency.target);
            let (port, port_row, label_row, arrow_row, arrow) = if target_row > source_row {
                (down_x, source_row + 1, source_row + 2, target_row - 1, 'v')
            } else {
                (up_x, source_row - 1, source_row - 2, target_row + 1, '^')
            };
            draw_line(canvas, port_row, port, '|');
            if dependency.source.abs_diff(dependency.target) != 1 {
                let lane = layout.boundary_left + 3 + layout.text_w + 2 * next_lane;
                next_lane += 1;
                draw_hline(
                    canvas,
                    label_row,
                    port + dependency.kind.label().chars().count(),
                    lane - 1,
                    '.',
                );
                draw_line(canvas, label_row, lane, '+');
                let (top, bottom) = if label_row < arrow_row {
                    (label_row + 1, arrow_row - 1)
                } else {
                    (arrow_row + 1, label_row - 1)
                };
                draw_vline(canvas, lane, top, bottom, '|');
                draw_line(canvas, arrow_row, lane, '+');
                draw_hline(canvas, arrow_row, port + 1, lane - 1, '.');
            }
            draw_text(canvas, label_row, port, dependency.kind.label());
            canvas[arrow_row][port] = arrow;
        }
    }

    fn actor_indices(&self) -> Vec<usize> {
        let (mut left, mut right) = (0, 0);
        self.actors
            .iter()
            .map(|actor| match actor.side {
                Side::Left => {
                    let index = left;
                    left += 1;
                    index
                }
                Side::Right => {
                    let index = right;
                    right += 1;
                    index
                }
            })
            .collect()
    }
}

fn draw_line(canvas: &mut [Vec<char>], row: usize, col: usize, glyph: char) {
    let current = canvas[row][col];
    let horizontal = |character: char| matches!(character, '-' | '.');
    let vertical = |character: char| character == '|';
    canvas[row][col] = if current == ' ' {
        glyph
    } else if current == '+'
        || glyph == '+'
        || (horizontal(current) && vertical(glyph))
        || (vertical(current) && horizontal(glyph))
    {
        '+'
    } else {
        current
    };
}

fn draw_text(canvas: &mut [Vec<char>], row: usize, col: usize, text: &str) {
    for (offset, character) in text.chars().enumerate() {
        canvas[row][col + offset] = character;
    }
}

fn draw_hline(canvas: &mut [Vec<char>], row: usize, start: usize, end: usize, glyph: char) {
    for col in start.min(end)..=start.max(end) {
        draw_line(canvas, row, col, glyph);
    }
}

fn draw_vline(canvas: &mut [Vec<char>], col: usize, start: usize, end: usize, glyph: char) {
    for row in start.min(end)..=start.max(end) {
        draw_line(canvas, row, col, glyph);
    }
}

pub fn run(input: &str) -> Result<String, String> {
    Diagram::parse(input).map(|diagram| diagram.render())
}

#[cfg(test)]
mod tests {
    use super::{run, validate_name};

    fn render(input: &str) -> String {
        run(input).unwrap_or_else(|error| panic!("expected success, got error: {error}"))
    }

    #[test]
    fn renders_the_golden_example() {
        let input = "\
system Online Shop {
  Browse catalog
  Place order
  Pay
}

actor Customer
actor(right) Payment Gateway

Customer --> Browse catalog
Customer --> Place order
Browse catalog ..> Pay: include
Place order ..> Pay: extend
Payment Gateway --> Pay
";
        let expected = concat!(
            "   o       +---- Online Shop -----+          o\n",
            "  /|\\----+ |                      | +-------/|\\\n",
            "  / \\    +-+-( Browse catalog )   | |       / \\\n",
            "Customer | |   |                  | | Payment Gateway\n",
            "         | |   include..........+ | |\n",
            "         | |                    | | |\n",
            "         +-+-( Place order )    | | |\n",
            "           |   |                | | |\n",
            "           |   extend           | | |\n",
            "           |   v................+ | |\n",
            "           | ( Pay )--------------+-+\n",
            "           |                      |\n",
            "           +----------------------+",
        );
        assert_eq!(render(input), expected);
    }

    #[test]
    fn renders_actor_variants() {
        let output = render(
            "system S {\n  A\n}\nactor Default\nactor(left) Left Actor\nactor(right) Right Actor\nactor Unused\nDefault --> A\nLeft Actor --> A\nRight Actor --> A\n",
        );
        let lines: Vec<_> = output.lines().collect();
        assert!(lines[3].starts_with("Default") && lines[3].ends_with("Right Actor"));
        assert!(lines[8].starts_with("Left Actor"));
        assert!(lines[13].starts_with("Unused"));
        assert!(lines[1].contains('+') && lines[6].contains('+'));
    }

    #[test]
    fn renders_associations() {
        let output = render(
            "system S {\n  A\n  B\n}\nactor Person\nactor(right) R\nPerson --> A\nPerson --> B\nB --> R\n",
        );
        assert!(output.lines().any(|line| line.contains("Person | |")));
        assert!(output.contains("( A )") && output.contains("( B )-"));
        let ports = render("system S {\nA\n}\nactor L\nactor(right) R\nL --> A\nR --> A\n");
        assert!(ports.lines().nth(2).unwrap().contains("-( A )-"));
    }

    #[test]
    fn routes_dependencies() {
        let down = render("system S {\nA\nB\n}\nA ..> B: include\n");
        assert!(down.lines().nth(3).unwrap().contains('|'));
        assert!(down.lines().nth(4).unwrap().contains("include"));
        assert!(down.lines().nth(5).unwrap().contains('v'));
        let up = render("system S {\nA\nB\n}\nB ..> A: extend\n");
        assert!(up.lines().nth(5).unwrap().contains('|'));
        assert!(up.lines().nth(4).unwrap().contains("extend"));
        assert!(up.lines().nth(3).unwrap().contains('^'));
        let lane = render("system S {\nA\nB\nC\n}\nA ..> C: include\n");
        assert!(lane.lines().any(|line| line.contains("include") && line.contains('+')));
        assert!(lane.lines().any(|line| line.contains('v') && line.contains('+')));
        let uplane = render("system S {\nA\nB\nC\n}\nC ..> A: extend\n");
        assert!(uplane.lines().any(|line| line.contains('^') && line.contains('+')));
        assert!(uplane.lines().any(|line| line.contains("extend") && line.contains('+')));
    }

    #[test]
    fn ignores_comments_and_accepts_names() {
        let input = "\
# comment
system My.System {
  # inside
  Use-Case One
}
# between
actor(right) The User
The User --> Use-Case One
";
        let output = render(input);
        assert!(
            output.contains("My.System")
                && output.contains("( Use-Case One )")
                && output.contains("The User")
        );
    }

    #[test]
    fn rejects_empty_name() {
        assert_eq!(validate_name("").unwrap_err(), "name must not be empty");
    }

    #[test]
    fn rejects_parser_and_validation_errors() {
        let cases = [
            ("", "missing system block"),
            ("actor A\n", "missing system block"),
            ("system S {\nA\n", "unclosed system block"),
            ("system{\nA\n}\n", "system name must not be empty"),
            ("system {\nA\n}\n", "system name must not be empty"),
            ("system S\nA\n", "expected '{'"),
            ("system S { A\n", "system contents must start on the next line"),
            ("system S {\n}\n", "has no use cases"),
            ("system S {\nA\n}\nsystem T {\nB\n}\n", "duplicate system block"),
            ("system S {\nA\n}\nactor\n", "actor name must not be empty"),
            ("system S {\nA\n}\nactor(bogus): B\n", "malformed actor line"),
            ("system S {\nA\n}\nactor(left)Name\n", "malformed actor line"),
            ("system S {\nA\n}\nactor(left) B\nactor B\n", "duplicate name 'B'"),
            ("system S {\nA\n}\nactor: B\n", "malformed actor line"),
            ("system S {\nA\n}\nactor(left): B\n", "malformed actor line"),
            ("system S {\nA\n}\nactor(right): B\n", "malformed actor line"),
            ("system S {\nA\nA\n}\n", "duplicate name 'A'"),
            ("system S {\nA\n}\nactor X\nX --> Y\n", "unknown endpoint 'Y'"),
            ("system S {\nA\n}\nactor X\nZ --> A\n", "unknown endpoint 'Z'"),
            ("system S {\nA\n}\nactor X\nX --> Z\n", "unknown endpoint 'Z'"),
            ("system S {\nA\nB\n}\nactor X\nX --> A\nX --> A\n", "duplicate relation"),
            ("system S {\nA\nB\n}\nA ..> B: include\nA ..> B: extend\n", "duplicate dependency"),
            ("system S {\nA\n}\nactor X\nactor Y\nX --> Y\n", "two actors"),
            ("system S {\nA\nB\n}\nA --> B\n", "two use cases"),
            ("system S {\nA\n}\nactor X\nX ..> A: include\n", "involve an actor"),
            ("system S {\nA\n}\nA ..> A: include\n", "self relation"),
            ("system S {\nA\nB\n}\nA ..> B\n", "missing dependency label"),
            ("system S {\nA\nB\n}\nA ..> B:\n", "missing dependency label"),
            ("system S {\nA\nB\n}\nA ..> B: extends\n", "invalid dependency label 'extends'"),
            ("system S {\nA\n}\n--> A\n", "association source must not be empty"),
            ("system S {\nA\n}\nA -->\n", "association target must not be empty"),
            ("system S {\nA\nB\n}\n..> B: include\n", "dependency source must not be empty"),
            ("system S {\nA\nB\n}\nA ..> : include\n", "dependency target must not be empty"),
            ("system S {\nA\nB\nC\n}\nA ..> B: include\nA ..> C: extend\n", "label row overlaps"),
            ("system S {\nA\nB\n}\nA ..> B: include\nB ..> A: extend\n", "label row overlaps"),
            ("system S {\nA{B\n}\n", "invalid name"),
            ("system S {\nA:B\n}\n", "invalid name"),
            ("system S {\nA|B\n}\n", "invalid name"),
            ("system S {\nA\u{7}\n}\n", "invalid name"),
            ("system Bad\u{7} {\nA\n}\n", "invalid name"),
            ("system S {\nA\n}\nactor X\nX --> A:B\n", "invalid name"),
            ("system S {\nA\n}\nnot a declaration\n", "expected a system block, actor declaration"),
        ];
        for (input, expected) in cases {
            let error = run(input).unwrap_err();
            assert!(error.contains(expected), "expected {expected:?} for {input:?}, got {error:?}");
        }
    }
}
