use anyhow::{anyhow, Context, Result};
use crafty_models::RecipeVariant;
use dialoguer::{console::Style, theme::ColorfulTheme, Input, Select};
use structopt::StructOpt;

include!(concat!(env!("OUT_DIR"), "/recipes.rs"));

#[derive(Debug, StructOpt)]
#[structopt(name = "crafty")]
/// a ffxiv crafting optimization tool
struct Opt {
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

    let opt = Opt::from_args();

    let cyan = Style::new().green();

    is_between(&opt.job_level, 1, 90, "job level")?;
    is_between(&opt.craftsmanship, 1, 5000, "craftsmanship")?;
    is_between(&opt.control, 1, 5000, "control")?;
    is_between(&opt.cp, 1, 400, "cp")?;

    println!(
        "\n{}\n  lv{} / craftsmanship {} / control {} / cp {}\n",
        cyan.apply_to("player stats:"),
        opt.job_level,
        opt.craftsmanship,
        opt.control,
        opt.cp
    );

    let recipe_job_level: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("recipe level? (1-90)")
        .with_initial_text("90")
        .validate_with(|input: &u32| is_between(input, 1, 90, "recipe level"))
        .interact_text()?;

    let possible_recipes = RECIPES.get(&recipe_job_level).unwrap();
    let recipe = select_prompt(possible_recipes, "recipe variant?")?;

    println!("\n{}\n  {}", cyan.apply_to("selected recipe:"), recipe);

    Ok(())
}

fn is_between(value: &u32, min: u32, max: u32, label: &str) -> Result<()> {
    if value >= &min && value <= &max {
        Ok(())
    } else {
        Err(anyhow!("{} should be between {} and {}", label, min, max))
    }
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
