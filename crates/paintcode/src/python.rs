//! `paintcode._core`: the engine, seen from Python. This is the whole Python package; the files
//! in `python/paintcode/` only re-export it.
//!
//! What Python reads is written here once: [`DOC`] is the module's docstring, and each doc comment
//! on a function or class below is its docstring. The `gen_stub` attributes turn the same
//! signatures and docs into the type stub, `python/paintcode/_core.pyi`, which
//! `src/bin/stub_gen.rs` writes, with the README that ends in it.
//!
//! A flower is a dict of dicts, lists, str, int, float, bool and None. The files are read and
//! written here, in Rust.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::flower::Potpourri;
use crate::{Error, RenderOpts, io};
use pyo3::exceptions::{PyMemoryError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};
use pyo3_stub_gen::runtime::PyModuleTypeAliasExt;

/// The module's docstring, at run time and in the stub.
const DOC: &str = "\
The paintcode engine, in Rust: functions over data. Each noun is a stage of a painting, a
function takes one stage to the next, and nothing keeps state between calls. _to_ turns one
thing into another; _on_ and _off_ put a flower on a canvas and take it off.

A load_ takes the path of a file and tells formats apart by its bytes, not its name; a save_
takes a pathname, the path without a suffix, adds the suffix of what it writes and returns the
path written. A function with more than one parameter takes them by keyword only.

A mistake raises ValueError with a message that names the petal, the field or the file, or the
size of a canvas past its limit (2^28 pixels); only memory that is not there raises MemoryError.";

pyo3_stub_gen::module_doc!("paintcode._core", "{}", DOC);

pyo3_stub_gen::type_alias!(
    "paintcode._core",
    Flower = HashMap<String, Py<PyAny>>,
    "A flower, as a dict of dicts, lists, str, int, float, bool and None, every key written."
);

pyo3_stub_gen::module_variable!(
    "paintcode._core",
    "BRUSHES",
    HashMap<String, HashMap<String, Py<PyAny>>>
);
pyo3_stub_gen::module_variable!("paintcode._core", "__version__", String);

/// The type stub of `paintcode._core`, for `src/bin/stub_gen.rs`. It reads the pyproject.toml at
/// the root of the repository, two levels above this crate, not the one beside its Cargo.toml
/// where `define_stub_info_gatherer!` would look.
pub fn stub_info() -> pyo3_stub_gen::Result<pyo3_stub_gen::StubInfo> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    pyo3_stub_gen::StubInfo::from_pyproject_toml(root.join("pyproject.toml"))
}

/// A Python exception for an engine error: MemoryError when the memory is not there, ValueError
/// for everything a caller can fix, with the engine's message.
fn value_error(e: Error) -> PyErr {
    match e {
        Error::Memory { .. } => PyMemoryError::new_err(e.to_string()),
        _ => PyValueError::new_err(e.to_string()),
    }
}

/// Write `bytes` at `pathname` plus `suffix`, and give back the path written.
fn write(pathname: PathBuf, suffix: &str, bytes: &[u8]) -> PyResult<PathBuf> {
    let mut path = pathname.into_os_string();
    path.push(suffix);
    let path = PathBuf::from(path);
    std::fs::write(&path, bytes)?;
    Ok(path)
}

/// [`value_error`], with the path of the file in front of the message.
fn file_error(path: &Path, e: Error) -> PyErr {
    let message = format!("{}: {e}", path.display());
    match e {
        Error::Memory { .. } => PyMemoryError::new_err(message),
        _ => PyValueError::new_err(message),
    }
}

/// A flower from its dict, checked: what the engine would refuse is refused here, before any file
/// is written or any pixel is painted.
fn to_flower(flower: &Bound<'_, PyAny>) -> PyResult<crate::Flower> {
    crate::Flower::from_potpourri(&to_potpourri(flower)?).map_err(value_error)
}

