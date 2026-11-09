//! What moved between two PNGs, for people reading a diff:
//!
//!     cargo run -q --release -p paintcode --example diff -- OLD.png NEW.png
//!
//! It also takes the seven arguments git gives a diff driver (path, old file, old hash, old mode,
//! new file, new hash, new mode), so with `.gitattributes` naming the `png` driver and
//!
//!     git config diff.png.command 'cargo run -q --release -p paintcode --example diff --'
//!
//! `git diff` says how many 8-bit values of each image moved and by how much, not its size.
use std::io::Cursor;

use paintcode::{Flower, decode_canvas, flower_off_canvas};

/// A PNG as the bytes of its pixels and the flower on it. The engine keeps a canvas's pixels to
/// itself, so they are read here with the png crate, expanded the way the engine expands them.
struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    flower: Option<Flower>,
}

fn load(path: &str) -> Option<Image> {
    // git passes /dev/null for the side of an added or a deleted file
    let bytes = std::fs::read(path).ok().filter(|b| !b.is_empty())?;
    let flower = flower_off_canvas(&decode_canvas(&bytes).ok()?).ok();
    let mut dec = png::Decoder::new(Cursor::new(&bytes[..]));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut pixels).ok()?;
    pixels.truncate(frame.buffer_size());
    Some(Image {
        width: frame.width,
        height: frame.height,
        pixels,
        flower,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (name, old, new) = match args.len() {
        2 if args[1] == "/dev/null" => (args[0].as_str(), &args[0], &args[1]),
        2 => (args[1].as_str(), &args[0], &args[1]),
        7 => (args[0].as_str(), &args[1], &args[4]),
        _ => {
            eprintln!("diff OLD.png NEW.png, or the seven arguments of a git diff driver");
            std::process::exit(2);
        }
    };
    let line = match (load(old), load(new)) {
        (None, None) => "not a PNG on either side".to_string(),
        (None, Some(n)) => format!("new, {}x{}", n.width, n.height),
        (Some(o), None) => format!("gone, was {}x{}", o.width, o.height),
        (Some(o), Some(n)) if (o.width, o.height) != (n.width, n.height) => {
            format!("{}x{} -> {}x{}", o.width, o.height, n.width, n.height)
        }
        (Some(o), Some(n)) => {
            let moved: Vec<u8> = o
                .pixels
                .iter()
                .zip(&n.pixels)
                .map(|(a, b)| a.abs_diff(*b))
                .filter(|&d| d > 0)
                .collect();
            let pixels = match moved.iter().max() {
                None => "the same pixels".to_string(),
                Some(max) => format!(
                    "{} of {} values moved, by up to {max}/255",
                    moved.len(),
                    o.pixels.len()
                ),
            };
            let flower = match (&o.flower, &n.flower) {
                (a, b) if a == b => "",
                (None, Some(_)) => "; a flower is on it now",
                (Some(_), None) => "; its flower is gone",
                _ => "; its flower changed",
            };
            format!("{pixels}{flower}")
        }
    };
    println!("{name}: {line}");
}
