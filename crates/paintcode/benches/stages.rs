//! cargo bench -p paintcode --features profile --bench stages
//!
//! Where a paint's time goes: the example flowers painted on one thread, each stage of
//! src/profile.rs timed, per paint. The rest is the conductor between stages: what the wall clock
//! saw that no stage did.
use std::path::Path;
use std::time::Instant;

use paintcode::profile::{self, Spent};
use paintcode::{Flower, RenderOpts, flower_to_canvas};

const CASES: [(&str, &str, i64); 3] = [
    ("kiku", "examples/kiku.flower", 1),
    ("kiku_ss2", "examples/kiku.flower", 2),
    ("brush_sheet", "examples/brush_sheet.flower", 1),
];

/// Paints per case, after one to warm up.
const PAINTS: u32 = 5;

fn load(file: &str) -> Flower {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(file);
    paintcode::decode_flower(&std::fs::read(path).unwrap()).unwrap()
}

fn main() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    for (name, file, supersample) in CASES {
        let flower = load(file);
        let opts = RenderOpts::new(1.0, supersample);
        pool.install(|| flower_to_canvas(&flower, &opts).unwrap());
        profile::take();
        let start = Instant::now();
        for _ in 0..PAINTS {
            pool.install(|| flower_to_canvas(&flower, &opts).unwrap());
        }
        let wall = start.elapsed().as_nanos() as f64 / PAINTS as f64;
        report(name, wall, &profile::take());
    }
}

fn report(name: &str, wall: f64, spent: &[Spent]) {
    let ms = |nanos: f64| nanos / 1e6;
    println!("{name}: one thread, per paint, mean of {PAINTS}");
    println!(
        "  {:<11} {:>9} {:>6} {:>8} {:>10}",
        "stage", "ms", "share", "calls", "us/call"
    );
    let mut staged = 0.0;
    for s in spent {
        let nanos = s.nanos as f64 / PAINTS as f64;
        let calls = s.calls as f64 / PAINTS as f64;
        staged += nanos;
        let per = if calls > 0.0 {
            nanos / calls / 1e3
        } else {
            0.0
        };
        println!(
            "  {:<11} {:>9.2} {:>5.1}% {:>8.0} {:>10.2}",
            s.stage.name(),
            ms(nanos),
            100.0 * nanos / wall,
            calls,
            per
        );
    }
    let rest = wall - staged;
    println!(
        "  {:<11} {:>9.2} {:>5.1}%",
        "rest",
        ms(rest),
        100.0 * rest / wall
    );
    println!("  {:<11} {:>9.2}", "wall", ms(wall));
    println!();
}
