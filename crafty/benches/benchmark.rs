use crafty::{Action, CraftContext, CraftOptions, Player, Recipe, SearchOptions, Simulator};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use std::time::Duration;
use Action::*;

fn setup_sim_1(rng_seed: Option<u32>) -> (CraftContext, SearchOptions) {
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
    let craft_options = CraftOptions {
        max_steps: 15,
        ..Default::default()
    };
    let context = CraftContext::new(&player, &recipe, craft_options);
    let options = SearchOptions {
        iterations: 50_000,
        rng_seed,
        ..Default::default()
    };
    (context, options)
}

fn setup_sim_2(rng_seed: Option<u32>) -> (CraftContext, SearchOptions) {
    let recipe = Recipe {
        recipe_level: 110,
        job_level: 50,
        stars: 4,
        progress: 480,
        quality: 2900,
        durability: 80,
        progress_div: 50,
        progress_mod: 80,
        quality_div: 30,
        quality_mod: 70,
        is_expert: false,
        conditions_flag: 15,
    };
    let player = Player::new(50, 500, 500, 300);
    let craft_options = CraftOptions {
        max_steps: 18,
        ..Default::default()
    };
    let context = CraftContext::new(&player, &recipe, craft_options);
    let options = SearchOptions {
        rng_seed,
        ..Default::default()
    };
    (context, options)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("rotation", |b| {
        b.iter_batched(
            || setup_sim_1(None),
            |(context, _)| {
                Simulator::simulate(&context, black_box(ROTATION_1.to_vec()));
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("search_exhaustive", |b| {
        b.iter_batched(
            || setup_sim_2(Some(123)),
            |(context, _)| {
                Simulator::search_exhaustive(&context, black_box(vec![]));
            },
            BatchSize::PerIteration,
        )
    });

    let mut group = c.benchmark_group("search_oneshot");
    group
        .warm_up_time(Duration::new(5, 0))
        .measurement_time(Duration::new(30, 0));
    for seed in 0..5_u32 {
        group.bench_function(seed.to_string().as_str(), |b| {
            b.iter_batched(
                || setup_sim_1(Some(seed)),
                |(context, options)| {
                    Simulator::search_oneshot(&context, black_box(vec![]), options);
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
    GroundworkTraited,
    GroundworkTraited,
    GroundworkTraited,
];