/// A Python value as a [`Potpourri`]: dicts (str keys, in their order), lists and tuples, str, int,
/// float, bool and None. Anything else is no part of a flower.
fn to_potpourri(v: &Bound<'_, PyAny>) -> PyResult<Potpourri> {
    if v.is_none() {
        return Ok(Potpourri::Null);
    }
    if let Ok(b) = v.cast::<PyBool>() {
        return Ok(Potpourri::Bool(b.is_true()));
    }
    if let Ok(i) = v.cast::<PyInt>() {
        // a seed may use all 64 bits; it is hashed as a u64
        return Ok(match i.extract::<i64>() {
            Ok(n) => Potpourri::Int(n),
            Err(_) => match i.extract::<u64>() {
                Ok(u) => Potpourri::Int(u as i64),
                Err(_) => Potpourri::Num(i.extract::<f64>()?),
            },
        });
    }
    if let Ok(f) = v.cast::<PyFloat>() {
        return Ok(Potpourri::Num(f.value()));
    }
    if let Ok(s) = v.cast::<PyString>() {
        return Ok(Potpourri::Str(s.to_str()?.to_owned()));
    }
    if let Ok(d) = v.cast::<PyDict>() {
        let mut m = Vec::with_capacity(d.len());
        for (k, val) in d.iter() {
            let k = k
                .cast::<PyString>()
                .map_err(|_| PyValueError::new_err(format!("a key must be a str, got {k}")))?
                .to_str()?
                .to_owned();
            m.push((k, to_potpourri(&val)?));
        }
        return Ok(Potpourri::Map(m));
    }
    if v.cast::<PyList>().is_ok() || v.cast::<PyTuple>().is_ok() {
        return Ok(Potpourri::List(
            v.try_iter()?
                .map(|x| to_potpourri(&x?))
                .collect::<PyResult<_>>()?,
        ));
    }
    Err(PyValueError::new_err(format!(
        "a flower holds dicts, lists, str, int, float, bool and None, not {}",
        v.get_type().name()?
    )))
}

/// A [`Potpourri`] as Python values: the inverse of [`to_potpourri`].
fn to_dict<'py>(py: Python<'py>, v: &Potpourri) -> PyResult<Bound<'py, PyAny>> {
    Ok(match v {
        Potpourri::Null => py.None().into_bound(py),
        Potpourri::Bool(b) => PyBool::new(py, *b).to_owned().into_any(),
        Potpourri::Int(n) => n.into_pyobject(py)?.into_any(),
        Potpourri::Num(f) => PyFloat::new(py, *f).into_any(),
        Potpourri::Str(s) => PyString::new(py, s).into_any(),
        Potpourri::List(l) => {
            let items: Vec<Bound<'py, PyAny>> =
                l.iter().map(|x| to_dict(py, x)).collect::<PyResult<_>>()?;
            PyList::new(py, items)?.into_any()
        }
        Potpourri::Map(m) => {
            let d = PyDict::new(py);
            for (k, x) in m {
                d.set_item(k, to_dict(py, x)?)?;
            }
            d.into_any()
        }
    })
}

fn opts(scale: f64, supersample: i64) -> RenderOpts {
    RenderOpts {
        scale,
        supersample,
        ..RenderOpts::default()
    }
}

/// The pixels you see, 8-bit RGB, and the flower on them if one was put there.
#[gen_stub_pyclass(module = "paintcode._core")]
#[pyclass(frozen, module = "paintcode")]
struct Canvas(crate::Canvas);

#[gen_stub_pymethods]
#[pymethods]
impl Canvas {
    #[getter]
    fn width(&self) -> usize {
        self.0.width
    }

    #[getter]
    fn height(&self) -> usize {
        self.0.height
    }

    /// The colour of a rectangle as (r, g, b), 0..255: each channel's median over its pixels, the
    /// upper one when they are even in number. A median, not a mean: on the edge of a stroke a
    /// mean is a mix that is painted nowhere, the median is the colour most of the rectangle has.
    #[pyo3(signature = (*, x, y, width, height))]
    fn peek(&self, x: usize, y: usize, width: usize, height: usize) -> PyResult<(u8, u8, u8)> {
        let rows = self.rows(x, y, width, height, "peek")?;
        let mut channels: [Vec<u8>; 3] =
            std::array::from_fn(|_| Vec::with_capacity(width * height));
        for row in rows {
            for px in row.as_chunks::<3>().0 {
                for (c, v) in channels.iter_mut().zip(px) {
                    c.push(*v);
                }
            }
        }
        let [r, g, b] = channels.map(|mut c| {
            let mid = c.len() / 2;
            *c.select_nth_unstable(mid).1
        });
        Ok((r, g, b))
    }

    /// A rectangle of this canvas as a canvas of its own; this one is left as it was. The flower
    /// stays behind: it painted the whole canvas, not this part.
    #[pyo3(signature = (*, x, y, width, height))]
    fn crop(&self, x: usize, y: usize, width: usize, height: usize) -> PyResult<Canvas> {
        let pixels = self.rows(x, y, width, height, "crop")?.concat();
        Ok(Canvas(crate::Canvas {
            width,
            height,
            pixels,
            flower: None,
        }))
    }

    fn __repr__(&self) -> String {
        let on = if self.0.flower.is_some() {
            ", with its flower"
        } else {
            ""
        };
        format!("<Canvas {}x{}{on}>", self.0.width, self.0.height)
    }
}

