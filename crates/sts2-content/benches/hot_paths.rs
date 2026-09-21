//! The engine costs a search client pays for: run generation, stepping,
//! snapshot and restore, and the two canonical keys.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use sts2_core::UnlockPresetManifest;
use sts2_engine::{Action, Simulator};

const SEED: &str = "NLD6VZXP94";

fn fresh_run() -> Simulator {
    let preset = UnlockPresetManifest::pinned().unwrap();
    let character = "CHARACTER.IRONCLAD".parse().unwrap();
    sts2_content::standard_run_on_preset_at(SEED, &character, &preset, 0).unwrap()
}

/// The first legal action, preferring a non-empty card pick so a skippable
/// reward cannot re-offer itself.
fn next_action(simulator: &Simulator) -> Action {
    simulator
        .legal_actions()
        .iter()
        .find(|action| matches!(action, Action::ChooseCards { cards, .. } if !cards.is_empty()))
        .or_else(|| simulator.legal_actions().first())
        .cloned()
        .expect("a live run offers an action")
}

/// A simulator standing mid-run: past Neow, into the first fights.
fn walked_run(steps: usize) -> Simulator {
    let mut simulator = fresh_run();
    for _ in 0..steps {
        if simulator.state().terminal.is_some() {
            break;
        }
        let action = next_action(&simulator);
        simulator.step(action).unwrap();
    }
    simulator
}

fn hot_paths(criterion: &mut Criterion) {
    criterion.bench_function("generate_run", |bencher| {
        bencher.iter(|| black_box(fresh_run()));
    });

    let simulator = walked_run(30);
    let snapshot = simulator.snapshot();

    criterion.bench_function("snapshot", |bencher| {
        bencher.iter(|| black_box(simulator.snapshot()));
    });

    criterion.bench_function("restore", |bencher| {
        let mut target = simulator.clone();
        bencher.iter(|| target.restore(black_box(&snapshot)).unwrap());
    });

    criterion.bench_function("state_key", |bencher| {
        bencher.iter(|| black_box(simulator.state_key().unwrap()));
    });

    criterion.bench_function("observation_key", |bencher| {
        bencher.iter(|| black_box(simulator.observation_key().unwrap()));
    });

    criterion.bench_function("agent_observation", |bencher| {
        bencher.iter(|| black_box(simulator.agent_observation()));
    });

    criterion.bench_function("legal_actions", |bencher| {
        bencher.iter(|| black_box(simulator.legal_actions().len()));
    });

    // Sixty-four steps from a fixed mid-run state, restore included: the
    // shape of one search rollout segment.
    criterion.bench_function("restore_and_step_64", |bencher| {
        let mut target = simulator.clone();
        bencher.iter(|| {
            target.restore(&snapshot).unwrap();
            for _ in 0..64 {
                if target.state().terminal.is_some() {
                    break;
                }
                let action = next_action(&target);
                target.step(action).unwrap();
            }
        });
    });

    // The same segment on the quiet step: what a search rollout actually pays.
    criterion.bench_function("restore_and_step_quietly_64", |bencher| {
        let mut target = simulator.clone();
        bencher.iter(|| {
            target.restore(&snapshot).unwrap();
            for _ in 0..64 {
                if target.state().terminal.is_some() {
                    break;
                }
                let action = next_action(&target);
                target.step_quietly(&action).unwrap();
            }
        });
    });

    // The same costs from deep in a run: an aged step should cost what a
    // young one costs. Step 100 is late in this walk's life, with room left
    // to step.
    let aged = walked_run(100);
    let aged_snapshot = aged.snapshot();

    criterion.bench_function("aged_snapshot", |bencher| {
        bencher.iter(|| black_box(aged.snapshot()));
    });

    criterion.bench_function("aged_state_key", |bencher| {
        bencher.iter(|| black_box(aged.state_key().unwrap()));
    });

    criterion.bench_function("aged_observation_key", |bencher| {
        bencher.iter(|| black_box(aged.observation_key().unwrap()));
    });

    criterion.bench_function("aged_agent_observation", |bencher| {
        bencher.iter(|| black_box(aged.agent_observation()));
    });

    criterion.bench_function("aged_restore_and_step_quietly_64", |bencher| {
        let mut target = aged.clone();
        bencher.iter(|| {
            target.restore(&aged_snapshot).unwrap();
            for _ in 0..64 {
                if target.state().terminal.is_some() {
                    break;
                }
                let action = next_action(&target);
                target.step_quietly(&action).unwrap();
            }
        });
    });
}

criterion_group!(benches, hot_paths);
criterion_main!(benches);
