//! The bones: a flower, the JSON a painter writes, as plain data.
//!
//! A [`Flower`] is a size, a background, a seed, named brushes and an ordered list of petals
//! (strokes of any shape). Nothing here paints; it only parses and validates.

mod brush;
mod color;
mod parse;

#[cfg(feature = "python")]
pub use brush::BRUSH_NAMES;
pub use brush::Brush;
#[cfg(any(test, feature = "python"))]
pub use brush::brush;
pub use color::parse_color;
pub use parse::{Color, Flower, POINT_MARGIN, Petal};

pub use crate::error::{Error, Result};

pub(crate) fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Flower(msg.into()))
}
