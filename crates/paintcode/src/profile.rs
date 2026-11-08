//! Where a paint's time goes, stage by stage, when the crate is built with the `profile` feature.
//!
//! [`time`] runs a closure and, with the feature, adds its wall time to its stage; [`take`] reads
//! the totals and starts them again. Without the feature `time` only runs the closure and the
//! totals stay empty, so the engine pays nothing for being measurable.
//!
//! The totals are per process and add up every thread's calls, so a stage's time reads as wall
//! time only when the paint runs on one thread.

/// A part of a paint, in the order a petal meets them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The canvas the flower is painted on, with its background.
    Blank,
    /// A petal's path resampled and its dabs placed, before any pixel is read.
    Plan,
    /// The paper grain a petal's brush needs, made once per grain scale.
    Prepare,
    /// The canvas under a petal kept as it was, for pickup to read.
    Snapshot,
    /// Every dab's kernel, and the wet paint under it.
    Footprint,
    /// The colour the brush carries from dab to dab.
    Carry,
    /// The footprints laid on the canvas in order.
    Composite,
    /// A supersampled canvas box-filtered to its size.
    Downsample,
    /// The float canvas rounded to 8 bits.
    Dry,
}

impl Stage {
    pub const ALL: [Stage; 9] = [
        Stage::Blank,
        Stage::Plan,
        Stage::Prepare,
        Stage::Snapshot,
        Stage::Footprint,
        Stage::Carry,
        Stage::Composite,
        Stage::Downsample,
        Stage::Dry,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stage::Blank => "blank",
            Stage::Plan => "plan",
            Stage::Prepare => "prepare",
            Stage::Snapshot => "snapshot",
            Stage::Footprint => "footprint",
            Stage::Carry => "carry",
            Stage::Composite => "composite",
            Stage::Downsample => "downsample",
            Stage::Dry => "dry",
        }
    }
}

/// One stage's totals since the last [`take`].
#[derive(Debug, Clone, Copy)]
pub struct Spent {
    pub stage: Stage,
    pub nanos: u64,
    pub calls: u64,
}

#[cfg(feature = "profile")]
mod totals {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

    const N: usize = super::Stage::ALL.len();
    static NANOS: [AtomicU64; N] = [const { AtomicU64::new(0) }; N];
    static CALLS: [AtomicU64; N] = [const { AtomicU64::new(0) }; N];

    pub fn add(i: usize, nanos: u64) {
        NANOS[i].fetch_add(nanos, Relaxed);
        CALLS[i].fetch_add(1, Relaxed);
    }

    pub fn take(i: usize) -> (u64, u64) {
        (NANOS[i].swap(0, Relaxed), CALLS[i].swap(0, Relaxed))
    }
}

/// Run `f` as part of `stage`.
#[inline(always)]
pub fn time<T>(stage: Stage, f: impl FnOnce() -> T) -> T {
    #[cfg(feature = "profile")]
    {
        let start = std::time::Instant::now();
        let out = f();
        totals::add(stage as usize, start.elapsed().as_nanos() as u64);
        out
    }
    #[cfg(not(feature = "profile"))]
    {
        let _ = stage;
        f()
    }
}

/// Every stage's totals since the last call, in the order of [`Stage::ALL`]; all zero without the
/// `profile` feature.
pub fn take() -> Vec<Spent> {
    Stage::ALL
        .iter()
        .map(|&stage| {
            #[cfg(feature = "profile")]
            let (nanos, calls) = totals::take(stage as usize);
            #[cfg(not(feature = "profile"))]
            let (nanos, calls) = (0, 0);
            Spent {
                stage,
                nanos,
                calls,
            }
        })
        .collect()
}
