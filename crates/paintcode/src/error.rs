//! Every way paintcode can fail, as one type. A message names what failed, so a person or a
//! model can fix it: the petal and the field, the file, or the size.

/// A failure, by what failed.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// A flower the engine refuses, or cannot paint with the options given.
    #[error("{0}")]
    Flower(String),
    /// Bytes that are not the file they were read as.
    #[error("{0}")]
    File(String),
    /// A canvas larger than the limit on one: 2^28 pixels, counted after scale and supersample.
    #[error("{0}")]
    Size(String),
    /// Memory that is not there, asked for a canvas or a buffer within the limit.
    #[error("{0}")]
    Memory(String),
    /// A canvas with no flower on it, read for one.
    #[error("no flower on this canvas")]
    NoFlower,
}

pub type Result<T> = std::result::Result<T, Error>;
