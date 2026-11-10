# Changelog

What changed between versions, newest first. How the engine works is in [DESIGN.md](DESIGN.md); how to use it is in the [README](README.md).

## 0.10

Paints the same pixels as 0.9. A canvas carries its flower in a `pcFL` chunk of its own instead of an iTXt text chunk, so a `.flower.png` saved by 0.9 opens as a canvas without a flower: load its flower with 0.9 and save it again. The flower is still its JSON, compressed as before. Two flowers on one PNG are an error. png is a fork of 0.18.1 that can keep a chunk it does not decode (DESIGN.md, "The flower on its canvas").

## 0.9

Paints the same pixels as 0.8. The Python API loses two methods, changes one and gains a type:

- `Canvas.png()` and `Canvas.pixels()` are gone. Nothing but the tests used them: a canvas is seen through the PNG `save_canvas` writes.
- `Canvas.peek(x, y, width, height)` takes a rectangle and gives each channel's median over it, the upper one of an even count. It was one pixel; `width=1, height=1` is that pixel.
- `Flower` is new: the type of a flower, `dict[str, Any]`, for type hints.

The Rust side is one crate: the binding is the `python` feature of `crates/paintcode`, the modules are private, and the crate exports what Python can do (with `profile` for the benches). `RenderOpts::new` and `Coat::len` are new; `paint_flower`, `lay_coat` and the fields behind them are crate-only. The type stub is generated from the binding, and the README ends in it.

## 0.8

Paints the same pixels as 0.7. `Painting` is gone: a version is painted once, so there was nothing for it to remember between paints. The flower on its canvas is the record to go on from.

## 0.7

Paints the same pixels as 0.6, but for two changes. A smudge that starts off the canvas loads the colour where it meets the canvas, where it loaded black (in kiku, one blender petal, 1,464 pixels by up to 24/255). The dab profile is read from a table per hardness instead of `powf` and `exp`, 25% faster on one thread, at most 1.1e-4 off the formula (2 of kiku's 1,228,800 values move by 1/255). It adds `Painting`, which paints only the petals a flower gained since the last paint, and the petal keys `id` and `group`, which keep a petal's jitter when others are taken out; a flower without them paints as before.

## 0.6

Paints the same pixels as 0.5 under new names: a scene is a flower, a stroke is a petal, `PRESETS` is `BRUSHES` (and a brush object names the brush it starts from with `"brush"`, not `"preset"`), and the flower is flat (`width`, `height`, `background` and `seed` at the top; `supersample` is an option of the paint). The `.petal` ZIP is gone: a `.flower.json` holds the flower and a `.flower.png` carries it on its canvas. Painting over a base image is gone too. A `.coat.ply` keeps every dab exactly, so it is larger than 0.5's `.ply`.

## 0.5

Paints the same pixels as 0.4 (within 1/255 on the PNG, 1e-5 on the float canvas), with the Rust core in place of the numpy engine. It is stricter about mistakes: canvas `width`, `height`, `seed` and `supersample` must be whole numbers written as numbers (0.4 also took `"640"` and rounded `640.7`), a control point may not lie more than 16 canvas sizes off the canvas (the base image's size, when there is one), and a canvas or a base PNG is at most 2^28 pixels after scale and supersample; each of these is an error naming the stroke or field, where 0.4 rounded quietly, hung or crashed. A base image is PNG only; 0.4 took anything Pillow opens. 0.4's command line (`python -m paintcode`) is gone; every option it had is a keyword of `render_png` and `render_ply`. A 16-bit base PNG keeps its tones (0.4 clipped 16-bit grey to white).

## 0.4

Changed the random numbers (jitter and paper tooth), so a scene from 0.3 renders with slightly different jitter.
