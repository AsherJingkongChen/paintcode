set shell := ['/usr/bin/env', 'bash', '-cueo', 'pipefail']

# Everything CI runs. It writes the golden snapshots, the type stub and the README again: commit what it wrote.
all: check test stub

# Check the formatting, and lint: warnings are errors; no unwrap, panic, todo or unimplemented outside tests.
check:
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --all-features -- --deny warnings

# Fix the formatting, then check.
check-fix:
  cargo fmt --all
  just check

# Test the engine (behaviour, golden snapshots, fast path = reference path, layering), then the Python module built from uv.lock.
test:
  cargo test --release -p paintcode -- --include-ignored
  uv sync --locked
  uv run pytest -q

# Write the type stub and the README from the binding (src/bin/stub_gen.rs).
stub:
  uv run cargo run --bin stub_gen --features python
