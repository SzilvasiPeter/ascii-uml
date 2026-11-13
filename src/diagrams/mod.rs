pub mod class;
pub mod sequence;
pub mod usecase;

pub fn validate_name(name: &str, prefix: &str, forbidden: &[char]) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".to_owned());
    }
    if name.chars().any(|character| character.is_control() || forbidden.contains(&character)) {
        return Err(format!("invalid {prefix}name '{name}'"));
    }
    Ok(())
}
