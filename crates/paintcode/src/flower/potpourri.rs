//! What a painter writes, before it is a flower: a potpourri, everything loose in one jar, a small
//! tree of maps, lists, strings, numbers, booleans and nulls. A Python dict becomes one
//! (`src/python.rs`), and so does every flower read from a file, so a flower passes the same checks
//! however it arrives.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Potpourri {
    Null,
    Bool(bool),
    /// A whole number as it was written, so a 64-bit seed keeps every bit.
    Int(i64),
    Num(f64),
    Str(String),
    List(Vec<Potpourri>),
    /// Keys in the order they were written.
    Map(Vec<(String, Potpourri)>),
}

impl Potpourri {
    /// The value under `key`, if this is a map that has it.
    pub fn get(&self, key: &str) -> Option<&Potpourri> {
        match self {
            Potpourri::Map(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Potpourri::Num(v) => Some(v),
            Potpourri::Int(v) => Some(v as f64),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&[(String, Potpourri)]> {
        match self {
            Potpourri::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Potpourri]> {
        match self {
            Potpourri::List(l) => Some(l),
            _ => None,
        }
    }
}

/// Written the way Python would print it, for error messages.
impl fmt::Display for Potpourri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Potpourri::Null => write!(f, "None"),
            Potpourri::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Potpourri::Int(v) => write!(f, "{v}"),
            Potpourri::Num(v) => write!(f, "{v:?}"),
            Potpourri::Str(s) => write!(f, "{s:?}"),
            Potpourri::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.iter().enumerate() {
                    write!(f, "{}{v}", if i > 0 { ", " } else { "" })?;
                }
                write!(f, "]")
            }
            Potpourri::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    write!(f, "{}{k:?}: {v}", if i > 0 { ", " } else { "" })?;
                }
                write!(f, "}}")
            }
        }
    }
}
