pub mod architecture;
mod block;
pub mod class;
pub mod sequence;
pub mod usecase;

// TODO: Share comment, name, and error helpers to prevent parser duplication.
pub fn validate_name(name: &str, prefix: &str, forbidden: &[char]) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".to_owned());
    }
    // Reject control characters because they cannot be drawn safely.
    if name.chars().any(|character| character.is_control() || forbidden.contains(&character)) {
        return Err(format!("invalid {prefix}name '{name}'"));
    }
    Ok(())
}
