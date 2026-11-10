use std::collections::HashMap;

use super::{Brush, Error, Result, Spec, err, parse_color};
use crate::math::rng;

/// What a petal paints with.
#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub enum Color {
    /// 0..255, as it was written.
    Rgb([f32; 3]),
    /// `None`: load the brush with whatever is under its first touch (a smudge).
    Smudge,
}

/// One petal: a stroke of any shape, with its brush resolved and its overrides applied.
#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub struct Petal {
    pub brush: Brush,
    pub color: Color,
    /// Rows of (x, y, pressure, tilt), every one written in full.
    pub points: Vec<[f64; 4]>,
    pub smooth: bool,
    /// A name of its own, unique in the flower: its jitter follows it wherever it moves.
    pub id: Option<String>,
    /// The petals it belongs with; its jitter follows its place among them.
    pub group: Option<String>,
}

/// A flower: what a painter writes. A size, a background, a seed for every random number, and the
/// petals in the order they are laid down, each with its whole brush. This is what a `.flower`
/// keeps; brush names and colours in hex are resolved when it is written.
#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub struct Flower {
    pub(crate) width: usize,
    pub(crate) height: usize,
    /// 0..255, as it was written.
    pub(crate) background: [f32; 3],
    pub(crate) seed: i64,
    pub(crate) petals: Vec<Petal>,
}

/// The keys of a flower, every one written, and the one it may have besides.
const FLOWER_KEYS: [&str; 5] = ["width", "height", "background", "seed", "petals"];
const FLOWER_MAY: [&str; 1] = ["brushes"];
/// The keys of a petal, every one written, and the ones it may have besides (with brush fields).
const PETAL_KEYS: [&str; 4] = ["brush", "color", "points", "smooth"];
const PETAL_MAY: [&str; 2] = ["id", "group"];

impl Flower {
    /// A flower from what a painter wrote. Every key is written: nothing has a default.
    pub fn from_spec(v: &Spec) -> Result<Flower> {
        let root = v
            .as_map()
            .ok_or_else(|| Error::Flower(format!("a flower is a dict, got {v}")))?;
        for (k, _) in root {
            if !FLOWER_KEYS.contains(&k.as_str()) && !FLOWER_MAY.contains(&k.as_str()) {
                return err(format!("unknown key {k:?}; a flower has {FLOWER_KEYS:?}"));
            }
        }
        let need = |k: &str| {
            v.get(k)
                .ok_or_else(|| Error::Flower(format!("the flower has no {k}")))
        };
        // whole numbers: a size of 640.7 or a seed of 1.5 is a mistake to report, not a value to
        // round quietly
        let int = |k: &str, min: i64| -> Result<i64> {
            let v = need(k)?;
            let n = match *v {
                Spec::Int(n) => Some(n),
                Spec::Num(f) if f.fract() == 0.0 && f.abs() < 9e15 => Some(f as i64),
                _ => None,
            }
            .ok_or_else(|| Error::Flower(format!("{k} must be a whole number, got {v}")))?;
            if n < min {
                return err(format!("{k} must be at least {min}, got {v}"));
            }
            Ok(n)
        };
        let width = int("width", 1)? as usize;
        let height = int("height", 1)? as usize;
        let background = parse_color(need("background")?)?;
        let seed = int("seed", i64::MIN)?;

        let mut named: HashMap<&str, Brush> = HashMap::new();
        if let Some(bs) = v.get("brushes") {
            let bs = bs
                .as_map()
                .ok_or_else(|| Error::Flower(format!("brushes is a dict of brushes, got {bs}")))?;
            for (name, spec) in bs {
                named.insert(
                    name,
                    Brush::from_spec(spec)
                        .map_err(|e| Error::Flower(format!("brush {name}: {e}")))?,
                );
            }
        }

        let list = need("petals")?;
        let list = list
            .as_list()
            .ok_or_else(|| Error::Flower(format!("petals is a list, got {list}")))?;
        let mut petals = Vec::with_capacity(list.len());
        for (idx, s) in list.iter().enumerate() {
            petals.push(
                parse_petal(s, &named).map_err(|e| Error::Flower(format!("petal {idx}: {e}")))?,
            );
        }
        let mut ids: HashMap<&str, usize> = HashMap::new();
        for (idx, p) in petals.iter().enumerate() {
            if let Some(id) = p.id.as_deref()
                && let Some(first) = ids.insert(id, idx)
            {
                return err(format!("petal {idx}: id {id:?} is petal {first}'s already"));
            }
        }
        Ok(Flower {
            width,
            height,
            background,
            seed,
            petals,
        })
    }

