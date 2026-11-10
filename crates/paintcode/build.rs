//! With the `python` feature, a binary or test that links libpython (`src/bin/stub_gen.rs`) is told
//! where that library is, so it runs without LD_LIBRARY_PATH. pyo3 skips this for the module
//! maturin builds, which links no libpython, so no wheel carries the path.

fn main() {
    #[cfg(feature = "python")]
    {
        pyo3_build_config::add_libpython_rpath_link_args();
        pyo3_build_config::add_python_framework_link_args();
    }
}
