use serde_wasm_bindgen::{from_value as from_js_value, to_value as to_js_value};
use std::str::FromStr;
use wasm_bindgen::{prelude::*, JsCast};

// some of these imports are only present to generate Typescript types
#[allow(unused_imports)]
use crafty::{Action, Buffs, CraftState, Player, Recipe, SearchOptions, Simulator};

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPE_SIMULATE_ACTIONS: &'static str = r#"
export function simulateActions(
    recipe: Recipe,
    player: Player,
    options: SearchOptions,
    actions: Action[]
): CraftState;
"#;

#[wasm_bindgen(js_name = simulateActions, skip_typescript)]
pub fn simulate_actions(
    recipe: JsValue,
    player: JsValue,
    options: JsValue,
    actions: JsValue,
) -> JsValue {
    let recipe: Recipe = from_js_value(recipe).unwrap();
    let player: Player = from_js_value(player).unwrap();
    let options: SearchOptions = from_js_value(options).unwrap();
    let actions_str: Vec<String> = from_js_value(actions).unwrap();
    let actions: Vec<Action> = actions_str
        .iter()
        .map(|a| Action::from_str(a).unwrap())
        .collect();

    let mut sim = Simulator::new(&recipe, &player, options);
    let (result_index, _) = sim.execute_actions(0, actions);
    let result_node = sim.tree.get(result_index);

    to_js_value(&result_node.state).unwrap().unchecked_into()
}
