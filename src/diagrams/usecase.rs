use std::collections::HashSet;

use super::block::{canvas, draw_frame, draw_hline, draw_text, draw_vline, put, render_canvas};
use super::{parse_document, single_block};

super::run!(Diagram);

struct Actor {
    name: String,
    right: bool,
}

struct Edge {
    src: usize,
    dst: usize,
    dep: Option<&'static str>,
}

struct Diagram {
    system: String,
    ucs: Vec<String>,
    actors: Vec<Actor>,
    edges: Vec<Edge>,
}

struct Relation {
    src: String,
    dst: String,
    dep: Option<&'static str>,
    ln: usize,
}

impl Diagram {
    fn parse(input: &str) -> Result<Self, String> {
        let doc = parse_document(input, "system")?;
        let block = single_block(&doc, "system")?;
        let system = block.name.clone();
        if !system.is_empty() {
            check_name(block.line, &system)?;
        }
        let mut names = HashSet::new();
        let mut ucs = Vec::new();
        for row in &block.rows {
            let [entry] = row.as_slice() else {
                return Err(format!("line {}: system rows support one use case", row[0].1));
            };
            let (name, ln) = entry;
            check_name(*ln, name)?;
            if !names.insert(name.clone()) {
                return Err(format!("line {ln}: duplicate name '{name}'"));
            }
            ucs.push(name.clone());
        }
        if ucs.is_empty() {
            return Err(format!("line {}: empty system block has no use cases", block.line));
        }
        let (mut actors, mut rels) = (Vec::new(), Vec::new());
        for (ln, text) in doc.statements {
            if text.contains("..>") || text.contains("-->") {
                rels.push(parse_relation(ln, &text)?);
            } else if text.strip_prefix("actor").is_some_and(|r| {
                r.is_empty() || r.starts_with(char::is_whitespace) || r.starts_with([':', '('])
            }) {
                let (name, right) = parse_actor(ln, &text)?;
                if !names.insert(name.clone()) {
                    return Err(format!("line {ln}: duplicate name '{name}'"));
                }
                actors.push(Actor { name, right });
            } else {
                let msg =
                    "a system block, actor declaration, association ('-->'), or dependency ('..>')";
                return Err(format!("line {ln}: expected {msg}"));
            }
        }
        let edges = resolve(&ucs, &actors, rels)?;
        Ok(Self { system, ucs, actors, edges })
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    super::validate_name(name, "", &['{', '}', '#', ',', ':', '|', '<', '>', '(', ')'])
}

fn check_name(ln: usize, name: &str) -> Result<(), String> {
    validate_name(name).map_err(|m| format!("line {ln}: {m}"))
}

fn parse_actor(ln: usize, text: &str) -> Result<(String, bool), String> {
    let rest = text.strip_prefix("actor").unwrap_or_default().trim_start();
    let bad = || format!("line {ln}: malformed actor line");
    let (right, name) = if let Some(a) = rest.strip_prefix('(') {
        let (side, a) = a.split_once(')').ok_or_else(bad)?;
        let right = ["left", "right"].iter().position(|s| *s == side).map(|i| i == 1);
        if !a.starts_with(char::is_whitespace) {
            return Err(bad());
        }
        (right.ok_or_else(bad)?, a.trim_start())
    } else if rest.starts_with(':') {
        return Err(bad());
    } else {
        (false, rest)
    };
    let name = name.trim();
    if name.is_empty() {
        return Err(format!("line {ln}: actor name must not be empty"));
    }
    check_name(ln, name)?;
    Ok((name.to_owned(), right))
}

fn parse_relation(ln: usize, text: &str) -> Result<Relation, String> {
    if let Some((src, rest)) = text.split_once("..>") {
        let missing = || format!("line {ln}: missing dependency label");
        let (dst, label) = rest.split_once(':').ok_or_else(missing)?;
        let (src, dst) = ends(ln, "dependency", src, dst)?;
        let dep = ["include", "extend"].into_iter().find(|d| *d == label.trim());
        let dep = dep.ok_or_else(|| match label.trim() {
            "" => missing(),
            label => format!("line {ln}: invalid dependency label '{label}'"),
        })?;
        return Ok(Relation { src, dst, dep: Some(dep), ln });
    }
    let bad = || format!("line {ln}: expected an association ('-->')");
    let (src, dst) = text.split_once("-->").ok_or_else(bad)?;
    let (src, dst) = ends(ln, "association", src, dst)?;
    Ok(Relation { src, dst, dep: None, ln })
}

fn ends(ln: usize, kind: &str, a: &str, b: &str) -> Result<(String, String), String> {
    let (a, b) = (a.trim(), b.trim());
    if a.is_empty() || b.is_empty() {
        let side = if a.is_empty() { "source" } else { "target" };
        return Err(format!("line {ln}: {kind} {side} must not be empty"));
    }
    check_name(ln, a)?;
    check_name(ln, b)?;
    Ok((a.to_owned(), b.to_owned()))
}

#[derive(Clone, Copy)]
enum Ep {
    A(usize),
    U(usize),
}

fn resolve(ucs: &[String], actors: &[Actor], rels: Vec<Relation>) -> Result<Vec<Edge>, String> {
    let (mut edges, mut pairs) = (Vec::new(), HashSet::new());
    let (mut deps, mut rows) = (HashSet::new(), HashSet::new());
    for Relation { src: rs, dst: rd, dep, ln } in rels {
        let find = |n: &str| {
            let u = ucs.iter().position(|u| u == n).map(Ep::U);
            u.or_else(|| actors.iter().position(|a| a.name == n).map(Ep::A))
                .ok_or_else(|| format!("line {ln}: unknown endpoint '{n}'"))
        };
        let (src, dst) = (find(&rs)?, find(&rd)?);
        if rs == rd {
            return Err(format!("line {ln}: a relation cannot be a self relation"));
        }
        if dep.is_none() {
            let (actor, uc) = match (src, dst) {
                (Ep::A(a), Ep::U(u)) | (Ep::U(u), Ep::A(a)) => (a, u),
                (Ep::A(_), Ep::A(_)) => return Err(format!("line {ln}: two actors")),
                (Ep::U(_), Ep::U(_)) => return Err(format!("line {ln}: two use cases")),
            };
            if !pairs.insert((actor, uc)) {
                return Err(format!("line {ln}: duplicate relation"));
            }
            edges.push(Edge { src: actor, dst: uc, dep: None });
            continue;
        }
        let (Ep::U(src), Ep::U(dst)) = (src, dst) else {
            return Err(format!("line {ln}: involve an actor"));
        };
        if !deps.insert((src, dst)) {
            return Err(format!("line {ln}: duplicate dependency"));
        }
        let row = 2 + 4 * src;
        let lr = if dst > src { row + 2 } else { row - 2 };
        if !rows.insert(lr) {
            return Err(format!("line {ln}: label row overlaps"));
        }
        edges.push(Edge { src, dst, dep });
    }
    Ok(edges)
}

struct Layout {
    left: usize,
    right: usize,
    rcol: usize,
    tw: usize,
}

impl Layout {
    fn new(d: &Diagram) -> (Self, usize, usize) {
        let (mut n, mut w) = ([0; 2], [0; 2]);
        for a in &d.actors {
            let i = usize::from(a.right);
            n[i] += 1;
            w[i] = w[i].max(a.name.chars().count().max(3));
        }
        let ([nl, nr], [lw, rw]) = (n, w);
        let uw = d.ucs.iter().map(|n| n.chars().count() + 4).max().unwrap_or(0);
        let has = d.edges.iter().any(|e| e.dep.is_some());
        let lanes = d.edges.iter().filter(|e| e.dep.is_some() && e.src.abs_diff(e.dst) > 1).count();
        let left = lw + 2 * nl + usize::from(nl > 0);
        let tw = if has { uw.max(13) } else { uw };
        let right = left + (tw + 2 + 2 * lanes).max(d.system.chars().count() + 4) + 1;
        let rcol = right + 1 + 2 * nr + usize::from(nr > 0);
        let w = if nr > 0 { (right + 1).max(rcol + rw) } else { right + 1 };
        let h = (4 * d.ucs.len() + 1).max(5 * nl.max(nr));
        (Self { left, right, rcol, tw }, w, h)
    }
}

impl Diagram {
    fn render(&self) -> String {
        let (l, w, h) = Layout::new(self);
        let mut c = canvas(w, h);
        draw_frame(&mut c, (l.left, 0, l.right, 4 * self.ucs.len()), &self.system);
        for (i, n) in self.ucs.iter().enumerate() {
            draw_text(&mut c, 2 + 4 * i, l.left + 2, &format!("( {n} )"));
        }
        self.draw_actors(&l, &mut c);
        self.draw_dependencies(&l, &mut c);
        render_canvas(&c)
    }

