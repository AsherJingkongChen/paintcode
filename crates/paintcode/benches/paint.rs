//! cargo bench -p paintcode [-- FILTER]
//!
//! paint: the example flowers on one thread and on every core. Throughput is in dabs, so flowers
//! of different sizes compare, and criterion compares each run with the one before it.
//!
//! loop: the rest of an agent's round with kiku, on one thread, step by step: the flower's JSON
//! read and written, and the canvas encoded as a PNG with and without its flower on it, and
//! decoded. Drying is timed with the other stages in benches/stages.rs.
use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

use criterion::{
    BenchmarkId, Criterion, SamplingMode, Throughput, criterion_group, criterion_main,
};
use paintcode::{
    Flower, RenderOpts, decode_canvas, decode_flower, encode_canvas, encode_flower,
    flower_on_canvas, flower_to_canvas, flower_to_coat,
};

const CASES: [(&str, &str, i64); 3] = [
    ("kiku", "examples/kiku.flower", 1),
    ("kiku_ss2", "examples/kiku.flower", 2),
    ("brush_sheet", "examples/brush_sheet.flower", 1),
];

/// Samples per case; a paint takes up to seconds, so each sample is one paint.
const SAMPLES: usize = 10;

fn load(file: &str) -> Flower {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(file);
    paintcode::decode_flower(&std::fs::read(path).unwrap()).unwrap()
}

fn threads() -> Vec<usize> {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut threads = vec![1, cores];
    threads.dedup();
    threads
}

fn pool(n: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(n)
        .build()
        .unwrap()
}

fn paint(c: &mut Criterion) {
    let mut group = c.benchmark_group("paint");
    group.sampling_mode(SamplingMode::Flat).sample_size(SAMPLES);
    for (name, file, supersample) in CASES {
        let flower = load(file);
        let opts = RenderOpts::new(1.0, supersample);
        let dabs = flower_to_coat(&flower, &opts).unwrap().len();
        group.throughput(Throughput::Elements(dabs as u64));
        for n in threads() {
            let pool = pool(n);
            let once =
                || pool.install(|| black_box(flower_to_canvas(black_box(&flower), &opts).unwrap()));
            // room for every sample and a little more, measured on this machine
            let t = Instant::now();
            once();
            group.measurement_time(
                t.elapsed()
                    .mul_f64(SAMPLES as f64 * 1.2)
                    .max(Duration::from_secs(1)),
            );
            group.bench_function(BenchmarkId::new(name, n), |b| b.iter(once));
        }
    }
    group.finish();
}

fn round(c: &mut Criterion) {
    let mut group = c.benchmark_group("loop");
    let text =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kiku.flower"))
            .unwrap();
    let flower = decode_flower(&text).unwrap();
    let pool = pool(1);
    for supersample in [1, 2] {
        let opts = RenderOpts::new(1.0, supersample);
        let canvas = flower_to_canvas(&flower, &opts).unwrap();
        let on = flower_on_canvas(&flower, canvas.clone());
        let png = encode_canvas(&on);
        let ss = format!("ss{supersample}");
        if supersample == 1 {
            group.bench_function("parse", |b| {
                b.iter(|| decode_flower(black_box(&text)).unwrap())
            });
            group.bench_function("save_flower", |b| {
                b.iter(|| encode_flower(black_box(&flower)))
            });
        }
        group.bench_function(BenchmarkId::new("png", &ss), |b| {
            b.iter(|| pool.install(|| encode_canvas(black_box(&canvas))))
        });
        group.bench_function(BenchmarkId::new("png_flower", &ss), |b| {
            b.iter(|| pool.install(|| encode_canvas(black_box(&on))))
        });
        group.bench_function(BenchmarkId::new("decode_png", &ss), |b| {
            b.iter(|| pool.install(|| decode_canvas(black_box(&png)).unwrap()))
        });
    }
    group.finish();
}

criterion_group!(benches, paint, round);
criterion_main!(benches);
