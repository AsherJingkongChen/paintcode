use super::{Potpourri, Result, err};

/// `[r, g, b]` in 0..255, or `"#rrggbb"` / `"#rgb"`, as float32 0..255: the colour as it was
/// written, which a flower keeps; [`unit`] is what painting uses.
pub fn parse_color(c: &Potpourri) -> Result<[f32; 3]> {
    match c {
        Potpourri::Str(s) => {
            let s = s.trim_start_matches('#');
            let s: String = if s.len() == 3 {
                s.chars().flat_map(|ch| [ch, ch]).collect()
            } else {
                s.to_string()
            };
            if s.len() != 6 || !s.is_ascii() {
                return err(format!("bad colour {c}"));
            }
            let mut out = [0f32; 3];
            for (i, o) in out.iter_mut().enumerate() {
                *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16)
                    .map_err(|_| super::Error::Flower(format!("bad colour {c}")))?
                    as f32;
            }
            Ok(out)
        }
        Potpourri::List(a) if a.len() == 3 => {
            let mut out = [0f32; 3];
            for (o, v) in out.iter_mut().zip(a) {
                *o = v
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| super::Error::Flower(format!("bad colour {c}")))?
                    as f32;
            }
            Ok(out)
        }
        _ => err(format!("colour must have 3 channels, got {c}")),
    }
}

/// A colour in 0..255 as painting uses it: 0..1, clamped.
pub fn unit(c: [f32; 3]) -> [f32; 3] {
    c.map(|v| (v / 255.0f32).clamp(0.0, 1.0))
}