    fn draw_actors(&self, layout: &Layout, canvas: &mut [Vec<char>]) {
        let mut index = [0; 2];
        for (g, a) in self.actors.iter().enumerate() {
            let idx = index[usize::from(a.right)];
            index[usize::from(a.right)] += 1;
            let start = if a.right { layout.rcol } else { 0 };
            let (width, top) = (a.name.chars().count().max(3), 1 + 5 * idx);
            let left = start + (width - 3) / 2;
            for (row, glyph) in [" o ", "/|\\", "/ \\"].into_iter().enumerate() {
                draw_text(canvas, top + row, left, glyph);
            }
            draw_text(canvas, top + 3, start + (width - a.name.chars().count()) / 2, &a.name);
            let mut links = self.edges.iter().filter(|e| e.dep.is_none() && e.src == g).peekable();
            let Some(_) = links.peek() else { continue };
            let arms = top + 1;
            let lane = if a.right { layout.right + 2 + 2 * idx } else { layout.left - 2 - 2 * idx };
            let start = if a.right { left - 1 } else { left + 3 };
            draw_hline(canvas, arms, start, lane, '-');
            canvas[arms][lane] = '+';
            for e in links {
                let row = 2 + 4 * e.dst;
                draw_vline(canvas, lane, arms, row, '|');
                let right = layout.left + self.ucs[e.dst].chars().count() + 5;
                let (x, y) = if a.right { (right + 1, lane) } else { (lane, layout.left + 1) };
                draw_hline(canvas, row, x, y, '-');
            }
        }
    }

