/// Generates the CLI entry point shared by every diagram module.
macro_rules! run {
    ($diagram:ident) => {
        pub fn run(input: &str) -> Result<String, String> {
            $diagram::parse(input).map(|diagram| diagram.render())
        }
    };
}
pub(super) use run;

pub mod architecture;
mod block;
pub mod class;
pub mod sequence;
pub mod usecase;

// TODO: Share name and error helpers across diagram parsers.
pub fn validate_name(name: &str, prefix: &str, forbidden: &[char]) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".to_owned());
    }
    // Reject control characters because they cannot be drawn safely.
    if name.chars().any(|character| character.is_control() || forbidden.contains(&character)) {
        return Err(format!("invalid {prefix}name '{}'", name.escape_debug()));
    }
    Ok(())
}

pub struct Block {
    pub name: String,
    pub line: usize,
    pub rows: Vec<Vec<(String, usize)>>,
}

pub struct Document {
    pub blocks: Vec<Block>,
    pub statements: Vec<(usize, String)>,
}

pub fn single_block<'a>(document: &'a Document, kind: &str) -> Result<&'a Block, String> {
    let [block] = document.blocks.as_slice() else {
        return Err(format!(
            "{} {kind} block",
            if document.blocks.is_empty() { "missing" } else { "duplicate" }
        ));
    };
    Ok(block)
}

pub fn parse_document(input: &str, expected: &str) -> Result<Document, String> {
    let mut document = Document { blocks: Vec::new(), statements: Vec::new() };
    let mut current = None;
    for (index, raw) in input.lines().enumerate() {
        let line = index + 1;
        let text = raw.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let opening = current.is_none();
        let mut block = if let Some(block) = current.take() {
            if text.starts_with(expected) && text.contains('{') {
                return Err(format!("line {line}: nested {expected}"));
            }
            block
        } else if let Some(brace) = text.find('{') {
            let header = text[..brace].trim();
            let name = header
                .strip_prefix(expected)
                .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
                .ok_or_else(|| format!("line {line}: mixed block kinds"))?;
            Block { name: name.trim().to_owned(), line, rows: Vec::new() }
        } else {
            if text == "}" {
                return Err(format!("line {line}: unexpected '}}'"));
            }
            if text
                .strip_prefix(expected)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
            {
                return Err(format!("line {line}: expected '{{'"));
            }
            document.statements.push((line, text.to_owned()));
            continue;
        };
        let content = if opening { text.split_once('{').unwrap().1 } else { text };
        let closed = content.ends_with('}');
        append_rows(&mut block, content.strip_suffix('}').unwrap_or(content), line);
        if closed {
            document.blocks.push(block);
        } else {
            current = Some(block);
        }
    }
    if let Some(block) = current {
        return Err(format!("line {}: unclosed {expected}", block.line));
    }
    Ok(document)
}

fn append_rows(block: &mut Block, content: &str, line: usize) {
    for row in content.split(';') {
        let row: Vec<_> = row
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| (item.to_owned(), line))
            .collect();
        if !row.is_empty() {
            block.rows.push(row);
        }
    }
}
