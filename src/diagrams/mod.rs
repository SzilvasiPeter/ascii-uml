pub mod architecture;
mod block;
pub mod class;
pub mod sequence;
pub mod usecase;

// TODO: extract a shared parsing layer: control characters, empty names,
// blank lines and comments, and `line N: ...` errors are duplicated per grammar.
pub fn validate_name(name: &str, prefix: &str, forbidden: &[char]) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".to_owned());
    }
    // Control characters cannot be drawn; they would corrupt the layout.
    if name.chars().any(|character| character.is_control() || forbidden.contains(&character)) {
        return Err(format!("invalid {prefix}name '{name}'"));
    }
    Ok(())
}
