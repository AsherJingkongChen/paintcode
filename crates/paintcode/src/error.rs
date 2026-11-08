//! Every way paintcode can fail, as one type. A message names what failed, so a person or a
//! model can fix it: the petal and the field, the file, or the size. Where another library's
//! error is the cause, it is kept as the source, so a caller can still look under the message.

use std::collections::TryReserveError;

/// A failure, by what failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A flower the engine refuses, or cannot paint with the options given.
    #[error("{0}")]
    Flower(String),
    /// A flower that is not JSON at all.
    #[error("flower is not JSON: {0}")]
    FlowerJson(#[from] serde_json::Error),
    /// Bytes that are not the file they were read as.
    #[error("{0}")]
    File(String),
    /// A PNG the png crate could not read.
    #[error("bad PNG: {0}")]
    Png(#[from] png::DecodingError),
    /// A flower on a PNG that cannot be read: its compressed text is broken, longer than 64 MiB
    /// or not UTF-8. The pixels around it are still a canvas.
    #[error("the flower on this PNG is unreadable: {0}")]
    FlowerChunk(#[source] png::DecodingError),
    /// A canvas larger than the limit on one: 2^28 pixels, counted after scale and supersample.
    #[error("{0}")]
    Size(String),
    /// Memory that is not there, asked for a canvas or a buffer within the limit.
    #[error("not enough memory for {what}")]
    Memory {
        what: String,
        #[source]
        source: TryReserveError,
    },
    /// A canvas with no flower on it, read for one.
    #[error("no flower on this canvas")]
    NoFlower,
}

pub type Result<T> = std::result::Result<T, Error>;
