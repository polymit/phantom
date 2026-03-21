# Contributing to Phantom Engine

Phantom Engine is a native Rust browser engine for AI agents. Contributions are welcome — this document explains the standards and process we hold to.

---

## Table of Contents

- [Before You Start](#before-you-start)
- [Architecture Principles](#architecture-principles)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Code Standards](#code-standards)
- [Testing Requirements](#testing-requirements)
- [Reporting Bugs](#reporting-bugs)
- [Security Vulnerabilities](#security-vulnerabilities)
- [Suggesting Features](#suggesting-features)
- [Crate Guide](#crate-guide)

---

## Code of Conduct

This project follows the [Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). By participating, you agree to uphold its terms.

---

## Before You Start

### Understand the architecture

Phantom Engine has deliberate, well-reasoned architectural decisions. Before writing code, read:

1. `README.md` — what Phantom is, what it does today, what is in progress
2. The [Architecture Principles](#architecture-principles) section below — the decisions that are locked and why

If you want to challenge a locked decision, open an issue first. Do not submit a PR that violates one without prior discussion.

### Check existing issues

Before starting work, confirm no one else is working on the same thing. Comment on the issue to claim it.

### Start with something focused

If you are new to the project, good first contributions are:
- Bug fixes with clear reproduction steps
- Documentation improvements
- Test coverage for existing behavior
- Performance improvements with before/after benchmark numbers

---

## Architecture Principles

These are non-negotiable. Every contribution must respect them.

**1. No rendering pipeline — ever.**
No `wgpu`, no `skia`, no `webrender`, no pixel rasterization. Agents do not have eyes. If your contribution requires rendering pixels, it does not belong in Phantom Engine.

**2. CCT is the output format.**
The Headless Serializer outputs CCT (Compact Context Text). Never raw HTML. Never raw DOM JSON. Never screenshots as primary perception output. The CCT specification is in `README.md`.

**3. rquest over reqwest.**
All HTTP client code uses `rquest`, not `reqwest`. The `rquest` crate uses BoringSSL for JA4 TLS fingerprint spoofing. `reqwest` produces a bot-detectable TLS handshake. This decision is locked.

**4. arena_id in JS wrappers — never Rust references.**
JS wrapper objects store `arena_id: u64` only. Never `Arc<DomNode>`, never `&DomNode`. Storing Rust references in JS objects creates memory cycles across two garbage collection systems, causing OOM in long-running agent sessions. This decision is locked.

**5. Burn it down.**
Each agent task gets its own JS isolate and DOM tree. When the task ends, drop everything. No shared mutable state between sessions. This decision is locked.

**6. Embed, don't rebuild.**
Use `html5ever` for HTML parsing, `taffy` for layout, `indextree` for the DOM arena. Do not reimplement what these crates provide.

---

## Development Setup

### Prerequisites

```bash
# Rust stable 1.80+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install stable
rustup default stable
rustup component add clippy rustfmt

# System dependencies (Debian/Ubuntu)
sudo apt-get install -y \
    build-essential cmake ninja-build clang llvm \
    libssl-dev pkg-config libsqlite3-dev git curl
```

### Build

```bash
git clone https://github.com/polymit/phantom
cd phantom
cargo build --workspace
```

### Verify setup

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --workspace --check
```

If any of these fail on the `main` branch, open an issue before contributing.

---

## How to Contribute

### Step 1 — Fork and branch

Fork the repository on GitHub, clone your fork, then create a branch:

```bash
git clone https://github.com/YOUR_USERNAME/phantom
cd phantom
git checkout -b fix/your-fix-description
```

Branch naming:
- `fix/` — bug fixes
- `feat/` — new features
- `perf/` — performance improvements
- `docs/` — documentation only
- `test/` — test additions only
- `refactor/` — code cleanup with no behavior change

### Step 2 — Make your changes

Keep changes focused. One PR per concern. Write tests for everything you add or change.

### Step 3 — Verify quality gates

All of these must pass before submitting:

```bash
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --workspace
cargo test --workspace
```

All four commands must exit cleanly.

### Step 4 — Submit your PR

Open a pull request against the `main` branch with a clear description.

---

## Pull Request Process

### PR title format

```
fix(phantom-core): correct visibility computation for opacity:0 elements
feat(phantom-mcp): add browser_go_forward history stack
perf(phantom-serializer): reduce CCT encoding allocations by 30%
docs(readme): add example for tab switching
test(phantom-storage): add cross-session isolation test
```

Format: `type(crate): description`

### PR description must include

1. **What** — what does this PR change?
2. **Why** — why is this change needed?
3. **How** — how was it implemented?
4. **Tests** — what tests were added or changed?
5. **Performance** — if relevant, before/after benchmark numbers

### Review process

All PRs require at least one review before merging. Reviewers check for architecture compliance first, then correctness, then performance.

### What reviewers look for

- Does it violate any locked architectural decision?
- Does it add `unwrap()` in production code paths?
- Does it use `reqwest` instead of `rquest`?
- Does it store Rust references in JS wrappers?
- Does it add a rendering pipeline dependency?
- Are all public functions documented?
- Are there tests for the new behavior?
- Does `cargo clippy` pass with zero warnings?

---

## Code Standards

### Error handling

```rust
// Correct — use thiserror for typed errors
#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("element not found: {selector}")]
    NotFound { selector: String },
}

// Wrong — avoid Box<dyn Error> in public APIs
pub fn my_fn() -> Result<(), Box<dyn Error>> { ... }

// Wrong — never unwrap() in production code paths
let node = tree.get_node(id).unwrap();

// Correct — handle explicitly
let node = match tree.get_node(id) {
    Some(n) => n,
    None => return Err(DomError::NodeNotFound { id }),
};
```

### Logging

```rust
// Correct
tracing::info!(url = %url, session_id = %id, "navigation started");
tracing::debug!(node_count = count, "serialization complete");
tracing::error!(error = %err, "JS execution failed");

// Wrong — never println! in production code
println!("debug: {}", value);
```

### Locking

```rust
// Correct — always parking_lot
use parking_lot::RwLock;
let data = RwLock::new(HashMap::new());

// Wrong — std RwLock is slower and less ergonomic
use std::sync::RwLock;
```

### Documentation

Every public function and struct must have doc comments:

```rust
/// Serializes the DOM tree to CCT format.
///
/// Runs the 8-stage pipeline: visibility computation (bottom-up),
/// geometry extraction (top-down), viewport culling, semantic
/// extraction (parallel), and CCT encoding.
///
/// # Arguments
/// * `dom` - The DOM tree to serialize (read-only)
/// * `bounds` - Bounding boxes from the layout engine
///
/// # Returns
/// A CCT-formatted string with one node per line.
pub fn serialize(
    &self,
    dom: &DomTree,
    bounds: &HashMap<NodeId, ViewportBounds>,
) -> String { ... }
```

---

## Testing Requirements

Every contribution must include tests.

### Test types

**Unit tests** — test individual functions in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_display_none() {
        assert_eq!(parse_display("none"), Some(Display::None));
        assert_eq!(parse_display("NONE"), Some(Display::None));
        assert_eq!(parse_display("invalid"), None);
    }
}
```

**Integration tests** — test cross-component behavior in the `tests/` directory.

**Benchmark tests** — for performance-sensitive code in the `benches/` directory.

### Performance gates

These must not regress. If your change causes a regression, include a justification in the PR description:

| Operation | Gate |
| --- | --- |
| CCT serialization (1000 nodes) | < 5ms |
| Delta serialization (10 mutations) | < 1ms |
| Session creation (from pool) | < 10ms |

Run benchmarks with:

```bash
cargo bench --workspace
```

### Test naming

```rust
// Correct — descriptive, verb-noun format
#[test]
fn test_hidden_elements_excluded_from_cct() { ... }

#[test]
fn test_session_isolation_prevents_cross_read() { ... }

#[tokio::test]
async fn test_navigate_retries_on_timeout() { ... }

// Wrong — vague names
#[test]
fn test1() { ... }

#[test]
fn it_works() { ... }
```

---

## Reporting Bugs

Open an issue with the title format: `[BUG] Short description of the problem`

Include:

1. **Phantom Engine version** — git commit hash or tag
2. **Operating system** — `uname -a` output
3. **Rust version** — `rustc --version`
4. **Reproduction steps** — exact steps to reproduce
5. **Expected behavior** — what should happen
6. **Actual behavior** — what actually happens
7. **Relevant logs** — with `RUST_LOG=debug` if possible
8. **Minimal reproduction** — smallest code that shows the bug

---

## Security Vulnerabilities

Do not report security vulnerabilities as public GitHub issues.

Email **security@polymit.dev** with a description of the issue, reproduction steps, and your assessment of impact. We will acknowledge within 48 hours and aim to ship a fix within 14 days of confirmed severity.

---

## Suggesting Features

Open an issue with the title format: `[FEAT] Short description of the feature`

Include:

1. **Problem** — what problem does this solve for AI agents?
2. **Proposed solution** — how would it work?
3. **Architecture impact** — which crates are affected?
4. **Does it violate any locked decision?** — if yes, why should the decision change?
5. **Alternatives considered** — what else did you consider?

Feature requests that violate locked architectural decisions require a strong justification. The bar is high.

---

## Crate Guide

| Crate | Responsibility | Key files |
| --- | --- | --- |
| `phantom-net` | HTTP client, TLS fingerprinting, cookie jar | `src/lib.rs` |
| `phantom-core` | HTML parsing, CSS cascade, DOM tree, layout | `src/dom/`, `src/css/`, `src/layout/`, `src/parser/` |
| `phantom-js` | QuickJS runtime, DOM bindings, browser shims | `src/quickjs/`, `src/shims/` |
| `phantom-serializer` | CCT encoder, 8-stage pipeline, delta engine | `src/lib.rs` |
| `phantom-session` | Session broker, isolate pool, circuit breaker, scheduler | `src/broker.rs`, `src/pool.rs` |
| `phantom-storage` | Per-session storage, snapshots, quota management | `src/session_mgr.rs`, `src/snapshot.rs`, `src/quota_mgr.rs`, `src/security.rs` |
| `phantom-mcp` | MCP server, tool dispatch, auth, metrics | `src/server.rs`, `src/tools/` |
| `phantom-anti-detect` | Persona pool, JS fingerprint shims, canvas noise, action sequences | `src/persona.rs`, `src/shims.rs`, `src/canvas.rs`, `src/action.rs` |

### Dependency rules

- Lower layers must never depend on higher layers
- `phantom-core` must never depend on `phantom-js`
- `phantom-serializer` must never depend on `phantom-mcp`
- `phantom-anti-detect` is cross-cutting and may be used by any layer

---

Phantom Engine is built by [Polymit](https://github.com/polymit).
