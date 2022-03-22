use serde::{Deserialize, Serialize};
use std::fmt;

// Must be separate from the `crafty` crate so it can be used in `crafty/build.rs`

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Recipe {
    pub recipe_level: u32,
    pub job_level: u32,
    pub stars: u32,
    pub progress: u32,
    pub quality: u32,
    pub durability: u32,
    pub progress_div: u32,
    pub progress_mod: u32,
    pub quality_div: u32,
    pub quality_mod: u32,
    pub is_expert: bool,
    pub conditions_flag: u32,
}

impl fmt::Display for Recipe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let stars = (0..self.stars).map(|_| "★").collect::<String>();
        write!(
            f,
            "({:>3}) lv{:>2} {} / {:>5} progress / {:>5} quality / {:>2} durability",
            self.recipe_level, self.job_level, stars, self.progress, self.quality, self.durability
        )
    }
}
