use anyhow::{anyhow, Context, Result};
use clap::Parser;
use crafty::{data, CraftResult, CraftState, Player, Simulator};
use dialoguer::{
    console::{Style, StyledObject},
    theme::ColorfulTheme,
    Confirm, Input, Select,
};
use std::time;

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
    /// When searching, limits the number of random simulations run
    #[clap(short, long, default_value_t = 100_000_u32)]
    iterations: u32,
    /// When searching, limits the maximum number of steps allowed
    #[clap(short, long, default_value_t = 15_u8)]
    steps: u8,
    /// When searching, sets a positive integer to use as a seed for RNG
    #[clap(long)]
    seed: Option<u64>,
}

fn main() -> Result<()> {
    ctrlc::set_handler(|| {
        let term = dialoguer::console::Term::stdout();
        let _ = term.show_cursor();
    })?;

    let args = Args::parse();

    is_between(args.job_level, 1, 90, "job level")?;
    is_between(args.craftsmanship, 1, 5000, "craftsmanship")?;
    is_between(args.control, 1, 5000, "control")?;
    is_between(args.cp, 1, 700, "cp")?;
    is_between(args.iterations, 100, 10_000_000, "iterations")?;
    is_between(u32::from(args.steps), 5, 50, "steps")?;

    let player = Player::new(args.job_level, args.craftsmanship, args.control, args.cp);
    println!("\n  player stats: {}\n", green(player.to_string().as_str()));

    let recipe_job_level: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("recipe level? (1-90)")
        .with_initial_text("90")
        .validate_with(|input: &u32| is_between(*input, 1, 90, "recipe level"))
        .interact_text()?;

    let recipe_options = data::recipes(recipe_job_level);
    let recipe = prompt_selection("recipe?", recipe_options)?;

    let mut sim = Simulator::new(recipe, &player, args.iterations, args.steps, args.seed);
    let mut current_index = 0;
    loop {
        let state = &sim.tree.get_mut(current_index).state;

        print_state(state);

        let manual = Confirm::new()
            .with_prompt("  continue manually?")
            .interact()?;

        if manual {
            // manually pick an action
            let mut actions = state.available_moves.clone();
            actions.sort_by_key(|k| format!("{}", k));
            let action = *prompt_selection("action?:", &actions)?;
            match sim.execute_actions(current_index, vec![action]) {
                Ok(next_index) => current_index = next_index,
                Err(CraftResult::Finished(_)) => {
                    println!("{}", green("\nThe craft is complete"),);
                    break;
                }
                Err(CraftResult::Failed) => {
                    println!("{}", red("\nThe craft has failed."));
                    break;
                }
            }
        } else {
            print_info(format!(
                "\n  attempting to find the best solution under {} steps...",
                args.steps
            ));

            let instant = time::Instant::now();
            let (actions, end_state) = sim.search(current_index).solution();
            let elapsed = instant.elapsed().as_secs_f64();

            print_info(format!("  completed in {elapsed} seconds."));

            print_state(&end_state);

            print_info("\n  actions taken:".to_string());
            for action in actions {
                println!("  {action:?}");
            }

            print_info(format!(
                "\nmax score: {}\nest. memory used: {} bytes\nvisits: {}\ndead ends: {}",
                end_state.max_score,
                sim.tree.nodes.capacity() * std::mem::size_of_val(&sim.tree.nodes[0]),
                sim.tree.nodes.len(),
                sim.dead_ends_selected
            ));
            break;
        }
    }

    Ok(())
}

fn is_between(value: u32, min: u32, max: u32, label: &str) -> Result<()> {
    if value >= min && value <= max {
        Ok(())
    } else {
        Err(anyhow!("{} should be between {} and {}", label, min, max))
    }
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
            .max_length(5)
            .interact_opt()?
            .context("no item selected")?;
        Ok(&items[selected])
    } else {
        Ok(&items[0])
    }
}

fn print_state(state: &CraftState) {
    println!(
        "\n  step {:>2}: {}",
        state.step,
        green(state.to_string().as_str())
    );
}

fn print_info(info: String) {
    println!("{}", cyan(info.as_str()));
}

fn cyan(s: &str) -> StyledObject<&str> {
    Style::new().cyan().apply_to(s)
}

fn green(s: &str) -> StyledObject<&str> {
    Style::new().green().apply_to(s)
}

fn red(s: &str) -> StyledObject<&str> {
    Style::new().red().apply_to(s)
}
