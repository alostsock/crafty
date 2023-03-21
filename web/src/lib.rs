use crafty::{Action, CraftResult, CraftState, Player, Recipe, SearchOptions, Simulator};
use serde::Serialize;
use serde_wasm_bindgen::{from_value as from_js_value, to_value as to_js_value};
use std::str::FromStr;
use ts_type::TsType;
use wasm_bindgen::{prelude::*, JsCast};

// only present to generate Typescript types
#[allow(unused_imports)]
use crafty::Buffs;

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

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPE_SEARCH_STEPWISE: &'static str = r#"
export function searchStepwise(
    recipe: Recipe,
    player: Player,
    action_history: Action[],
    options: SearchOptions,
    action_callback: (action: Action) => void,
): Action[];
"#;

#[wasm_bindgen(js_name = searchStepwise, skip_typescript)]
pub fn search_stepwise(
    recipe: JsValue,
    player: JsValue,
    action_history: JsValue,
    options: JsValue,
    action_callback: js_sys::Function,
) -> JsValue {
    let recipe: Recipe = from_js_value(recipe).unwrap();
    let player: Player = from_js_value(player).unwrap();
    let action_history_str: Vec<String> = from_js_value(action_history).unwrap();
    let action_history: Vec<Action> = action_history_str
        .iter()
        .map(|a| Action::from_str(a).unwrap())
        .collect();
    let options: SearchOptions = from_js_value(options).unwrap();

    let start_state = CraftState::new(&player, &recipe, 50);

    let callback = |action: Action| {
        let null = JsValue::null();
        let action_str = JsValue::from(action.to_string());
        action_callback.call1(&null, &action_str).unwrap();
    };

    let (actions, _) =
        Simulator::search_stepwise(&start_state, action_history, options, Some(&callback));

    to_js_value(&actions).unwrap().unchecked_into()
}
