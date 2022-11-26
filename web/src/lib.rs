use crafty::{CraftResult, CraftState, Player, Recipe, Simulator};
use serde::Serialize;
use serde_wasm_bindgen::{from_value as from_js_value, to_value as to_js_value};
use std::str::FromStr;
use ts_type::TsType;
use wasm_bindgen::{prelude::*, JsCast};

// some of these imports are only present to generate Typescript types
#[allow(unused_imports)]
use crafty::{Action, Buffs};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Recipe[]")]
    pub type Recipes;
}

#[wasm_bindgen(js_name = recipesByJobLevel)]
pub fn recipes_by_job_level(player_job_level: u32) -> Recipes {
    let recipes = crafty::data::recipes(player_job_level);

    to_js_value(&recipes).unwrap().unchecked_into()
}

#[derive(Serialize, TsType)]
enum CompletionReason {
    Finished,
    DurabilityFailure,
    MaxStepsFailure,
    InvalidActionFailure,
}

#[derive(Serialize, TsType)]
struct SimulatorResult {
    craft_state: CraftState,
    completion_reason: Option<CompletionReason>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPE_SIMULATE_ACTIONS: &'static str = r#"
export function simulateActions(
    recipe: Recipe,
    player: Player,
    actions: Action[]
): SimulatorResult;
"#;

#[wasm_bindgen(js_name = simulateActions, skip_typescript)]
pub fn simulate_actions(recipe: JsValue, player: JsValue, actions: JsValue) -> JsValue {
    let recipe: Recipe = from_js_value(recipe).unwrap();
    let player: Player = from_js_value(player).unwrap();
    let actions_str: Vec<String> = from_js_value(actions).unwrap();
    let actions: Vec<Action> = actions_str
        .iter()
        .map(|a| Action::from_str(a).unwrap())
        .collect();

    let start_state = CraftState::new(&player, &recipe, 50);
    let (end_state, result) = Simulator::simulate(&start_state, actions);

    let sim_result = SimulatorResult {
        craft_state: end_state,
        completion_reason: match result {
            Some(CraftResult::Finished(_)) => Some(CompletionReason::Finished),
            Some(CraftResult::DurabilityFailure) => Some(CompletionReason::DurabilityFailure),
            Some(CraftResult::MaxStepsFailure) => Some(CompletionReason::MaxStepsFailure),
            Some(CraftResult::InvalidActionFailure) => Some(CompletionReason::InvalidActionFailure),
            _ => None,
        },
    };

    to_js_value(&sim_result).unwrap().unchecked_into()
}
