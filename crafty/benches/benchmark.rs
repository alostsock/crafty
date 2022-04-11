use crafty::{Action, Player, Recipe, SearchOptions, Simulator};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use std::time::Duration;
use Action::*;

fn setup_sim(rng_seed: Option<u64>) -> Simulator {
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
    let options = SearchOptions {
        iterations: 50_000,
        max_steps: 15,
        rng_seed,
        score_storage_threshold: None,
    };
    Simulator::new(&recipe, &player, options)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("rotation", |b| {
        b.iter_batched(
            || -> Simulator { setup_sim(None) },
            |mut sim| sim.execute_actions(black_box(0), black_box(ROTATION_1.to_vec())),
            BatchSize::SmallInput,
        )
    });

    let mut group = c.benchmark_group("search");
    group
        .warm_up_time(Duration::new(5, 0))
        .measurement_time(Duration::new(30, 0));
    for seed in 0..3_u64 {
        group.bench_function(seed.to_string().as_str(), |b| {
            b.iter_batched(
                || -> Simulator { setup_sim(Some(seed)) },
                |mut sim| {
                    sim.search(black_box(0));
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(997, Output::Flamegraph(None)));
    targets = criterion_benchmark
);
criterion_main!(benches);

const ROTATION_1: &[Action] = &[
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
