use wasm_bindgen::{prelude::*, JsCast};

// TODO: Create a proc derive macro that somehow serializes Rust structs into
// Typescript type definitions

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"

export interface Recipe {
    recipe_level: number,
    job_level: number,
    stars: number,
    progress: number,
    quality: number,
    durability: number,
    progress_div: number,
    progress_mod: number,
    quality_div: number,
    quality_mod: number,
    is_expert: boolean,
    conditions_flag: number,
}

"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Recipe[]")]
    pub type Recipes;
}

#[wasm_bindgen]
pub fn recipes(player_job_level: u32) -> Recipes {
    let recipes = crafty::data::recipes(player_job_level);
    serde_wasm_bindgen::to_value(&recipes)
        .unwrap()
        .unchecked_into()
}