impl Canvas {
    /// The rows of a rectangle, each `width * 3` bytes, or a ValueError naming `what` when the
    /// rectangle is empty or does not fit.
    fn rows(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        what: &str,
    ) -> PyResult<Vec<&[u8]>> {
        let (w, h) = (self.0.width, self.0.height);
        let fits =
            |at: usize, len: usize, side: usize| at.checked_add(len).is_some_and(|e| e <= side);
        if width == 0 || height == 0 || !fits(x, width, w) || !fits(y, height, h) {
            return Err(PyValueError::new_err(format!(
                "a {width}x{height} {what} at ({x}, {y}) does not fit in the {w}x{h} canvas"
            )));
        }
        Ok((y..y + height)
            .map(|row| {
                let i = (row * w + x) * 3;
                &self.0.pixels[i..i + width * 3]
            })
            .collect())
    }
}

/// Every dab a flower laid, in order: what coat_to_canvas lays again. len() counts the dabs.
#[gen_stub_pyclass(module = "paintcode._core")]
#[pyclass(frozen, module = "paintcode")]
struct Coat(crate::Coat);

#[gen_stub_pymethods]
#[pymethods]
impl Coat {
    /// The canvas it makes, in pixels.
    #[getter]
    fn width(&self) -> usize {
        self.0.width
    }

    #[getter]
    fn height(&self) -> usize {
        self.0.height
    }

    #[getter]
    fn supersample(&self) -> usize {
        self.0.supersample
    }

    /// The number of dabs.
    fn __len__(&self) -> usize {
        self.0.dabs.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "<Coat of {} dabs for a {}x{} canvas>",
            self.0.dabs.len(),
            self.0.width,
            self.0.height
        )
    }
}

// ------------------------------------------------------------------------------------- files

/// The flower in a .flower, or on a .flower.png, written out in full: every petal with its whole
/// brush, every colour as numbers. A PNG with no flower on it is an error.
#[gen_stub_pyfunction]
#[pyfunction]
#[gen_stub(override_return_type(type_repr = "Flower"))]
fn load_flower<'py>(py: Python<'py>, path: PathBuf) -> PyResult<Bound<'py, PyAny>> {
    let bytes = std::fs::read(&path)?;
    let flower = io::decode_flower(&bytes).map_err(|e| file_error(&path, e))?;
    to_dict(py, &flower.to_potpourri())
}

/// Write a flower at pathname.flower: bitcode inside zstd. A flower the engine would refuse is
/// refused.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*, flower, pathname))]
fn save_flower(
    #[gen_stub(override_type(type_repr = "Flower"))] flower: &Bound<'_, PyAny>,
    pathname: PathBuf,
) -> PyResult<PathBuf> {
    let flower = to_flower(flower)?;
    write(pathname, ".flower", &io::encode_flower(&flower))
}

/// The canvas in a PNG: only the pixels, whether or not a flower is on it (load_flower takes the
/// flower). A flower that is there but damaged is an error. Any PNG opens: grey, palette, alpha
/// and 16-bit images become 8-bit RGB.
#[gen_stub_pyfunction]
#[pyfunction]
fn load_canvas(path: PathBuf) -> PyResult<Canvas> {
    let bytes = std::fs::read(&path)?;
    let canvas = io::decode_canvas(&bytes).map_err(|e| file_error(&path, e))?;
    Ok(Canvas(crate::Canvas {
        flower: None,
        ..canvas
    }))
}

/// Write a canvas as a PNG at pathname.png, or with its flower on it at pathname.flower.png.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*, canvas, pathname))]
fn save_canvas(py: Python<'_>, canvas: PyRef<'_, Canvas>, pathname: PathBuf) -> PyResult<PathBuf> {
    let c: &Canvas = &canvas;
    let png = py.detach(|| io::encode_canvas(&c.0));
    let suffix = if c.0.flower.is_some() {
        ".flower.png"
    } else {
        ".png"
    };
    write(pathname, suffix, &png)
}

/// The coat in a .coat.ply. A 3DGS file from elsewhere is not a coat and is refused.
#[gen_stub_pyfunction]
#[pyfunction]
fn load_coat(path: PathBuf) -> PyResult<Coat> {
    let bytes = std::fs::read(&path)?;
    io::decode_coat(&bytes)
        .map(Coat)
        .map_err(|e| file_error(&path, e))
}

