//! The fast path must paint the same bits as the reference path: whole 3-sigma squares, one
//! thread. Any speed-up that changes a pixel fails here.

use crate::render::Spans;
use crate::{Flower, RenderOpts, paint_flower};

fn flower(name: &str) -> Flower {
    super::load(&format!("examples/{name}"))
}

fn reference(s: &Flower) -> [Vec<f32>; 3] {
    let mut o = RenderOpts::default();
    o.petal.spans = Spans::Square;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    pool.install(|| paint_flower(s, &o).unwrap().planes)
}

#[test]
fn fast_path_is_bit_identical_on_the_brush_sheet() {
    let s = flower("brush_sheet.flower");
    let fast = paint_flower(&s, &RenderOpts::default()).unwrap().planes;
    assert!(fast == reference(&s), "the fast path changed pixels");
}

#[test]
fn chunking_and_snapshots_do_not_change_pixels() {
    // a tiny budget forces every petal into many chunks, and pickup petals onto a snapshot
    let s = flower("brush_sheet.flower");
    let mut o = RenderOpts::default();
    o.petal.budget = 2_000;
    assert!(paint_flower(&s, &o).unwrap().planes == reference(&s));
}

#[test]
fn errors_name_the_petal() {
    let bad = r##"{"width": 10, "height": 10, "petals": [{"brush": "ink", "points": [[1, 1]]}, {"brush": "ink", "sise": 3, "points": [[1, 1]]}]}"##;
    let bad: serde_json::Value = serde_json::from_str(bad).unwrap();
    let e = Flower::from_spec(&super::spec(&super::full(&bad)))
        .unwrap_err()
        .to_string();
    assert!(e.starts_with("petal 1:") && e.contains("sise"), "{e}");
}

#[test]
fn png_round_trip_of_a_render() {
    let s = flower("brush_sheet.flower");
    let img = paint_flower(&s, &RenderOpts::default()).unwrap().dry();
    assert_eq!(
        crate::decode_canvas(&crate::encode_canvas(&img)).unwrap(),
        img
    );
}
