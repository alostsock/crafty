use crafty::{Action, Player, Recipe, Simulator};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use pprof::criterion::{Output, PProfProfiler};
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
    Simulator::new(&recipe, &player, 50_000, 15)
}

const ACTIONS: &[Action] = &[
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
];

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("basic rotation", |b| {
        b.iter_batched(
            || -> Simulator { setup_sim() },
            |mut sim| sim.execute_actions(black_box(0), black_box(ACTIONS.to_vec())),
            BatchSize::SmallInput,
        )
    });

    c.bench_function("basic exploration", |b| {
        b.iter_batched(
            || -> Simulator { setup_sim() },
            |mut sim| {
                sim.search(black_box(0));
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(997, Output::Flamegraph(None)));
    targets = criterion_benchmark
);
criterion_main!(benches);
