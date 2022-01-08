mod validators;

use crate::validators::is_between;
use anyhow::{Context, Result};
use crafty::{Player, Simulator};
use crafty_models::RecipeVariant;
use dialoguer::{console::Style, theme::ColorfulTheme, Input, Select};
use structopt::StructOpt;

include!(concat!(env!("OUT_DIR"), "/recipes.rs"));

#[derive(Debug, StructOpt)]
#[structopt(name = "crafty")]
/// a ffxiv crafting optimization tool
struct CliArgs {
    /// the player's job level
    job_level: u32,

    /// the player's craftsmanship stat
    craftsmanship: u32,

    /// the player's control stat
    control: u32,

    /// the player's cp stat
    cp: u32,
}

fn main() -> Result<()> {
    ctrlc::set_handler(|| {
        let term = dialoguer::console::Term::stdout();
        let _ = term.show_cursor();
    })?;

    let args = CliArgs::from_args();

    is_between(&args.job_level, 1, 90, "job level")?;
    is_between(&args.craftsmanship, 1, 5000, "craftsmanship")?;
    is_between(&args.control, 1, 5000, "control")?;
    is_between(&args.cp, 1, 700, "cp")?;

    let recipe_job_level: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("recipe level? (1-90)")
        .with_initial_text("90")
        .validate_with(|input: &u32| is_between(input, 1, 90, "recipe level"))
        .interact_text()?;

    let possible_recipes = RECIPES.get(&recipe_job_level).unwrap();

    let recipe = select_prompt(possible_recipes, "recipe variant?")?;
    // println!("{:?}", recipe);

    let player = Player::new(
        args.job_level,
        args.craftsmanship,
        args.control,
        args.cp,
        recipe,
    );

    print_info("player stats:", &format!("{}", player));

    let sim = Simulator::new(recipe, &player);

    Ok(())
}

fn print_info(header: &str, details: &str) {
    let cyan = Style::new().green();
    println!("\n{}\n  {}\n", header, cyan.apply_to(details));
}

fn select_prompt<'a, T>(items: &'a [T], prompt: &str) -> Result<&'a T>
where
    T: std::fmt::Display,
{
    if items.len() > 1 {
        let selected = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .items(items)
            .default(0)
            .interact_opt()?
            .context("no item selected")?;
        Ok(&items[selected])
    } else {
        Ok(&items[0])
    }
}
