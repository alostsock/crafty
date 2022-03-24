use anyhow::{anyhow, Context, Result};
use clap::Parser;
use crafty::{player::Player, recipes::recipes_by_level, simulator::Simulator};
use dialoguer::{
    console::{Style, StyledObject},
    theme::ColorfulTheme,
    Input, Select,
};

/// A ffxiv crafting tool
#[derive(Parser, Debug)]
#[clap(name = "crafty", bin_name = "crafty")]
struct Args {
    /// The player's job level
    #[clap(index = 1)]
    job_level: u32,
    /// The player's craftsmanship stat
    #[clap(index = 2)]
    craftsmanship: u32,
    /// The player's control stat
    #[clap(index = 3)]
    control: u32,
    /// The player's cp stat
    #[clap(index = 4)]
    cp: u32,
}

fn main() -> Result<()> {
    ctrlc::set_handler(|| {
        let term = dialoguer::console::Term::stdout();
        let _ = term.show_cursor();
    })?;

    let args = Args::parse();

    is_between(&args.job_level, 1, 90, "job level")?;
    is_between(&args.craftsmanship, 1, 5000, "craftsmanship")?;
    is_between(&args.control, 1, 5000, "control")?;
    is_between(&args.cp, 1, 700, "cp")?;

    let player = Player::new(args.job_level, args.craftsmanship, args.control, args.cp);
    println!("\n  player stats: {}", cyan(player.to_string().as_str()));

    let recipe_job_level: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("recipe level? (1-90)")
        .with_initial_text("90")
        .validate_with(|input: &u32| is_between(input, 1, 90, "recipe level"))
        .interact_text()?;

    let recipe_options = recipes_by_level(recipe_job_level);
    let recipe = prompt_selection("recipe?", recipe_options)?;

    let mut sim = Simulator::new(recipe, &player);

    let mut node = 0;
    loop {
        let state = &sim.tree.get_mut(node).unwrap().state;

        println!(
            "\n  step {:>2}: {}",
            state.step,
            cyan(state.to_string().as_str())
        );

        if state.is_terminating() {
            break;
        }

        let mut options = state.available_moves.clone();
        options.sort_by_key(|k| format!("{}", k));
        let action = *prompt_selection("action?:", &options)?;

        node = sim.execute_actions(node, vec![action]);
    }

    Ok(())
}

fn is_between(value: &u32, min: u32, max: u32, label: &str) -> Result<()> {
    if value >= &min && value <= &max {
        Ok(())
    } else {
        Err(anyhow!("{} should be between {} and {}", label, min, max))
    }
}

fn cyan(s: &str) -> StyledObject<&str> {
    let cyan = Style::new().green();
    cyan.apply_to(s)
}

fn prompt_selection<'a, T>(prompt: &str, items: &'a [T]) -> Result<&'a T>
where
    T: std::fmt::Display,
{
    if items.len() > 1 {
        let selected = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .items(items)
            .default(0)
            .max_length(8)
            .interact_opt()?
            .context("no item selected")?;
        Ok(&items[selected])
    } else {
        Ok(&items[0])
    }
}
