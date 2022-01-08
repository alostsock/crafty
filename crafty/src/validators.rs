use anyhow::{anyhow, Result};

pub fn is_between(value: &u32, min: u32, max: u32, label: &str) -> Result<()> {
    if value >= &min && value <= &max {
        Ok(())
    } else {
        Err(anyhow!("{} should be between {} and {}", label, min, max))
    }
}
