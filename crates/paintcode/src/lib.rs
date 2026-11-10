//! paintcode: a flower goes in, a canvas comes out.
//!
//! A flower is what you write: a size, a background and petals (strokes of any shape). Painting
//! turns each petal into dabs; all the dabs together are the coat. The coat over the background
//! is the canvas, the pixels you see.
//!
//! One library, one module per part of the engine. Dependencies point one way, from the leaf up,
//! and `tests/layering.rs` fails if a module reaches back down the list:
//!
//! | module   | part   | may use               | does                                            |
//! |----------|--------|-----------------------|-------------------------------------------------|
//! | `error`  | leaf   | nothing               | every way paintcode fails, as one enum          |
//! | `profile`| leaf   | nothing               | stage timers, empty without the `profile` feature|
//! | `math`   | leaf   | nothing               | counter-based RNG, dab profile and geometry     |
//! | `flower` | bones  | error, math           | flower, petal and brush types, brushes, colours |
//! | `render` | render | error, math           | dabs to pixels, coats, paper grain, downsampling|
//! | `petal`  | flesh  | profile, math, flower, render | petals to dabs, carrying paint          |
//! | `io`     | files  | error, flower, render | encoders and decoders, in pairs                 |
//! | `python` | face   | everything            | `paintcode._core`, with the `python` feature    |
//!
//! This file only conducts: [`flower_to_canvas`] walks the petals of a [`Flower`] over a wet canvas
//! and dries it into a [`Canvas`], [`flower_to_coat`] keeps the dabs it lays, and
//! [`coat_to_canvas`] lays them again.
//!
//! The public API is what Python can do: the modules are private, and only the nouns, the
//! functions between them and the encoders are exported, with [`profile`] for the benches and,
//! with the `python` feature, `stub_info` for `src/bin/stub_gen.rs`. Tests that reach inside a
//! module live in `src/tests/`.

#![deny(
    clippy::unwrap_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::panic))]
#![feature(portable_simd)]

mod error;
mod flower;
mod io;
mod math;
mod petal;
pub mod profile;
#[cfg(feature = "python")]
mod python;
mod render;
#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use flower::Flower;
pub use io::{
    decode_canvas, decode_coat, decode_flower, encode_canvas, encode_coat, encode_flower,
};
#[cfg(feature = "python")]
pub use python::stub_info;
pub use render::{Canvas, Coat};

use flower::Petal;
use petal::Options;
use render::WetCanvas;

#[derive(Debug, Clone, Copy)]
pub struct RenderOpts {
    /// Render larger: coordinates and brush sizes scale together.
    pub scale: f64,
    /// Paint at N times the size and box-filter down; crisp hard edges at 2 or 3.
    pub supersample: i64,
    /// How fast, never what it looks like: tests turn these to check fast paths against slow ones.
    pub(crate) petal: Options,
}

impl RenderOpts {
    /// Options at this scale and supersampling.
    pub fn new(scale: f64, supersample: i64) -> RenderOpts {
        RenderOpts {
            scale,
            supersample,
            petal: Options::default(),
        }
    }
}

impl Default for RenderOpts {
    fn default() -> Self {
        RenderOpts::new(1.0, 1)
    }
}

/// The canvas with a flower on it, which [`encode_canvas`] writes into the PNG. Any flower that was
/// on it is replaced.
pub fn flower_on_canvas(flower: &Flower, canvas: Canvas) -> Canvas {
    let text = String::from_utf8(encode_flower(flower)).expect("JSON is UTF-8");
    Canvas {
        flower: Some(text),
        ..canvas
    }
}

/// The flower on a canvas. A canvas without one is an error.
pub fn flower_off_canvas(canvas: &Canvas) -> Result<Flower> {
    let text = canvas.flower.as_deref().ok_or(Error::NoFlower)?;
    Flower::from_json(text)
}

/// Paint a flower: its canvas.
pub fn flower_to_canvas(flower: &Flower, opts: &RenderOpts) -> Result<Canvas> {
    let canvas = paint_flower(flower, opts)?;
    Ok(profile::time(profile::Stage::Dry, || canvas.dry()))
}

/// Paint a flower on a blank canvas, and leave it wet: float, with its wetness.
pub(crate) fn paint_flower(flower: &Flower, opts: &RenderOpts) -> Result<WetCanvas> {
    paint(flower, opts, false)
}

/// Paint a flower and keep every dab it lays, in order: its coat.
pub fn flower_to_coat(flower: &Flower, opts: &RenderOpts) -> Result<Coat> {
    let canvas = paint(flower, opts, true)?;
    Ok(Coat {
        width: canvas.width,
        height: canvas.height,
        supersample: opts.supersample as usize,
        background: flower.background,
        seed: flower.seed,
        dabs: canvas.dabs.unwrap_or_default(),
    })
}

