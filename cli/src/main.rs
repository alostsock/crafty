#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

use anyhow::{anyhow, Context, Error, Result};
use clap::Parser;
use crafty::{
    data, Action, CraftContext, CraftResult, CraftState, Player, Recipe, SearchOptions, Simulator,
};
use dialoguer::{
    console::{Style, StyledObject},
    theme::ColorfulTheme,
    Confirm, FuzzySelect, Input, Select,
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

    /// The maximum number of steps allowed
    #[clap(short = 's', long, default_value_t = 25_u8, display_order = 1000)]
    steps: u8,

    /// The number of craft simulations to run per search
    #[clap(short = 'i', long, default_value_t = 500_000_u32, display_order = 1101)]
    search_iterations: u32,

    /// The number of searches to run in parallel
    #[clap(short = 'p', long, default_value_t = 1_u16, display_order = 1102)]
    search_pool_size: u16,

    /// Search mode (stepwise or oneshot)
    #[clap(short = 'm', long, default_value_t = SearchMode::Stepwise, display_order = 1103)]
    search_mode: SearchMode,

    /// A positive integer to use for seeding RNG
    #[clap(long, display_order = 1200)]
    seed: Option<u32>,

    /// A constant used for search. The higher the weight, the more a node's potential max score
    /// is valued over its average score. A weight of 1.0 means only max scores will be used; 0.0
    /// means only average scores will be used.
    #[clap(short = 'w', default_value_t = 0.1_f32, display_order = 2000)]
    max_score_weighting_constant: f32,

    /// A constant used for search. Higher values will cause nodes with more uncertain scores to be
    /// explored.
    #[clap(short = 'c', default_value_t = 1.5_f32, display_order = 2000)]
    exploration_constant: f32,
}

#[derive(Debug, Clone, Copy)]
enum SearchMode {
    Stepwise,
    Oneshot,
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = format!("{self:?}").to_lowercase();
        write!(f, "{s}")
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
        dialoguer::console::Term::stdout().show_cursor().unwrap();
    })?;

    let args = Args::parse();
    validate_args(&args)?;

    let player = &Player::new(args.job_level, args.craftsmanship, args.control, args.cp);
    println!("\n  player stats: {}\n", green(player.to_string().as_str()));

    let recipe = prompt_recipe()?;

    let search_options = SearchOptions {
        iterations: args.search_iterations,
        rng_seed: args.seed,
        score_storage_threshold: Some(0.75),
        max_score_weighting_constant: Some(args.max_score_weighting_constant),
        exploration_constant: Some(args.exploration_constant),
    };

    let context = CraftContext::new(player, recipe, args.steps);
    let mut action_history: Vec<Action> = vec![];
    loop {
        let (state, result) = Simulator::simulate(&context, action_history.clone());
        match result {
            None => {
                print_state(&state);
            }
            Some(CraftResult::Finished(_)) => {
                println!("{}", green("\nThe craft is complete."),);
                print_state(&state);
                break;
            }
            _ => {
                println!("{}", red("\nThe craft has failed."));
                print_state(&state);
                break;
            }
        }

        let continue_manually = Confirm::new()
            .with_prompt("  continue manually?")
            .interact()?;

        if continue_manually {
            // Prompt for an action
            let mut actions = state.available_moves.to_vec();
            actions.sort_by_key(|k| format!("{k}"));
            let action = *prompt_selection("action?:", &actions, true)?;
            action_history.push(action);
        } else {
            print_info(&format!(
                "\n  attempting to find the best solution under {} steps...",
                args.steps
            ));

            let instant = time::Instant::now();

            // Run multiple simulations in parallel, and take the one with the max score
            let (actions, result_state) = (0..args.search_pool_size)
                .into_par_iter()
                .map(|_| match args.search_mode {
                    SearchMode::Stepwise => Simulator::search_stepwise(
                        &context,
                        action_history.clone(),
                        search_options,
                        None,
                    ),
                    SearchMode::Oneshot => {
                        Simulator::search_oneshot(&context, action_history.clone(), search_options)
                    }
                })
                .max_by(|(_, a), (_, b)| a.max_score.partial_cmp(&b.max_score).unwrap())
                .unwrap();

            let elapsed = instant.elapsed().as_secs_f64();
            print_info(&format!("  completed in {elapsed} seconds."));

            print_state(&result_state);

            let action_count = actions.len();
            print_info(&format!("\n  {action_count} actions taken:\n"));
            for action in actions {
                println!("{}", action.macro_text());
            }

            break;
        }
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn is_between<T: std::cmp::PartialOrd + std::fmt::Display>(
    value: T,
    min: T,
    max: T,
    label: &str,
) -> Result<()> {
    if value >= min && value <= max {
        Ok(())
    } else {
        Err(anyhow!("{} should range from {} and {}", label, min, max))
    }
}

fn validate_args(args: &Args) -> Result<()> {
    is_between(args.job_level, 1, 90, "job level")?;
    is_between(args.craftsmanship, 1, 5000, "craftsmanship")?;
    is_between(args.control, 1, 5000, "control")?;
    is_between(args.cp, 1, 700, "cp")?;
    is_between(args.search_iterations, 100, 10_000_000, "iteration count")?;
    is_between(args.search_pool_size, 1, 10_000, "search pool")?;
    is_between(args.steps, 5, 50, "max steps")?;
    is_between(
        args.max_score_weighting_constant,
        0.0,
        1.0,
        "max score weighting constant",
    )?;
    is_between(
        args.exploration_constant,
        0.0,
        1000.0,
        "exploration constant",
    )?;
    Ok(())
}

fn prompt_selection<'a, T>(prompt: &str, items: &'a [T], is_fuzzy: bool) -> Result<&'a T>
where
    T: std::fmt::Display,
{
    if items.len() > 1 {
        let selected = if is_fuzzy {
            FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt)
                .items(items)
                .default(0)
                .interact_opt()?
                .context("no item selected")?
        } else {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt)
                .items(items)
                .default(0)
                .max_length(5)
                .interact_opt()?
                .context("no item selected")?
        };
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
    let recipe = prompt_selection("recipe?", recipe_options, false)?;
    Ok(recipe)
}

fn print_state(state: &CraftState) {
    println!(
        "\n  step {:>2}: {}",
        state.step,
        green(state.to_string().as_str())
    );
}

fn print_info(info: &str) {
    println!("{}", cyan(info));
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
