//! Writes the two files that are made from the code, so that neither is edited by hand or falls
//! behind it:
//!
//! - `python/paintcode/_core.pyi`, the type stub of `paintcode._core`, from the `gen_stub`
//!   attributes and doc comments in `src/python.rs`;
//! - `README.md`, which is `README.template.md` with that stub in place of its marker line.
//!
//! ```bash
//! cargo run --bin stub_gen --features python
//! ```
//!
//! CI runs it and fails when either file differs from what is committed.

use std::error::Error;
use std::fs;

/// The line of `README.template.md` that the stub takes the place of.
const MARK: &str = "<!-- python/paintcode/_core.pyi -->\n";

fn main() -> Result<(), Box<dyn Error>> {
    let stub = paintcode::stub_info()?;
    let root = stub
        .pyproject_dir
        .clone()
        .ok_or("the stub info has no pyproject.toml")?;
    let module = stub
        .modules
        .get("paintcode._core")
        .ok_or("the stub info has no paintcode._core")?;
    // one file beside paintcode/__init__.py, not the paintcode/_core/__init__.pyi that
    // StubInfo::generate writes for a module in a mixed layout
    let pyi = module.format_with_config(stub.config.use_type_statement);
    let pyi_path = root.join("python/paintcode/_core.pyi");
    fs::write(&pyi_path, &pyi)?;

    let template = fs::read_to_string(root.join("README.template.md"))?;
    if template.matches(MARK).count() != 1 {
        return Err(format!("README.template.md must hold the line {MARK:?} once").into());
    }
    let readme = format!(
        "<!-- Written by src/bin/stub_gen.rs from README.template.md and the stub; change those. -->\n{}",
        template.replace(MARK, &format!("```python\n{pyi}```\n"))
    );
    let readme_path = root.join("README.md");
    fs::write(&readme_path, readme)?;

    println!("wrote {}", pyi_path.display());
    println!("wrote {}", readme_path.display());
    Ok(())
}
