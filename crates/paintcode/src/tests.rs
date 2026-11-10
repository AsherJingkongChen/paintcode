//! Tests that reach inside the crate. The public API is what Python sees, so a test of a part of
//! the engine lives here, beside it, rather than in tests/.
//!
//! A test flower is written as a `json!` literal, turned into the [`Spec`] a Python dict would
//! become. serde_json is only a dev-dependency, for that.

mod engine;
mod exact;
mod golden;
mod splats;

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::Flower;
use crate::flower::Spec;

/// The repository root.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A `json!` value as the [`Spec`] the same Python value would become.
pub fn spec(v: &Value) -> Spec {
    match v {
        Value::Null => Spec::Null,
        Value::Bool(b) => Spec::Bool(*b),
        // a seed may use all 64 bits, as a Python int may
        Value::Number(n) => match n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)) {
            Some(i) => Spec::Int(i),
            None => Spec::Num(n.as_f64().expect("a JSON number")),
        },
        Value::String(s) => Spec::Str(s.clone()),
        Value::Array(a) => Spec::List(a.iter().map(spec).collect()),
        Value::Object(m) => Spec::Map(m.iter().map(|(k, v)| (k.clone(), spec(v))).collect()),
    }
}

/// A test flower with what a painter must write filled in from what 0.10 assumed when it was left
/// out: an 800 x 600 white canvas and seed 0; a black round brush, smooth, pressure 1 and tilt 0.
/// A test of painting then says only what it is about. The parser itself fills in nothing.
pub fn full(v: &Value) -> Value {
    let mut v = v.clone();
    let Some(root) = v.as_object_mut() else {
        return v;
    };
    let put = |m: &mut serde_json::Map<String, Value>, k: &str, d: Value| {
        m.entry(k.to_string()).or_insert(d);
    };
    put(root, "width", json!(800));
    put(root, "height", json!(600));
    put(root, "background", json!([255, 255, 255]));
    put(root, "seed", json!(0));
    put(root, "petals", json!([]));
    for p in root["petals"].as_array_mut().into_iter().flatten() {
        let Some(p) = p.as_object_mut() else { continue };
        put(p, "brush", json!("round"));
        put(p, "color", json!([0, 0, 0]));
        put(p, "smooth", json!(true));
        for row in p
            .get_mut("points")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            if let Some(r) = row.as_array_mut() {
                if r.len() == 2 {
                    r.push(json!(1.0));
                }
                if r.len() == 3 {
                    r.push(json!(0.0));
                }
            }
        }
    }
    v
}

/// A test flower from a `json!` literal, filled in by [`full`].
pub fn flower(v: &Value) -> Flower {
    Flower::from_spec(&spec(&full(v))).unwrap()
}

/// A `.flower` in the repository, by its path from the root.
pub fn load(path: &str) -> Flower {
    crate::decode_flower(&std::fs::read(root().join(path)).unwrap()).unwrap()
}
