use anyhow::{anyhow, Context, Error, Result};
use clap::Parser;
use crafty::{data, Action, CraftResult, CraftState, Player, Recipe, SearchOptions, Simulator};
use dialoguer::{
    console::{Style, StyledObject},
    theme::ColorfulTheme,
    Confirm, Input, Select,
};
use rayon::prelude::*;
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
    /// Search mode (stepwise/oneshot)
    #[clap(short, long, default_value_t = SearchMode::Stepwise)]
    mode: SearchMode,
    /// The number craft simulations to run per search
    #[clap(short = 'i', long, default_value_t = 500_000_u32)]
    search_iterations: u32,
    /// The number of searches to run in parallel
    #[clap(short = 'p', long, default_value_t = 1_u16)]
    search_pool: u16,
    /// The maximum number of steps allowed
    #[clap(short, long, default_value_t = 30_u8)]
    steps: u8,
    /// A positive integer for seeding RNG
    #[clap(long)]
    seed: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
enum SearchMode {
    Stepwise,
    Oneshot,
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = format!("{:?}", self).to_lowercase();
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for SearchMode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "stepwise" => Ok(Self::Stepwise),
            "oneshot" => Ok(Self::Oneshot),
            _ => Err(anyhow!("expected \"stepwise\" or \"oneshot\"")),
        }
    }
}

fn main() -> Result<()> {
    ctrlc::set_handler(|| {
        let term = dialoguer::console::Term::stdout();
        let _ = term.show_cursor();
    })?;

    let args = Args::parse();
    validate_args(&args)?;

    let player = &Player::new(args.job_level, args.craftsmanship, args.control, args.cp);
    println!("\n  player stats: {}\n", green(player.to_string().as_str()));

    let recipe = prompt_recipe()?;

    let search_options = SearchOptions {
        iterations: args.search_iterations,
        max_steps: args.steps,
        rng_seed: args.seed,
        score_storage_threshold: Some(0.75),
    };

    let mut sim = Simulator::new(recipe, player, search_options);
    let mut action_history: Vec<Action> = vec![];
    let mut current_index = 0;
    loop {
        let state = &sim.tree.get_mut(current_index).state;
        print_state(state);

        let manual_action = Confirm::new()
            .with_prompt("  continue manually?")
            .interact()?;

        if manual_action {
            let mut actions = state.available_moves.clone();
            actions.sort_by_key(|k| format!("{}", k));
            let action = *prompt_selection("action?:", &actions)?;
            action_history.push(action);
            let (next_index, result) = sim.execute_actions(current_index, vec![action]);
            match result {
                None => current_index = next_index,
                Some(CraftResult::Finished(_)) => {
                    println!("{}", green("\nThe craft is complete."),);
                    print_state(&sim.tree.get(next_index).state);
                    break;
                }
                Some(CraftResult::Failed) => {
                    println!("{}", red("\nThe craft has failed."));
                    print_state(&sim.tree.get(next_index).state);
                    break;
                }
            }
        } else {
            print_info(format!(
                "\n  attempting to find the best solution under {} steps...",
                args.steps
            ));

            let instant = time::Instant::now();

            // Run multiple simulations in parallel, and take the one with the max score
            let (actions, result_state) = (0..args.search_pool)
                .into_par_iter()
                .map(|_| match args.mode {
                    SearchMode::Stepwise => Simulator::search_stepwise(
                        state.clone(),
                        action_history.clone(),
                        search_options,
                    ),
                    SearchMode::Oneshot => Simulator::search_oneshot(
                        state.clone(),
                        action_history.clone(),
                        search_options,
                    ),
                })
                .max_by(|(_, a), (_, b)| a.max_score.partial_cmp(&b.max_score).unwrap())
                .unwrap();

            let elapsed = instant.elapsed().as_secs_f64();
            print_info(format!("  completed in {elapsed} seconds."));

            print_state(&result_state);
            let action_count = actions.len();
            print_info(format!("\n  {action_count} actions taken:"));
            for action in actions {
                println!("  {action:?}");
            }

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

fn validate_args(args: &Args) -> Result<()> {
    is_between(args.job_level, 1, 90, "job level")?;
    is_between(args.craftsmanship, 1, 5000, "craftsmanship")?;
    is_between(args.control, 1, 5000, "control")?;
    is_between(args.cp, 1, 700, "cp")?;
    is_between(args.search_iterations, 100, 10_000_000, "iteration count")?;
    is_between(u32::from(args.search_pool), 1, 10_000, "search pool")?;
    is_between(u32::from(args.steps), 5, 50, "max steps")?;
    Ok(())
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

fn prompt_recipe() -> Result<&'static Recipe> {
    let recipe_job_level: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("recipe level? (1-90)")
        .with_initial_text("90")
        .validate_with(|input: &u32| is_between(*input, 1, 90, "recipe level"))
        .interact_text()?;

    let recipe_options = data::recipes(recipe_job_level);
    let recipe = prompt_selection("recipe?", recipe_options)?;
    Ok(recipe)
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
