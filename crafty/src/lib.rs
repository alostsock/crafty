#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::enum_glob_use)]

mod action;
mod action_set;
mod craft_context;
mod craft_state;
pub mod data;
mod player;
mod simulator;
mod tree;

pub use action::Action;
use action_set::ActionSet;
pub use craft_context::{CraftContext, CraftOptions};
pub use craft_state::{Buffs, CraftResult, CraftState};
pub use player::Player;
pub use recipe::Recipe;
pub use simulator::{SearchOptions, Simulator};
