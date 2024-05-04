use crafty::{
    Action, CraftContext, CraftOptions, CraftResult, CraftState as InternalCraftState, Player,
    Recipe, SearchOptions, Simulation,
};
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
struct CraftState {
    step: u8,
    step_max: u8,
    progress: u32,
    progress_target: u32,
    quality: u32,
    quality_target: u32,
    durability: i8,
    durability_max: i8,
    cp: u32,
    cp_max: u32,
    observe: bool,
    next_combo_action: Option<Action>,
    buffs: Buffs,
    available_moves: Vec<Action>,
}

impl CraftState {
    fn from_internal(state: &InternalCraftState) -> Self {
        Self {
            step: state.step,
            step_max: state.context.step_max,
            progress: state.progress,
            progress_target: state.context.progress_target,
            quality: state.quality,
            quality_target: state.context.quality_target,
            durability: state.durability,
            durability_max: state.context.durability_max,
            cp: state.cp,
            cp_max: state.context.cp_max,
            observe: state.observe,
            next_combo_action: state.next_combo_action,
            buffs: state.buffs.clone(),
            available_moves: state.available_moves.to_vec(),
        }
    }
}

#[derive(Serialize, TsType)]
enum CompletionReason {
    Finished,
    DurabilityFailure,
    MaxStepsFailure,
    InvalidActionFailure,
}

impl CompletionReason {
    fn from_craft_result(result: Option<CraftResult>) -> Option<CompletionReason> {
        match result {
            Some(CraftResult::Finished(_)) => Some(CompletionReason::Finished),
            Some(CraftResult::DurabilityFailure) => Some(CompletionReason::DurabilityFailure),
            Some(CraftResult::MaxStepsFailure) => Some(CompletionReason::MaxStepsFailure),
            Some(CraftResult::InvalidActionFailure) => Some(CompletionReason::InvalidActionFailure),
            _ => None,
        }
    }
}

#[derive(Serialize, TsType)]
struct SimulatorResult {
    craft_state: CraftState,
    completion_reason: Option<CompletionReason>,
    score: f32,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPE_SIMULATE_ACTIONS: &'static str = r#"
export function simulateActions(
    recipe: Recipe,
    player: Player,
    actions: Action[],
    craft_options: CraftOptions,
): SimulatorResult;
"#;

#[wasm_bindgen(js_name = simulateActions, skip_typescript)]
pub fn simulate_actions(
    recipe: JsValue,
    player: JsValue,
    actions: JsValue,
    craft_options: JsValue,
) -> JsValue {
    console_error_panic_hook::set_once();

    let recipe: Recipe = from_js_value(recipe).unwrap();
    let player: Player = from_js_value(player).unwrap();
    let actions_str: Vec<String> = from_js_value(actions).unwrap();
    let actions: Vec<Action> = actions_str
        .iter()
        .map(|a| Action::from_str(a).unwrap())
        .collect();
    let craft_options: CraftOptions = from_js_value(craft_options).unwrap();

    let context = CraftContext::new(&player, &recipe, craft_options);
    let (end_state, result) = Simulation::simulate(&context, actions);

    let sim_result = SimulatorResult {
        craft_state: CraftState::from_internal(&end_state),
        completion_reason: CompletionReason::from_craft_result(result),
        score: end_state.score(),
    };

    to_js_value(&sim_result).unwrap().unchecked_into()
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPE_SEARCH_STEPWISE: &'static str = r#"
export function searchStepwise(
    recipe: Recipe,
    player: Player,
    action_history: Action[],
    craft_options: CraftOptions,
    search_options: SearchOptions,
    action_callback: (action: Action) => void,
): Action[];
"#;

#[wasm_bindgen(js_name = searchStepwise, skip_typescript)]
pub fn search_stepwise(
    recipe: JsValue,
    player: JsValue,
    action_history: JsValue,
    craft_options: JsValue,
    search_options: JsValue,
    action_callback: js_sys::Function,
) -> JsValue {
    console_error_panic_hook::set_once();

    let recipe: Recipe = from_js_value(recipe).unwrap();
    let player: Player = from_js_value(player).unwrap();
    let action_history_str: Vec<String> = from_js_value(action_history).unwrap();
    let action_history: Vec<Action> = action_history_str
        .iter()
        .map(|a| Action::from_str(a).unwrap())
        .collect();
    let craft_options: CraftOptions = from_js_value(craft_options).unwrap();
    let search_options: SearchOptions = from_js_value(search_options).unwrap();

    let callback = |action: Action| {
        let null = JsValue::null();
        let action_str = JsValue::from(action.name());
        action_callback.call1(&null, &action_str).unwrap();
    };

    let (actions, _) = Simulation::search_stepwise(
        &CraftContext::new(&player, &recipe, craft_options),
        action_history,
        search_options,
        Some(&callback),
    );

    let actions_str: Vec<&'static str> = actions.iter().map(|a| a.name()).collect();
    to_js_value(&actions_str).unwrap().unchecked_into()
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPE_GENERATE_MACRO_TEXT: &'static str = r#"
export function generateMacroText(actions: Action[]): string[];
"#;

#[wasm_bindgen(js_name = generateMacroText, skip_typescript)]
pub fn generate_macro_text(actions: JsValue) -> JsValue {
    let actions_str: Vec<String> = from_js_value(actions).unwrap();
    let macro_text: Vec<String> = actions_str
        .iter()
        .map(|a| Action::from_str(a).unwrap().macro_text())
        .collect();

    to_js_value(&macro_text).unwrap().unchecked_into()
}
