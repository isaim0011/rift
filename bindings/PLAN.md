# Deliverable 5: Node/Bun (`napi-rs`) and Python (`PyO3`) Bindings Plan

## Goal
Build complete, spec-compliant, high-performance bindings for Node/Bun (`napi-rs`) and Python (`PyO3`) exposing Rift's core primitives, REST rate limiting, signature verification, state cache, Gateway v10 state machine, and async client.

## Architecture & Dual-Mode Strategy
Both `bindings/node` and `bindings/python` are organized as standalone Cargo packages that can be built and tested in two distinct modes:

1. **Pure Rust / Standalone Verification Mode** (`--no-default-features`):
   - Compiles pure Rust implementations with zero external native dependency requirements (no `python3.dll` linking needed, no `@napi-rs/cli` requirement).
   - Allows running exhaustive Rust tests via:
     `cargo test --manifest-path bindings/node/Cargo.toml --no-default-features --target-dir ../../target`
     `cargo test --manifest-path bindings/python/Cargo.toml --no-default-features --target-dir ../../target`
   - Shares the workspace `target/` directory via `--target-dir ../../target` to prevent disk bloat.

2. **Native Language Bridge Mode**:
   - **Node/Bun**: Built via `@napi-rs/cli` or Cargo with feature `napi-binding`. Includes platform binary targets, TS declarations (`index.d.ts`), and JS loader (`index.js`).
   - **Python**: Built via `maturin develop` or `maturin build` with feature `extension-module`. Includes PEP 561 typed package `rift` with `__init__.py`, `__init__.pyi`, and `py.typed`.

## File Layout
```
bindings/
├── PLAN.md
├── node/
│   ├── Cargo.toml
│   ├── package.json
│   ├── index.js
│   ├── index.d.ts
│   ├── src/
│   │   ├── lib.rs
│   │   ├── snowflake.rs
│   │   ├── permissions.rs
│   │   ├── intents.rs
│   │   ├── verify.rs
│   │   ├── limiter.rs
│   │   ├── cache.rs
│   │   ├── gateway.rs
│   │   └── client.rs
│   └── test/
│       └── node_binding.test.js
└── python/
    ├── Cargo.toml
    ├── pyproject.toml
    ├── src/
    │   ├── lib.rs
    │   ├── snowflake.rs
    │   ├── permissions.rs
    │   ├── intents.rs
    │   ├── verify.rs
    │   ├── limiter.rs
    │   ├── cache.rs
    │   ├── gateway.rs
    │   └── client.rs
    ├── python/
    │   └── rift/
    │       ├── __init__.py
    │       ├── __init__.pyi
    │       └── py.typed
    └── tests/
        └── test_rift.py
```

## Step-by-Step Implementation
1. **Node Bindings (`bindings/node`)**:
   - `Cargo.toml`: Package `rift-node`, cdylib + rlib, referencing `../../crates/rift-*`.
   - Rust modules: `snowflake`, `permissions`, `intents`, `verify`, `limiter`, `cache`, `gateway`, `client`.
   - Comprehensive unit tests in each module validating functionality directly in Rust.
   - `package.json`: N-API CLI scripts, binary triples, exports.
   - `index.d.ts`: Full TypeScript signatures.
   - `index.js`: Robust binary loader and fallback.
   - `test/node_binding.test.js`: Node test suite.

2. **Python Bindings (`bindings/python`)**:
   - `Cargo.toml`: Package `rift-py`, cdylib + rlib, referencing `../../crates/rift-*`.
   - `pyproject.toml`: Maturin build system configuration.
   - Rust modules: `PySnowflake`, `PyPermissions`, `PyRateLimiter`, `PyCache`, `PyGatewaySession`, `verify_interaction`, `normalize_route`, `PyRiftClient`.
   - Feature gating: `default = ["extension-module"]`, `extension-module = ["dep:pyo3", "pyo3/extension-module"]`. Pure Rust unit tests run with `--no-default-features`.
   - `python/rift/__init__.py`, `python/rift/__init__.pyi`, `python/rift/py.typed`.
   - `tests/test_rift.py`: Python pytest test suite.

3. **Verification**:
   - Run `cargo test --manifest-path bindings/node/Cargo.toml --no-default-features --target-dir ../../target`.
   - Run `cargo test --manifest-path bindings/python/Cargo.toml --no-default-features --target-dir ../../target`.
   - If node and python are installed, test native loading and JS/Python tests.
   - Verify zero errors, zero warnings.

## Risks & Mitigations
- **Windows Python linker issues**: On Windows, linking against `python3.dll` in `cargo test` without maturin can fail if Python import libs are missing from `LIB`. The `--no-default-features` flag cleanly skips PyO3 C extension linkage while still testing 100% of the Rust wrapper logic and API surfaces!
- **N-API dynamic loading**: Provide both native addon loading and a clean, zero-dependency pure-Rust bridge.
- **Disk space & target duplication**: Strictly pass `--target-dir ../../target` for all cargo invocations.
