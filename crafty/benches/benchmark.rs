use crafty::{action::Action, player::Player, simulator::Simulator};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use recipe::Recipe;
use Action::*;

fn setup_sim() -> Simulator {
    let recipe = Recipe {
        recipe_level: 560,
        job_level: 90,
        stars: 0,
        progress: 3500,
        quality: 7200,
        durability: 80,
        progress_div: 130,
        progress_mod: 90,
        quality_div: 115,
        quality_mod: 80,
        is_expert: false,
        conditions_flag: 15,
    };
    let player = Player::new(90, 3304, 3374, 575);
    Simulator::new(&recipe, &player)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("basic rotation", |b| {
        let sim = &mut setup_sim();
        b.iter(|| {
            sim.execute_actions(
                black_box(0),
                black_box(vec![
                    Reflect,
                    Manipulation,
                    PreparatoryTouch,
                    WasteNotII,
                    PreparatoryTouch,
                    Innovation,
                    PreparatoryTouch,
                    PreparatoryTouch,
                    GreatStrides,
                    ByregotsBlessing,
                    Veneration,
                    Groundwork,
                    Groundwork,
                    Groundwork,
                ]),
            )
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
);
criterion_main!(benches);
