use recipe::Recipe;
use serde::{de, Deserialize};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::{env, process};

fn main() {
    if let Err(error) = process_recipe_tables() {
        println!("{}", error);
        process::exit(1);
    }
    if let Err(error) = process_level_table() {
        println!("{}", error);
        process::exit(1);
    }
}

/// Process Recipe.csv and RecipeLevelTable.csv. Neither table has all of the
/// information required for a single craft. Recipe.csv contains metadata for
/// every recipe, for which progress, quality, and durability (PQD) should be
/// derived from base values in RecipeLevelTable.csv.
///
/// Many crafts will have identical PQD requirements. Here, we merge
/// both CSV files and create a distinct list.
fn process_recipe_tables() -> Result<(), Box<dyn std::error::Error>> {
    let mut recipes_csv = csv::Reader::from_path("data/Recipe.csv")?;
    let mut recipe_levels_csv = csv::Reader::from_path("data/RecipeLevelTable.csv")?;

    // Process the recipe level table and create a lookup by recipe level
    let mut recipe_levels = HashMap::new();

    for record in recipe_levels_csv.deserialize::<RecipeLevelRecord>() {
        let recipe_level = record?;
        recipe_levels.insert(recipe_level.recipe_level, recipe_level);
    }

    // Convert base values from RecipeLevelTable to actual PQD requirements for
    // a recipe
    fn apply_factor(base: u32, factor: u32) -> u32 {
        ((base * factor) as f64 / 100.0).floor() as u32
    }

    // Process the recipe table, and keep track of distinct recipe variants
    let mut distinct_recipe_variants = HashSet::new();

    for record in recipes_csv.deserialize::<RecipeRecord>() {
        let recipe = record?;
        if recipe.can_hq {
            let base = recipe_levels.get(&recipe.recipe_level).unwrap();
            let variant = Recipe {
                recipe_level: recipe.recipe_level,
                job_level: base.job_level,
                stars: base.stars,
                progress: apply_factor(base.progress, recipe.progress_factor),
                quality: apply_factor(base.quality, recipe.quality_factor),
                durability: apply_factor(base.durability, recipe.durability_factor),
                progress_div: base.progress_divider,
                progress_mod: base.progress_modifier,
                quality_div: base.quality_divider,
                quality_mod: base.quality_modifier,
                is_expert: recipe.is_expert,
                conditions_flag: base.conditions_flag,
            };
            distinct_recipe_variants.insert(variant);
        }
    }

    // Sort and group recipes by job level for ease of selection
    let mut recipe_variants: Vec<_> = distinct_recipe_variants.into_iter().collect();
    recipe_variants.sort_by(|a, b| {
        let first = a.job_level.cmp(&b.job_level);
        let second = a.stars.cmp(&b.stars);
        let third = a.recipe_level.cmp(&b.recipe_level);
        let fourth = a.durability.cmp(&b.durability);
        first.then(second).then(third).then(fourth)
    });
    let mut recipes_by_level: HashMap<u32, Vec<Recipe>> = HashMap::new();
    for variant in recipe_variants {
        if let Entry::Vacant(entry) = recipes_by_level.entry(variant.job_level) {
            entry.insert(vec![variant]);
        } else {
            recipes_by_level
                .get_mut(&variant.job_level)
                .unwrap()
                .push(variant);
        }
    }

    // Prepare phf map
    let mut recipes = phf_codegen::Map::new();
    for (key, val) in recipes_by_level {
        let static_array = &format!("&{:?}", val);
        recipes.entry(key, static_array);
    }

    // Generate source files
    let out_dir = env::var("OUT_DIR")?;
    let out_path = Path::new(&out_dir).join("recipes.rs");
    let mut recipes_writer = BufWriter::new(File::create(&out_path).unwrap());
    writeln!(
        recipes_writer,
        "static RECIPES: phf::Map<u32, &'static [Recipe]> = {};\n",
        recipes.build()
    )?;

    Ok(())
}

/// Process LevelTable.csv, used to convert the player's job level to a base
/// recipe level.
fn process_level_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut levels_csv = csv::Reader::from_path("data/LevelTable.csv")?;

    // Process the level table
    let mut recipe_level_by_job_level = HashMap::new();

    for record in levels_csv.deserialize::<LevelRecord>() {
        let level = record?;
        recipe_level_by_job_level.insert(level.job_level, level.recipe_level);
    }

    // Prepare phf map
    let mut levels = phf_codegen::Map::new();
    for (key, val) in recipe_level_by_job_level {
        levels.entry(key, &val.to_string());
    }

    // Generate source files
    let out_dir = env::var("OUT_DIR")?;
    let out_path = Path::new(&out_dir).join("levels.rs");
    let mut levels_writer = BufWriter::new(File::create(&out_path).unwrap());
    writeln!(
        levels_writer,
        "static LEVELS: phf::Map<u32, u32> = {};\n",
        levels.build()
    )?;

    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LevelRecord {
    job_level: u32,
    recipe_level: u32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RecipeRecord {
    #[serde(rename = "RecipeLevelTable")]
    recipe_level: u32,

    #[serde(rename = "DifficultyFactor")]
    progress_factor: u32,

    #[serde(rename = "QualityFactor")]
    quality_factor: u32,

    #[serde(rename = "DurabilityFactor")]
    durability_factor: u32,

    #[serde(rename = "RequiredCraftsmanship")]
    required_craftsmanship: u32,

    #[serde(rename = "RequiredControl")]
    required_control: u32,

    #[serde(rename = "CanHq")]
    #[serde(deserialize_with = "bool_string")]
    can_hq: bool,

    #[serde(rename = "IsExpert")]
    #[serde(deserialize_with = "bool_string")]
    is_expert: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
struct RecipeLevelRecord {
    #[serde(rename = "#")]
    recipe_level: u32,

    #[serde(rename = "ClassJobLevel")]
    job_level: u32,

    #[serde(rename = "Stars")]
    stars: u32,

    #[serde(rename = "Durability")]
    durability: u32,

    #[serde(rename = "Difficulty")]
    progress: u32,

    #[serde(rename = "Quality")]
    quality: u32,

    #[serde(rename = "ProgressDivider")]
    progress_divider: u32,

    #[serde(rename = "QualityDivider")]
    quality_divider: u32,

    #[serde(rename = "ProgressModifier")]
    progress_modifier: u32,

    #[serde(rename = "QualityModifier")]
    quality_modifier: u32,

    #[serde(rename = "ConditionsFlag")]
    conditions_flag: u32,
}

fn bool_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: de::Deserializer<'de>,
{
    let b = String::deserialize(deserializer)?;
    match b.trim().to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(de::Error::custom("invalid boolean string")),
    }
}
