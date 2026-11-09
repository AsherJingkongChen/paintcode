# paintcode

A small dab engine for painting from code.

A flower is what you write: a size, a background and petals (strokes of any shape).
Painting turns each petal into dabs; all the dabs together are the coat.
The coat over the background is the canvas, the pixels you see.

The nouns are the stages of a painting, each made from the one above it, and every name in the API is one of them. One is a single thing; many is the collection of them. File formats may change; these names do not.

| one | many | stage | kept as |
|---|---|---|---|
| petal | flower | what you write | `.flower.json`, or on a `.flower.png` |
| dab | coat | what painting lays | `.coat.ply` |
| pixel | canvas | what you see | `.png`, with its flower as `.flower.png` |

A petal is a list of control points with pressure and tilt; the engine resamples it along its arc length and lays down a row of dabs, each an **anisotropic 2D Gaussian**. Every mark you see is made of splats.

It was written so a language model can paint with its own strokes. The model writes a program; one run of it builds a flower, paints it once and saves the flower on its canvas; the model looks at the canvas and changes the program. The same flower always paints the same pixels.

This file is for painting with it, and ends with the full API. [DESIGN.md](DESIGN.md) is how the engine works, for whoever changes it; [CHANGELOG.md](CHANGELOG.md) is what changed between versions.

![A yellow chrysanthemum at dusk](examples/kiku.png)

`examples/kiku.flower.json` holds its 629 petals in 97 KB (23 KB gzipped). Painted, they become 87,114 dabs, which take 13 MB as a coat. The petals are the part that has to fit in a context window; the dabs never do.

## Install

```bash
pip install .               # builds the engine (needs a Rust toolchain)
uv sync                     # for development: the package, pytest, numpy and Pillow
```

## Paint

```python
import paintcode as pc

flower = {"width": 400, "height": 200, "background": "#f4efe6", "seed": 1,
          "petals": [{"brush": "nib", "color": "#223", "points": [[20, 100], [80, 140], [140, 90]]},
                     {"brush": "filbert", "size": 12, "color": [200, 120, 40],
                      "points": [[20, 170, 0.3], [200, 165, 1], [380, 175, 0.2]]}]}

canvas = pc.flower_to_canvas(flower=flower, supersample=2)
pc.save_canvas(canvas=pc.flower_on_canvas(flower=flower, canvas=canvas), pathname="sketch")  # sketch.flower.png
```

A painter works in versions. The one `.flower.png` is both the picture to look at and the record to go on from: to keep working, load the flower from it, append petals and paint again.

```python
flower = pc.load_flower("sketch.flower.png")
flower["petals"].append({"brush": "ink", "color": "#223", "points": [[60, 40], [340, 60]]})
canvas = pc.flower_to_canvas(flower=flower, supersample=2)
pc.save_canvas(canvas=pc.flower_on_canvas(flower=flower, canvas=canvas), pathname="sketch")
```

The flower is all there is to keep: the coat and the canvas come from it.

## Flower format

```json
{
  "width": 640, "height": 640, "background": "#2a2f4a", "seed": 24,
  "brushes": { "soft": { "brush": "filbert", "size": 8.5 } },
  "petals": [
    { "brush": "soft", "color": [234, 164, 52],
      "points": [[330, 270, 0.3], [380, 240, 1.0], [420, 250, 0.45]] },
    { "brush": "ink", "color": "#e8d9b0", "size": 1.4, "flow": 0.3,
      "points": [[340, 268], [400, 246]] },
    { "brush": "blender", "color": null, "points": [[300, 300], [360, 300]] }
  ]
}
```

- **points** are `[x, y]`, `[x, y, pressure]` or `[x, y, pressure, tilt]`, with pressure and tilt in 0..1 (defaults 1 and 0). Coordinates are floats.
- **brush** is a name from `paintcode.BRUSHES`, a name from `brushes`, or an inline object `{"brush": ..., ...}`. Any brush field can also be overridden directly on the petal.
- **color** is `[r, g, b]` (0..255) or `"#rrggbb"`. `null` loads the brush with whatever is under its first touch, which makes it a smudge.
- **smooth** (default `true`) runs a centripetal Catmull-Rom spline through the points; `false` keeps straight segments.
- **id** (optional) names the petal, once in the flower. Its jitter is keyed by the name, so taking out, adding or reordering other petals never changes how it lies.
- **group** (optional) names the petals it belongs with, such as `"sky"`. A petal in a group, and without an id, is keyed by the group and its place among them, so only a change in its own group can move its jitter.

