<!-- stub_gen: written from README.template.md and crates/paintcode/src/python.rs; change those, not this file -->
# paintcode

A small dab engine for painting from code.

![A yellow chrysanthemum at dusk](examples/kiku.png)

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

The chrysanthemum above is `examples/kiku.flower.json`: 629 petals in 97 KB (23 KB gzipped). Painted, they become 87,114 dabs, which take 13 MB as a coat. The petals are the part that has to fit in a context window; the dabs never do.

This file is for painting with it and ends with the API. [DESIGN.md](DESIGN.md) is how the engine works, for whoever changes it; [CHANGELOG.md](CHANGELOG.md) is what changed between versions.

## Install

```bash
uv add ./paintcode          # for use, from a clone: builds the engine (needs rustup)
uv sync                     # for development, in the clone: the package, pytest, numpy and Pillow
```

The package has no dependencies of its own, and no command line.

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

The ideas of paint depletion, colour pickup and short-stroke painting were learned from [paint-mcp](https://github.com/joeyballentine/paint-mcp). No code was taken from it.

## API

The type stub, [`python/paintcode/_core.pyi`](python/paintcode/_core.pyi):

```python
# This file is automatically generated by pyo3_stub_gen
# ruff: noqa: E501, F401, F403, F405
r"""
The paintcode engine, in Rust: functions over data. Each noun is a stage of a painting, a
function takes one stage to the next, and nothing keeps state between calls. _to_ turns one
thing into another; _on_ and _off_ put a flower on a canvas and take it off.

A load_ takes the path of a file and tells formats apart by its bytes, not its name; a save_
takes a pathname, the path without a suffix, adds the suffix of what it writes and returns the
path written. A function with more than one parameter takes them by keyword only.

A mistake raises ValueError with a message that names the petal, the field or the file, or the
size of a canvas past its limit (2^28 pixels); only memory that is not there raises MemoryError.
"""

import builtins
import os
import pathlib
import typing
from typing import TypeAlias
__all__ = [
    "BRUSHES",
    "Canvas",
    "Coat",
    "Flower",
    "coat_to_canvas",
    "flower_off_canvas",
    "flower_on_canvas",
    "flower_to_canvas",
    "flower_to_coat",
    "load_canvas",
    "load_coat",
    "load_flower",
    "save_canvas",
    "save_coat",
    "save_flower",
]

Flower: TypeAlias = builtins.dict[builtins.str, typing.Any]
r"""
A flower, as a dict: anything json.dumps takes.
"""

BRUSHES: builtins.dict[builtins.str, builtins.dict[builtins.str, typing.Any]]
__version__: builtins.str
@typing.final
class Canvas:
    r"""
    The pixels you see, 8-bit RGB, and the flower on them if one was put there.
    """
    @property
    def width(self) -> builtins.int: ...
    @property
    def height(self) -> builtins.int: ...
    def peek(self, *, x: builtins.int, y: builtins.int, width: builtins.int, height: builtins.int) -> tuple[builtins.int, builtins.int, builtins.int]:
        r"""
        The colour of a rectangle as (r, g, b), 0..255: each channel's median over its pixels, the
        upper one when they are even in number. A median, not a mean: on the edge of a stroke a
        mean is a mix that is painted nowhere, the median is the colour most of the rectangle has.
        """
    def crop(self, *, x: builtins.int, y: builtins.int, width: builtins.int, height: builtins.int) -> Canvas:
        r"""
        A rectangle of this canvas as a canvas of its own; this one is left as it was. The flower
        stays behind: it painted the whole canvas, not this part.
        """
    def __repr__(self) -> builtins.str: ...

@typing.final
class Coat:
    r"""
    Every dab a flower laid, in order: what coat_to_canvas lays again. len() counts the dabs.
    """
    @property
    def width(self) -> builtins.int:
        r"""
        The canvas it makes, in pixels.
        """
    @property
    def height(self) -> builtins.int: ...
    @property
    def supersample(self) -> builtins.int: ...
    def __len__(self) -> builtins.int:
        r"""
        The number of dabs.
        """
    def __repr__(self) -> builtins.str: ...

def coat_to_canvas(coat: Coat) -> Canvas:
    r"""
    Lay a coat's dabs again: the same canvas, bit for bit, as painting its flower.
    """

def flower_off_canvas(canvas: Canvas) -> Flower:
    r"""
    The flower on a canvas. A canvas with no flower on it is an error.
    """

def flower_on_canvas(*, flower: Flower, canvas: Canvas) -> Canvas:
    r"""
    The canvas with a flower on it, which save_canvas writes into the PNG. Any flower that was on
    it is replaced.
    """

def flower_to_canvas(*, flower: Flower, scale: builtins.float = 1.0, supersample: builtins.int = 1) -> Canvas:
    r"""
    Paint a flower: its canvas.
    
    scale paints larger: coordinates and brush sizes scale together.
    supersample paints at N times the size and box-filters down; 2 or 3 give crisp hard edges.
    """

def flower_to_coat(*, flower: Flower, scale: builtins.float = 1.0, supersample: builtins.int = 1) -> Coat:
    r"""
    Paint a flower and keep every dab it lays: its coat. Options as flower_to_canvas.
    """

def load_canvas(path: builtins.str | os.PathLike | pathlib.Path) -> Canvas:
    r"""
    The canvas in a PNG: only the pixels, whether or not a flower is on it (load_flower takes the
    flower). A flower that is there but damaged is an error. Any PNG opens: grey, palette, alpha
    and 16-bit images become 8-bit RGB.
    """

def load_coat(path: builtins.str | os.PathLike | pathlib.Path) -> Coat:
    r"""
    The coat in a .coat.ply. A 3DGS file from elsewhere is not a coat and is refused.
    """

def load_flower(path: builtins.str | os.PathLike | pathlib.Path) -> Flower:
    r"""
    The flower in a .flower.json, or on a .flower.png; keys in their written order. A PNG with no
    flower on it is an error.
    """

def save_canvas(*, canvas: Canvas, pathname: builtins.str | os.PathLike | pathlib.Path) -> pathlib.Path:
    r"""
    Write a canvas as a PNG at pathname.png, or with its flower on it at pathname.flower.png.
    """

def save_coat(*, coat: Coat, pathname: builtins.str | os.PathLike | pathlib.Path) -> pathlib.Path:
    r"""
    Write a coat at pathname.coat.ply, a binary PLY in the 3D Gaussian Splatting layout (SH degree
    0), which any splat viewer opens: paint order becomes depth, so the dabs composite in the order
    they were laid when looking at the plane. Every dab is also kept exactly, so load_coat gives the
    same coat back.
    """

def save_flower(*, flower: Flower, pathname: builtins.str | os.PathLike | pathlib.Path) -> pathlib.Path:
    r"""
    Write a flower as JSON, one petal per line, at pathname.flower.json. A flower the engine would
    refuse is refused.
    """

```
