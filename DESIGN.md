# paintcode, inside

For whoever changes the engine. The [README](README.md) is for whoever uses it, and [CHANGELOG.md](CHANGELOG.md) says what changed between versions.

## The pyramid

The nouns are the stages of a painting, each made from the one above it, kept in the order of a storage pyramid: small and written by hand at the top, large and derived at the bottom.

| one | many | stage | made by |
|---|---|---|---|
| petal | flower | what you write | the painter's program |
| dab | coat | what painting lays | `flower_to_coat` |
| pixel | canvas | what you see | `flower_to_canvas`, `coat_to_canvas` |

Every name in the API is one of these six. One is a single thing, many is the collection; a noun with no place in the pyramid does not belong in the API. Laid out this way, a noun that spans two stages shows itself: `Painting` (0.7) held a flower and a wet canvas at once, and it went in 0.8.

Painting is an event loop. The painter (a language model, usually) writes a program; one run of it writes a flower, paints it once and saves the flower on its canvas; between runs the painter looks at the picture. The painter's context holds the program and the picture, never the flower itself. So nothing in the engine keeps state between calls: the flower is the record, and the coat and the canvas can always be made again from it. Files are blobs that hold a stage; their formats may change, the nouns do not.

## The dab

Each dab is

$$\alpha(\mathbf{x}) = \text{flow}\cdot\exp\!\left(-\tfrac12\, q^{\,n}\right),\qquad n = \frac{1}{1-h},\qquad q = \left(\tfrac{u}{\sigma_x}\right)^2 + \left(\tfrac{v}{\sigma_y}\right)^2$$

where $(u, v)$ is the pixel offset rotated into the dab's frame and $h$ is hardness. The profile reaches $e^{-2}$ at $\sigma\, 2^{1/n}$, so the engine sets $\sigma = \text{radius}\cdot 2^{-1/n}$: the visible edge stays at the brush radius whatever the hardness, with $\sigma = r/2$ for a Gaussian and $\sigma = r$ for a hard ellipse. With $h = 0$ it is exactly a 2D Gaussian; as $h \to 1$ it becomes a hard-edged ellipse one sigma across. Sigmas below half a pixel are raised to 0.5 px, dabs are cut off at $q > 9$, and they are composited with plain "over". Dab geometry is computed in float64; the per-pixel kernel and the compositing are float32.

A Gaussian is a low-pass filter, so a petal made of Gaussians has soft edges by nature; hardness is the cure. A petal of size 16 keeps a half-coverage width of 26 to 30 px from hardness 0 to 0.95, while its 10–90% edge narrows from about 14 px to 1 px. Hard brushes alias, which is what `supersample` is for.

The canvas keeps a wetness map. Every dab makes the area under it wetter, and `pickup` only takes colour from wet paint, read from a snapshot taken when the petal starts. So a brush never picks up the dry background, never re-picks its own trail, and the result does not depend on dab spacing.

Jitter is counter-based: every random number is a hash of (seed, petal key, dab index, slot). A petal's key is its index, or its `id`, or its `group` and its place in the group.

## The coat as Gaussian splats

`.coat.ply` writes one Gaussian per dab: position on the image plane (x right, y down, the longer side spanning $[-1, 1]$), scales $\log\sigma$ with a vanishing third axis, a quaternion for the dab angle about $z$, colour as the SH DC term, opacity as a logit. Paint order becomes depth: the first dab is farthest and the last is nearest to a camera on the $-z$ side, so depth-sorted splatting composites them in the order they were painted. The stack is only 0.05 deep, so this holds facing the plane: turned away, the splats sort by where they sit instead of when they were painted, and the wide early sky petals drift in front of the flower. After the 3DGS fields come `pc_*` fields that keep each dab exactly (position, sigmas, angle, opacity, hardness and grain as doubles, its colour as floats), and a `paintcode` header comment keeps the size, supersample, seed and background, so `load_coat` gives back a coat that paints the same canvas bit for bit.

