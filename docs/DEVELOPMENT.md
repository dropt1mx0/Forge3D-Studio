# Development Guide

## Setup and Daily Workflow

### 1. Check Toolchain
```bash
rustup show
cargo --version
```

### 2. Format and Lint
Always format code and run clippy before committing:
```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

### 3. Run Tests
```bash
cargo test --workspace
```

### 4. Run Studio Application
```bash
cargo run -p forge3d-studio
```

## Adding a New Primitive or Geometry Feature
1. Implement the generator in `crates/forge-geometry/src/primitives/`.
2. Add validation invariants (no NaNs, no degenerates) in `crates/forge-geometry/src/validation.rs`.
3. Add unit tests covering topology and boundary conditions.
4. Expose the primitive in `crates/forge-scene/src/scene.rs`.
5. Connect menu/button in `apps/forge3d-studio/src/ui/menu_bar.rs`.
