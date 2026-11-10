//! Tests that reach inside the crate. The public API is what Python sees, so a test of a part of
//! the engine lives here, beside it, rather than in tests/.

mod engine;
mod exact;
mod golden;
mod splats;
