# Contributing to Phantom Engine

Thank you for your interest in contributing to Phantom Engine — a purpose-built browser engine for AI agents written in Rust.

This document explains how to contribute effectively. Please read it completely before opening an issue or pull request.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Before You Contribute](#before-you-contribute)
- [Architecture Principles](#architecture-principles)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Code Standards](#code-standards)
- [Testing Requirements](#testing-requirements)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)
- [Crate Guide](#crate-guide)

---

## Code of Conduct

Phantom Engine is built by engineers who care deeply about quality. We expect all contributors to:

- Be respectful and constructive in all communication
- Focus on technical merit in reviews and discussions
- Welcome contributors of all experience levels
- Give specific, actionable feedback — not vague criticism

Disrespectful behavior will result in removal from the project.

---

## Before You Contribute

### Understand the architecture first

Phantom Engine has a precise, research-backed architecture with locked design decisions. Before writing any code, read:

1. `README.md` — understand what we're building and why
3. The locked decisions in the project architecture documentation the architecture section of the README

**The most important rule:** Every architectural decision in this project exists for a specific reason. If you want to change an architectural decision, open an issue first and explain your reasoning. Do not submit a PR that violates a locked decision without prior discussion.

### Check existing issues

Before starting work, check if someone is already working on the same thing. Comment on the issue to claim it.

### Start small

If you are new to the project, start with:
- Bug fixes with clear reproduction steps
- Documentation improvements
- Test coverage additions
- Performance improvements with benchmark evidence

---

## Architecture Principles

These principles are non-negotiable. Every contribution must respect them.

**1. No rendering pipeline — ever.**
No `wgpu`, no `skia`, no `webrender`, no pixel rasterization. Agents do not have eyeballs. If your contribution requires rendering pixels, it does not belong in Phantom Engine.

**2. CCT is the output format.**
The Headless Serializer outputs CCT (Custom Compressed Text). Never raw HTML. Never raw DOM JSON. Never screenshots as primary perception output. See the project architecture documentation for the full CCT specification.

**3. rquest over reqwest.**
All HTTP client code uses `rquest`, not `reqwest`. The difference matters: `rquest` uses BoringSSL for JA4 TLS fingerprint spoofing. `reqwest` produces bot-detectable TLS handshakes. This is Decision D-13. It is locked.

**4. arena_id in JS wrappers — never Rust references.**
JS wrapper objects store `arena_id: u64` only. Never `Arc<DomNode>`, never `&DomNode`. Storing Rust references in JS objects creates memory cycles across two GC systems, causing OOM crashes in long-running agent sessions. This is Decision D-09. It is locked.

**5. Burn it down.**
Each agent task gets its own JS isolate and DOM tree. When the task completes, drop everything. No shared mutable state between sessions. This is Decision D-08. It is locked.

**6. Embed, don't rebuild.**
Use `html5ever` for HTML parsing. Use `taffy` for layout. Use `indextree` for the DOM tree arena. Do not rebuild what these crates provide. See the crate guide at the bottom of this document.

---

## Development Setup

### Prerequisites

```bash
# Rust (nightly recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly
rustup component add clippy rustfmt

# System dependencies (Debian/Ubuntu/Linux Mint)
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
cargo test --workspace          # All tests pass
cargo clippy --workspace -- -D warnings  # Zero warnings
cargo fmt --workspace --check   # Formatted correctly
```

If any of these fail on the main branch, open an issue before contributing.

---

## How to Contribute

### Step 1 — Fork and branch

```bash
git fork https://github.com/polymit/phantom
git checkout -b fix/your-fix-description
# or
git checkout -b feat/your-feature-description
```

Branch naming:
- `fix/` — bug fixes
- `feat/` — new features
- `perf/` — performance improvements
- `docs/` — documentation only
- `test/` — test additions only
- `refactor/` — code cleanup with no behavior change

### Step 2 — Make your changes

- Keep changes focused. One PR per concern.
- Write tests for everything you add or change.
- Run the full test suite before submitting.

### Step 3 — Verify quality gates

All of these must pass before submitting:

```bash
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --workspace
cargo test --workspace
```

Zero errors. Zero warnings. No exceptions.

### Step 4 — Submit your PR

Open a pull request against the `main` branch with a clear description.

---

## Pull Request Process

### PR title format

```
fix(phantom-core): correct visibility computation for opacity:0 elements
feat(phantom-mcp): add browser_scroll tool to MCP interface
perf(phantom-serializer): reduce CCT encoding allocations by 30%
docs(readme): add example for session cloning
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

- All PRs require at least one review before merging
- Reviewers will check for architecture compliance first
- Performance-sensitive changes require benchmark evidence
- Security-sensitive changes require extra scrutiny

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

### Rust style

Follow official Rust style guidelines enforced by `cargo fmt` and `cargo clippy`.

**Error handling:**
```rust
// CORRECT — use thiserror for typed errors
#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("element not found: {selector}")]
    NotFound { selector: String },
}

// WRONG — never in public APIs
pub fn my_fn() -> Result<(), Box<dyn Error>> { ... }

// WRONG — never in production code paths
let node = tree.get_node(id).unwrap();

// CORRECT — use match or if let
let node = match tree.get_node(id) {
    Some(n) => n,
    None => return Err(DomError::NodeNotFound { id }),
};
```

**Logging:**
```rust
// CORRECT
tracing::info!(url = %url, session_id = %id, "navigation started");
tracing::debug!(node_count = count, "serialization complete");
tracing::error!(error = %err, "JS execution failed");

// WRONG — never in production code
println!("debug: {}", value);
```

**Locking:**
```rust
// CORRECT — always parking_lot
use parking_lot::RwLock;
let data = RwLock::new(HashMap::new());

// WRONG — std RwLock is slower
use std::sync::RwLock;
```

**Documentation:**
```rust
// Every public function and struct must have doc comments
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

Every contribution must include tests. No exceptions.

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

**Integration tests** — test cross-component behavior in `tests/` directory.

**Benchmark tests** — for performance-sensitive code in `benches/` directory.

### Performance gates

These gates must not regress. If your change causes a regression, include a justification:

| Operation | Gate |
|---|---|
| CCT serialization (1000 nodes) | < 5ms |
| Delta serialization (10 mutations) | < 1ms |
| Session creation (from pool) | < 10ms |
| Session clone (COW snapshot) | < 200ms |

Run benchmarks with:
```bash
cargo bench --workspace
```

### Test naming

```rust
// CORRECT — descriptive, verb-noun format
#[test]
fn test_hidden_elements_excluded_from_cct() { ... }

#[test]
fn test_session_isolation_prevents_cross_read() { ... }

#[tokio::test]
async fn test_navigate_retries_on_timeout() { ... }

// WRONG — vague names
#[test]
fn test1() { ... }

#[test]
fn it_works() { ... }
```

---

## Reporting Bugs

Open an issue with:

**Title:** `[BUG] Short description of the problem`

**Body must include:**

1. **Phantom Engine version** — git commit hash or tag
2. **Operating system** — `uname -a` output
3. **Rust version** — `rustc --version`
4. **Reproduction steps** — exact steps to reproduce
5. **Expected behavior** — what should happen
6. **Actual behavior** — what actually happens
7. **Relevant logs** — with `RUST_LOG=debug` if possible
8. **Minimal reproduction** — smallest code that shows the bug

Security vulnerabilities must not be reported as public issues. Email the maintainer directly.

---

## Suggesting Features

Open an issue with:

**Title:** `[FEAT] Short description of the feature`

**Body must include:**

1. **Problem** — what problem does this solve for AI agents?
2. **Proposed solution** — how would it work?
3. **Architecture impact** — which crates are affected?
4. **Does it violate any locked decision?** — if yes, why should the decision change?
5. **Alternatives considered** — what else did you consider?

Feature requests that violate locked architectural decisions will be discussed carefully. They may be accepted if the justification is strong enough, but the bar is high.

---

## Crate Guide

Understanding which crate owns what responsibility:

| Crate | Responsibility | Key files |
|---|---|---|
| `phantom-net` | HTTP client, TLS fingerprinting, cookie jar | `src/lib.rs` |
| `phantom-core` | HTML parsing, CSS cascade, DOM tree, layout | `src/dom/`, `src/css/`, `src/layout/`, `src/parser/` |
| `phantom-js` | QuickJS + V8 engines, DOM bindings, shims | `src/quickjs/`, `src/shims/` |
| `phantom-serializer` | CCT encoder, 8-stage pipeline, delta engine | `src/lib.rs` |
| `phantom-session` | Session broker, isolate pool, scheduler | `src/broker.rs`, `src/pool.rs` |
| `phantom-storage` | Per-session storage, snapshots, quotas | `src/manager.rs`, `src/security.rs` |
| `phantom-mcp` | MCP server, 20 tools, auth, SSE streaming | `src/server.rs`, `src/tools/` |
| `phantom-anti-detect` | Persona pool, JS shims, action sequences | `src/persona.rs`, `src/shims.rs` |

**Dependency rules:**
- Lower layers must never depend on higher layers
- `phantom-core` must never depend on `phantom-js`
- `phantom-serializer` must never depend on `phantom-mcp`
- Cross-cutting concerns (`phantom-anti-detect`) may be used by any layer

---

## Questions

If you have questions about the architecture, the codebase, or how to implement something:

1. Read the project architecture documentation first — most answers are there
2. Search existing issues — your question may already be answered
3. Open a discussion issue with the `[QUESTION]` prefix

---

## About

Phantom Engine is a product of **Polymit** — building the infrastructure layer for the agentic web.

https://github.com/polymit