Jitter is counter-based, so editing or lengthening one petal never moves the jitter of another, and appending petals leaves the ones before them alone. A petal without an `id` or a `group` is keyed by its index: taking one out of the middle moves the jitter of the unnamed petals after it.

## Brushes

| field | meaning |
|---|---|
| `size` | visible radius in px at full pressure; the same size looks the same at every hardness |
| `aspect` | minor / major axis ratio |
| `angle` | `null`: the major axis lies across the petal and turns with it; a number: fixed nib angle in degrees |
| `hardness` | 0 is a Gaussian, 0.9 is nearly a hard ellipse; hard brushes alias, so paint them with `supersample=2` or 3 |
| `flow` | opacity built up at the default spacing |
| `spacing` | dab distance as a fraction of the footprint; flow is normalised to it, so this only trades speed for smoothness |
| `min_size`, `size_gamma` | radius $= \text{size}\cdot\text{lerp}(\text{min\_size}, 1, p^{\gamma})$ |
| `min_flow`, `flow_gamma` | the same curve for flow |
| `tilt_stretch` | tilt 1 stretches the major axis by $1 + \text{tilt\_stretch}$ and thins the film |
| `size_jitter`, `angle_jitter`, `pos_jitter`, `flow_jitter` | uniform noise from the canvas seed |
| `load`, `half_life`, `min_load` | paint on the brush, and how many px of travel it takes to halve |
| `pickup` | share of the wet paint under the brush mixed into the carried colour, per radius of travel; only wet paint, so a brush never picks up the dry background or re-picks its own trail |
| `grain`, `grain_scale` | paper tooth fixed to the canvas, modulating every dab |

Brushes: `ink`, `pencil`, `nib`, `round`, `filbert`, `flat`, `airbrush`, `dry`, `blender`. `paintcode.BRUSHES` holds each one with every field, written as a flower would write it.

![Every brush drawing the same curve](examples/brush_sheet.png)

## Credits

The ideas of paint depletion, colour pickup and short-stroke painting were learned from [paint-mcp](https://github.com/joeyballentine/paint-mcp). No code was taken from it.

## API

The API is functions over data: each noun is a stage of a painting, a function takes one stage to the next, and nothing keeps state between calls. A flower is a dict (anything `json.dumps` takes). `_to_` turns one thing into another, `_on_` and `_off_` put a flower on a canvas and take it off. A function with more than one parameter takes them by keyword only. `help(paintcode._core)` and the type stub `paintcode/_core.pyi` say the same as this section. There is no command line; the package has no dependencies of its own.

Paint:

| call | gives |
|---|---|
| `flower_to_canvas(flower=, scale=1.0, supersample=1)` | the canvas; `scale` makes coordinates and brush sizes larger together, `supersample` paints at N times the size and box-filters down |
| `flower_to_coat(flower=, scale=1.0, supersample=1)` | every dab it lays, in order |
| `coat_to_canvas(coat)` | the same canvas, bit for bit, as painting the flower |
| `flower_on_canvas(flower=, canvas=)` | the canvas with the flower on it, replacing any flower that was |
| `flower_off_canvas(canvas)` | the flower on it; a canvas without one is an error |

Files: a `load_` takes the path of a file and tells formats apart by its bytes, not its name; a `save_` takes a pathname (the path without a suffix), adds the suffix of what it writes and returns the path written. Paths are a `str` or an `os.PathLike`.

| call | reads or writes |
|---|---|
| `load_flower(path)` | a `.flower.json`, or the flower on a `.flower.png`; keys in their written order |
| `save_flower(flower=, pathname=)` | `pathname.flower.json`, one petal per line |
| `load_canvas(path)` | any PNG, only its pixels: grey, palette, alpha and 16-bit become 8-bit RGB |
| `save_canvas(canvas=, pathname=)` | `pathname.png`, or `pathname.flower.png` with its flower in a compressed iTXt chunk |
| `load_coat(path)` | a `.coat.ply`; a 3DGS file from elsewhere is refused |
| `save_coat(coat=, pathname=)` | `pathname.coat.ply`: every dab as a flat Gaussian in the 3D Gaussian Splatting layout, which splat viewers open, and kept exactly |

`Canvas` has `width` and `height`, `peek(x=, y=, width=, height=)`, the colour of a rectangle as `(r, g, b)`, each channel's median, and `crop(x=, y=, width=, height=)`, a rectangle as a canvas of its own (the flower stays behind). `Coat` has `width`, `height`, `supersample` and `len()`, its number of dabs. `BRUSHES` maps each brush name to every field.

A mistake raises `ValueError` with a message that names the petal, the field or the file, or the size of a canvas past its limit (2^28 pixels); only memory that is not there raises `MemoryError`.