/// Write a coat at pathname.coat.ply, a binary PLY in the 3D Gaussian Splatting layout (SH degree
/// 0), which any splat viewer opens: paint order becomes depth, so the dabs composite in the order
/// they were laid when looking at the plane. Every dab is also kept exactly, so load_coat gives the
/// same coat back.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*, coat, pathname))]
fn save_coat(py: Python<'_>, coat: PyRef<'_, Coat>, pathname: PathBuf) -> PyResult<PathBuf> {
    let coat: &crate::Coat = &coat.0;
    let ply = py.detach(|| io::encode_coat(coat));
    write(pathname, ".coat.ply", &ply)
}

// ------------------------------------------------------------------------------- transforms

/// Paint a flower: its canvas.
///
/// scale paints larger: coordinates and brush sizes scale together.
/// supersample paints at N times the size and box-filters down; 2 or 3 give crisp hard edges.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*, flower, scale = 1.0, supersample = 1))]
fn flower_to_canvas(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "Flower"))] flower: &Bound<'_, PyAny>,
    scale: f64,
    supersample: i64,
) -> PyResult<Canvas> {
    let flower = to_flower(flower)?;
    let o = opts(scale, supersample);
    let canvas = py
        .detach(|| crate::flower_to_canvas(&flower, &o))
        .map_err(value_error)?;
    Ok(Canvas(canvas))
}

/// Paint a flower and keep every dab it lays: its coat. Options as flower_to_canvas.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*, flower, scale = 1.0, supersample = 1))]
fn flower_to_coat(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "Flower"))] flower: &Bound<'_, PyAny>,
    scale: f64,
    supersample: i64,
) -> PyResult<Coat> {
    let flower = to_flower(flower)?;
    let o = opts(scale, supersample);
    py.detach(|| crate::flower_to_coat(&flower, &o))
        .map(Coat)
        .map_err(value_error)
}

/// Lay a coat's dabs again: the same canvas, bit for bit, as painting its flower.
#[gen_stub_pyfunction]
#[pyfunction]
fn coat_to_canvas(py: Python<'_>, coat: PyRef<'_, Coat>) -> PyResult<Canvas> {
    let coat: &crate::Coat = &coat.0;
    let canvas = py
        .detach(|| crate::coat_to_canvas(coat))
        .map_err(value_error)?;
    Ok(Canvas(canvas))
}

/// The canvas with a flower on it, which save_canvas writes into the PNG. Any flower that was on
/// it is replaced.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (*, flower, canvas))]
fn flower_on_canvas(
    #[gen_stub(override_type(type_repr = "Flower"))] flower: &Bound<'_, PyAny>,
    canvas: PyRef<'_, Canvas>,
) -> PyResult<Canvas> {
    let flower = to_flower(flower)?;
    Ok(Canvas(crate::flower_on_canvas(&flower, canvas.0.clone())))
}

/// The flower on a canvas. A canvas with no flower on it is an error.
#[gen_stub_pyfunction]
#[pyfunction]
#[gen_stub(override_return_type(type_repr = "Flower"))]
fn flower_off_canvas<'py>(
    py: Python<'py>,
    canvas: PyRef<'py, Canvas>,
) -> PyResult<Bound<'py, PyAny>> {
    let flower = crate::flower_off_canvas(&canvas.0).map_err(value_error)?;
    to_dict(py, &flower.to_potpourri())
}

#[pymodule]
#[pyo3(name = "_core")]
fn paintcode_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", DOC)?;
    m.add_type_alias::<Flower>()?;
    m.add_class::<Canvas>()?;
    m.add_class::<Coat>()?;
    m.add_function(wrap_pyfunction!(load_flower, m)?)?;
    m.add_function(wrap_pyfunction!(save_flower, m)?)?;
    m.add_function(wrap_pyfunction!(load_canvas, m)?)?;
    m.add_function(wrap_pyfunction!(save_canvas, m)?)?;
    m.add_function(wrap_pyfunction!(load_coat, m)?)?;
    m.add_function(wrap_pyfunction!(save_coat, m)?)?;
    m.add_function(wrap_pyfunction!(flower_to_canvas, m)?)?;
    m.add_function(wrap_pyfunction!(flower_to_coat, m)?)?;
    m.add_function(wrap_pyfunction!(coat_to_canvas, m)?)?;
    m.add_function(wrap_pyfunction!(flower_on_canvas, m)?)?;
    m.add_function(wrap_pyfunction!(flower_off_canvas, m)?)?;
    // every brush with its fields, as a flower would write them
    let brushes = crate::flower::BRUSH_NAMES
        .iter()
        .map(|n| {
            let b = crate::flower::brush(n).expect("a listed brush");
            (n.to_string(), b.to_potpourri())
        })
        .collect();
    m.add("BRUSHES", to_dict(m.py(), &Potpourri::Map(brushes))?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