/// Lay a coat's dabs again: the same canvas, bit for bit, as painting its flower.
pub fn coat_to_canvas(coat: &Coat) -> Result<Canvas> {
    let canvas = lay_coat(coat)?;
    Ok(profile::time(profile::Stage::Dry, || canvas.dry()))
}

/// Lay a coat's dabs on a blank canvas, and leave it wet: the same, bit for bit, as
/// [`paint_flower`] on the flower that made the coat.
pub(crate) fn lay_coat(coat: &Coat) -> Result<WetCanvas> {
    render::lay_coat(coat)
}

fn paint(flower: &Flower, opts: &RenderOpts, record: bool) -> Result<WetCanvas> {
    let size = Size::of(flower, opts)?;
    let mut canvas = profile::time(profile::Stage::Blank, || size.blank(flower))?;
    if record {
        canvas.dabs = Some(Vec::new());
    }
    let keys = flower.jitter_keys();
    lay_petals(&mut canvas, &flower.petals, &keys, size.scale, opts);
    profile::time(profile::Stage::Downsample, || {
        canvas.downsample(size.supersample)
    });
    Ok(canvas)
}

/// The canvas a flower is painted on, after checking that the flower and the options can be
/// painted at all.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Size {
    /// In laid pixels: scaled and supersampled.
    width: usize,
    height: usize,
    /// What the flower's coordinates and brush sizes are multiplied by.
    scale: f64,
    supersample: usize,
}

impl Size {
    fn of(flower: &Flower, opts: &RenderOpts) -> Result<Size> {
        if !(opts.scale.is_finite() && opts.scale > 0.0) {
            return Err(Error::Flower(format!(
                "scale must be a positive number, got {}",
                opts.scale
            )));
        }
        let ss = opts.supersample;
        if ss < 1 {
            return Err(Error::Flower(format!(
                "supersample must be at least 1, got {ss}"
            )));
        }
        let side = |v: usize| (v as f64 * opts.scale).round_ties_even() as usize;
        let (w, h) = (side(flower.width), side(flower.height));
        if w == 0 || h == 0 {
            return Err(Error::Flower(format!(
                "a {}x{} flower at scale {} has no pixels",
                flower.width, flower.height, opts.scale
            )));
        }
        let (fw, fh) = (flower.width, flower.height);
        for (i, p) in flower.petals.iter().enumerate() {
            if let Some([x, y]) = p.far_point(fw, fh) {
                return Err(Error::Flower(format!(
                    "petal {i}: point [{x}, {y}] is too far outside the {fw}x{fh} canvas \
                     (more than {} px)",
                    flower::POINT_MARGIN * fw.max(fh) as f64
                )));
            }
        }
        Ok(Size {
            width: w.saturating_mul(ss as usize),
            height: h.saturating_mul(ss as usize),
            scale: opts.scale * ss as f64,
            supersample: ss as usize,
        })
    }

    fn blank(&self, flower: &Flower) -> Result<WetCanvas> {
        WetCanvas::new(self.width, self.height, flower.background, flower.seed)
    }
}

/// Lay petals on a canvas, after the ones already on it, each with its jitter key.
fn lay_petals(
    canvas: &mut WetCanvas,
    petals: &[Petal],
    keys: &[i64],
    scale: f64,
    opts: &RenderOpts,
) {
    let seed = canvas.seed;
    let plan = |i: usize| {
        let p: &Petal = &petals[i];
        let brush = p.brush.scaled(scale);
        let pts: Vec<[f64; 4]> = p
            .points
            .iter()
            .map(|q| [q[0] * scale, q[1] * scale, q[2], q[3]])
            .collect();
        let planned = profile::time(profile::Stage::Plan, || {
            petal::plan_petal(seed, keys[i], &brush, &pts, p.smooth)
        });
        (brush, planned)
    };
    let n = petals.len().min(keys.len());
    let mut next = (n > 0).then(|| plan(0));
    for (i, p) in petals.iter().enumerate().take(n) {
        let (brush, planned) = next.take().expect("planned before it is painted");
        // the next petal is planned while this one is painted: planning reads no pixel
        if render::alone() {
            petal::paint_planned(canvas, &planned, &brush, &p.color, opts.petal);
            next = (i + 1 < n).then(|| plan(i + 1));
            continue;
        }
        (_, next) = rayon::join(
            || petal::paint_planned(canvas, &planned, &brush, &p.color, opts.petal),
            || (i + 1 < n).then(|| plan(i + 1)),
        );
    }
}