    fn draw_dependencies(&self, layout: &Layout, canvas: &mut [Vec<char>]) {
        let (down, up) = (layout.left + 4, layout.left + 6);
        let mut lane = 0;
        for e in self.edges.iter().filter(|e| e.dep.is_some()) {
            let (src, dst) = (2 + 4 * e.src, 2 + 4 * e.dst);
            let (pos, py, ch) = if dst > src { (down, dst - 1, 'v') } else { (up, dst + 1, '^') };
            let (pr, lr) = if dst > src { (src + 1, src + 2) } else { (src - 1, src - 2) };
            put(canvas, pos, pr, ':');
            let label = e.dep.unwrap();
            if e.src.abs_diff(e.dst) != 1 {
                let px = layout.left + 3 + layout.tw + 2 * lane;
                lane += 1;
                draw_hline(canvas, lr, pos + 2 + label.chars().count(), px - 1, '.');
                put(canvas, px, lr, '+');
                let (top, bottom) = (lr.min(py) + 1, lr.max(py) - 1);
                draw_vline(canvas, px, top, bottom, ':');
                put(canvas, px, py, '+');
                draw_hline(canvas, py, pos + 1, px - 1, '.');
            }
            draw_text(canvas, lr, pos, &format!(": {label}"));
            canvas[py][pos] = ch;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{parse_relation, run, validate_name};

    fn render(input: &str) -> String {
        run(input).unwrap()
    }

    #[test]
    fn renders_diagrams() {
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
            "           + Online Shop ---------+\n",
            "   o       |                      |          o\n",
            "  /|\\----+-+-( Browse catalog )   | +-------/|\\\n",
            "  / \\    | |   :                  | |       / \\\n",
            "Customer | |   : include........+ | | Payment Gateway\n",
            "         | |                    : | |\n",
            "         +-+-( Place order )    : | |\n",
            "           |   :                : | |\n",
            "           |   : extend         : | |\n",
            "           |   v................+ | |\n",
            "           | ( Pay )--------------+-+\n",
            "           |                      |\n",
            "           +----------------------+",
        );
        assert_eq!(render(input), expected);
        assert!(render("system {\nBrowse\n}").contains("( Browse )"));
        assert!(run("system S {\nBrowse, Order\n}").is_err());
        let output = render(
            "system S {\n  A\n}\nactor Default\nactor(left) Left Actor\nactor(right) Right Actor\nactor Unused\nDefault --> A\nLeft Actor --> A\nRight Actor --> A\n",
        );
        let lines: Vec<_> = output.lines().collect();
        assert!(lines[4].starts_with("Default") && lines[4].ends_with("Right Actor"));
        assert!(lines[9].starts_with("Left Actor") && lines[14].starts_with("Unused"));
        assert!(lines[2].contains('+') && lines[7].contains('+'));
        let output = render(
            "system S {\n  A\n  B\n}\nactor Person\nactor(right) R\nPerson --> A\nPerson --> B\nB --> R\n",
        );
        assert!(output.lines().any(|line| line.contains("Person | |")));
        assert!(output.contains("( A )") && output.contains("( B )-"));
        let ports = render("system S {\nA\n}\nactor L\nactor(right) R\nL --> A\nR --> A\n");
        assert!(ports.lines().nth(2).unwrap().contains("-( A )-"));
        let down: Vec<_> =
            render("system S {\nA\nB\n}\nA ..> B: include\n").lines().map(str::to_owned).collect();
        assert!(down[3].contains(':') && down[4].contains(": include"));
        assert!(down[5].contains('v'));
        let up: Vec<_> =
            render("system S {\nA\nB\n}\nB ..> A: extend\n").lines().map(str::to_owned).collect();
        assert!(up[5].contains(':') && up[4].contains(": extend"));
        assert!(up[3].contains('^'));
        let lane = render("system S {\nA\nB\nC\n}\nA ..> C: include\n");
        assert!(lane.lines().any(|line| line.contains("include") && line.contains('+')));
        assert!(lane.lines().any(|line| line.contains('v') && line.contains('+')));
        let uplane = render("system S {\nA\nB\nC\n}\nC ..> A: extend\n");
        assert!(uplane.lines().any(|line| line.contains('^') && line.contains('+')));
        assert!(uplane.lines().any(|line| line.contains("extend") && line.contains('+')));
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
    fn rejects_parser_and_validation_errors() {
        assert_eq!(validate_name("").unwrap_err(), "name must not be empty");
        assert_eq!(
            parse_relation(4, "X - Y").err().unwrap(),
            "line 4: expected an association ('-->')"
        );
        for (input, expected) in [
            ("", "missing system block"),
            ("actor A\n", "missing system block"),
            ("system S {\nA\n", "unclosed system"),
            ("system S\nA\n", "expected '{'"),
            ("system S { A\n", "unclosed system"),
            ("system S {\n}\n", "has no use cases"),
            ("system S {\nA\n}\nsystem T {\nB\n}\n", "duplicate system block"),
            ("system S {\nA\n}\nactor\n", "actor name must not be empty"),
            ("system S {\nA\n}\nactor(bogus): B\n", "malformed actor line"),
            ("system S {\nA\n}\nactor(left B\n", "malformed actor line"),
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
            ("system S {\nA\n}\nactor X:Y\n", "invalid name"),
            ("system S {\nA\n}\nX:Y --> A\n", "invalid name"),
            ("system S {\nA\nB\n}\nX:Y ..> B: include\n", "invalid name"),
            ("system S {\nA\nB\n}\nA ..> B|C: include\n", "invalid name"),
            ("system S {\nA\n}\nnot a declaration\n", "expected a system block, actor declaration"),
        ] {
            let error = run(input).unwrap_err();
            assert!(error.contains(expected), "expected {expected:?} for {input:?}, got {error:?}");
        }
    }
}