    /// The flower written out in full: every petal with its whole brush, every colour as numbers.
    /// [`Flower::from_spec`] takes it back as it is.
    pub fn to_spec(&self) -> Spec {
        let rgb = |c: [f32; 3]| Spec::List(c.map(|v| Spec::Num(v as f64)).to_vec());
        let petals = self
            .petals
            .iter()
            .map(|p| {
                let mut m = vec![
                    ("brush".to_string(), p.brush.to_spec()),
                    (
                        "color".to_string(),
                        match p.color {
                            Color::Rgb(c) => rgb(c),
                            Color::Smudge => Spec::Null,
                        },
                    ),
                    (
                        "points".to_string(),
                        Spec::List(
                            p.points
                                .iter()
                                .map(|r| Spec::List(r.map(Spec::Num).to_vec()))
                                .collect(),
                        ),
                    ),
                    ("smooth".to_string(), Spec::Bool(p.smooth)),
                ];
                if let Some(id) = &p.id {
                    m.push(("id".to_string(), Spec::Str(id.clone())));
                }
                if let Some(g) = &p.group {
                    m.push(("group".to_string(), Spec::Str(g.clone())));
                }
                Spec::Map(m)
            })
            .collect();
        Spec::Map(vec![
            ("width".to_string(), Spec::Int(self.width as i64)),
            ("height".to_string(), Spec::Int(self.height as i64)),
            ("background".to_string(), rgb(self.background)),
            ("seed".to_string(), Spec::Int(self.seed)),
            ("petals".to_string(), Spec::List(petals)),
        ])
    }

    /// The background as painting uses it: 0..1.
    pub(crate) fn paint_background(&self) -> [f32; 3] {
        super::color::unit(self.background)
    }
}

impl Flower {
    /// What keys each petal's jitter, in order. A petal with an `id` is keyed by it, so taking out
    /// or adding other petals never moves its jitter. A petal in a `group` (and without an id) is
    /// keyed by the group and its place among the group's petals without an id, so only its own
    /// group can move it. Any other petal is keyed by its index in the flower, as it always was.
    pub(crate) fn jitter_keys(&self) -> Vec<i64> {
        let mut places: HashMap<&str, i64> = HashMap::new();
        self.petals
            .iter()
            .enumerate()
            .map(|(i, p)| match (&p.id, &p.group) {
                (Some(id), _) => rng::named(&[rng::ID], id),
                (None, Some(g)) => {
                    let place = places.entry(g).or_insert(0);
                    *place += 1;
                    rng::named(&[rng::GROUP, *place - 1], g)
                }
                (None, None) => i as i64,
            })
            .collect()
    }
}

/// How far outside the canvas a control point may lie, in canvas sizes. Petals may start off the
/// canvas, but the path is walked a pixel at a time, so a point a million pixels away (a typo in
/// a coordinate) would take seconds and gigabytes before anything is painted.
pub const POINT_MARGIN: f64 = 16.0;

impl Petal {
    /// The first control point more than [`POINT_MARGIN`] canvas sizes outside a `width` x
    /// `height` canvas, if there is one.
    pub fn far_point(&self, width: usize, height: usize) -> Option<[f64; 2]> {
        let margin = POINT_MARGIN * width.max(height) as f64;
        let outside = |v: f64, side: usize| v < -margin || v > side as f64 + margin;
        self.points
            .iter()
            .find(|p| outside(p[0], width) || outside(p[1], height))
            .map(|p| [p[0], p[1]])
    }
}

fn parse_petal(s: &Spec, named: &HashMap<&str, Brush>) -> Result<Petal> {
    let m = s
        .as_map()
        .ok_or_else(|| Error::Flower(format!("a petal is a dict, got {s}")))?;
    let need = |k: &str| {
        s.get(k)
            .ok_or_else(|| Error::Flower(format!("the petal has no {k}")))
    };
    let mut brush = match need("brush")? {
        Spec::Str(name) if named.contains_key(name.as_str()) => named[name.as_str()].clone(),
        other => Brush::from_spec(other)?,
    };
    let overrides: Vec<(String, Spec)> = m
        .iter()
        .filter(|(k, _)| !PETAL_KEYS.contains(&k.as_str()) && !PETAL_MAY.contains(&k.as_str()))
        .cloned()
        .collect();
    if !overrides.is_empty() {
        brush = brush.with_fields(&overrides)?;
    }
    let color = match need("color")? {
        Spec::Null => Color::Smudge,
        c => Color::Rgb(parse_color(c)?),
    };
    let pts = need("points")?;
    let pts = pts
        .as_list()
        .ok_or_else(|| Error::Flower(format!("points is a list of rows, got {pts}")))?;
    let mut points = Vec::with_capacity(pts.len());
    for pt in pts {
        let a = pt.as_list().filter(|a| a.len() == 4).ok_or_else(|| {
            Error::Flower(format!(
                "a point is a row of four, [x, y, pressure, tilt], got {pt}"
            ))
        })?;
        let mut row = [0.0; 4];
        for (o, v) in row.iter_mut().zip(a) {
            *o = v
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| Error::Flower(format!("point values must be numbers, got {pt}")))?;
        }
        points.push(row);
    }
    if points.is_empty() {
        return err("petal has no points");
    }
    let name = |k: &str| -> Result<Option<String>> {
        match s.get(k) {
            None => Ok(None),
            Some(Spec::Str(t)) if !t.is_empty() => Ok(Some(t.clone())),
            Some(v) => err(format!("{k} must be a non-empty string, got {v}")),
        }
    };
    let (id, group) = (name("id")?, name("group")?);
    let smooth = match need("smooth")? {
        Spec::Bool(b) => *b,
        v => return err(format!("smooth must be True or False, got {v}")),
    };
    Ok(Petal {
        brush,
        color,
        points,
        smooth,
        id,
        group,
    })
}