The tests carry a separate, plain rasteriser that reads only the 3DGS fields of the PLY. For a painting made with hardness 0 and no grain it reproduces the canvas to within 2/255 (`a_soft_painting_round_trips_through_the_ply` in `crates/paintcode/src/tests/splats.rs`). The 3DGS kernel has no hardness or grain, so other paintings come back softer:

![The painting, and the same dabs rendered back from the PLY](examples/png_vs_ply.png)

## The crate

The engine is Rust: one crate, `crates/paintcode`, with one module per part of the engine. Built with its `python` feature it is the Python package's `paintcode._core`; `python/paintcode/` only re-exports it and carries the type stub `_core.pyi`, which is written by hand and checked by a test. The public Rust API is what Python can do and no more (with the `profile` timers for the benches): the modules are private, and the tests that reach inside them live inside the crate, in `src/tests/`. Dependencies point one way, from the leaf up, and `tests/layering.rs` fails if a module reaches back down the list.

| module | part | may use | does |
|---|---|---|---|
| `error` | leaf | nothing | every way paintcode fails, as one enum |
| `profile` | leaf | nothing | stage timers, empty unless built with the `profile` feature |
| `math` | leaf | nothing | counter-based RNG, dab profile and geometry |
| `flower` | bones | error, math | flower, petal and brush types, brushes, colours |
| `render` | render | error, math | dabs to pixels, coats, paper grain, downsampling |
| `petal` | flesh | profile, math, flower, render | control points to dabs, carrying paint |
| `io` | files | error, flower, render | encoders and decoders, in pairs: flower JSON, PNG, PLY |
| `python` | face | everything | `paintcode._core`, only with the `python` feature |

Errors are one `thiserror` enum. Clippy denies `unwrap`, `panic`, `todo` and `unimplemented` outside tests; an `expect` says why it cannot fail.

## Painting fast

A petal is painted in two parallel phases: every dab's kernel and the wet paint under it are evaluated independently (pickup reads the canvas as it was when the petal began), then the dabs are composited in order, one band of rows per thread, eight pixels at a time. While one petal is painted, the next is planned. The fast path also skips the parts of each dab's square that lie outside its $q \le 9$ ellipse. None of this changes a bit: the tests render with the reference path (whole squares, one thread) and require identical output. The profile is read from a table per hardness, 8192 steps over $0 \le q \le 9$ with linear interpolation, instead of `powf` and `exp` on every pixel: at most 1.1e-4 off the formula, for the hardest brush.

kiku, 640², supersample 2 (87,110 dabs), from `cargo bench` on an M2 Pro:

| threads | time | dabs per second |
|---|---|---|
| 1 | 1.83 s | 47.6 k |
| 10 | 0.48 s | 182 k |

On the 2-core VM where the numpy engine of 0.4 took 52 s, the Rust core took 3.5 s on both cores.

```bash
cargo bench -p paintcode                                    # the example flowers on one core and on all, compared with the last run
cargo bench -p paintcode --features profile --bench stages  # where one paint's time goes, stage by stage
```

## Tests

```bash
cargo test --release -p paintcode -- --include-ignored   # the engine: behaviour, golden snapshots, coats, fast path = reference path, layering
uv run pytest                                            # the Python face: arguments, files, errors
```

The engine tests write the golden snapshots in `tests/golden/` again every run, and git shows what moved; CI fails when a run changes them, so a change that moves pixels commits them with it. `.gitattributes` gives the PNGs a `png` diff driver, and one line points it at an example in this repo, so `git diff` says how many values of each image moved and by how much instead of its size:

```bash
git config diff.png.command 'cargo run -q --release -p paintcode --example diff --'
```

Up to 0.4 a numpy engine in this package was the reference, and the core was checked against it at 1e-5 on the float canvas (2.4e-7 at worst on kiku). 0.5 retired it: its output for the examples and a feature matrix was frozen in `tests/golden/` (made by `tests/golden/make.py`, which `git show 02f2aea:tests/golden/make.py` has), and 0.7 painted the snapshots anew.
