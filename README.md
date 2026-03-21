# Phantom Engine

> Purpose-built browser engine for AI agents — zero rendering, full MCP protocol, 1000+ concurrent sessions.

[![Docker Hub](https://img.shields.io/docker/pulls/polymit/phantom?style=flat-square)](https://hub.docker.com/r/polymit/phantom)
[![Docker Image Size](https://img.shields.io/docker/image-size/polymit/phantom/latest?style=flat-square)](https://hub.docker.com/r/polymit/phantom)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.2.0-green?style=flat-square)](https://github.com/polymit/phantom/releases)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-CE412B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)

---

Phantom Engine is not a Chrome wrapper. It is not Playwright with a different API.
It is a native Rust browser engine designed from the ground up for one purpose:
giving AI agents a fast, token-efficient, anti-detection-resistant browser.

```
 ██████╗ ██╗  ██╗ █████╗ ███╗   ██╗████████╗ ██████╗ ███╗   ███╗
 ██╔══██╗██║  ██║██╔══██╗████╗  ██║╚══██╔══╝██╔═══██╗████╗ ████║
 ██████╔╝███████║███████║██╔██╗ ██║   ██║   ██║   ██║██╔████╔██║
 ██╔═══╝ ██╔══██║██╔══██║██║╚██╗██║   ██║   ██║   ██║██║╚██╔╝██║
 ██║     ██║  ██║██║  ██║██║ ╚████║   ██║   ╚██████╔╝██║ ╚═╝ ██║
 ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝     ╚═╝
```

---

## Why Phantom Engine

| Problem | Phantom's Answer |
| --- | --- |
| Chrome costs 300ms per session startup | QuickJS sessions start in **< 1ms** |
| Raw DOM JSON uses ~121 tokens/node | CCT uses **~20 tokens/node** — 6× more efficient |
| Headless Chrome has a detectable TLS fingerprint | `rquest` + BoringSSL produces **JA4-spoofed handshakes** |
| Chrome cannot run 1000 sessions on one machine | Phantom runs **1000+ concurrent sessions** with < 1GB RAM |

---

## Table of Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Connect an Agent](#connect-an-agent-mcp-config)
- [First Agent Session](#first-agent-session)
- [CCT Format](#cct-format)
- [Performance](#performance)
- [Architecture](#architecture)
- [MCP Tools Reference](#mcp-tools-reference)
- [Configuration](#configuration)
- [Development](#development)
- [Roadmap](#roadmap)

---

## Install

### Option 1 — One-line install (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/polymit/phantom/main/install.sh | sh
```

### Option 2 — Docker (recommended for production)

```bash
docker pull polymit/phantom:latest
```

### Option 3 — Build from source

**Prerequisites:** Rust 1.80+, `cmake`, `clang`, `pkg-config`, `libssl-dev`

```bash
# 1. Install system dependencies (Debian/Ubuntu)
sudo apt-get install -y pkg-config libssl-dev cmake clang

# 2. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Clone and build
git clone https://github.com/polymit/phantom.git
cd phantom
cargo build --release --workspace

# 4. Run
cargo run --release --bin phantom-mcp -- \
  --port 8080 \
  --api-keys your-secret-key
```

> **Note on build time:** The first build compiles BoringSSL via `cmake` and takes 5–10 minutes. Subsequent incremental builds are fast. If you hit a `cmake` not found error, install it with `sudo apt-get install cmake` (Linux) or `brew install cmake` (macOS).

---

## Quick Start

### Run with Docker

```bash
# Pull and run v0.2.0
docker run -d \
  -p 8080:8080 \
  -e PHANTOM_API_KEYS=your-secret-key \
  polymit/phantom:v0.2.0

# Verify it's running
curl http://localhost:8080/health
# {"status":"ok","version":"0.2.0","circuit_breaker":"Closed"}
```

### Full stack with monitoring (Prometheus + Grafana)

```bash
# Clone and configure
git clone https://github.com/polymit/phantom.git
cd phantom
cp .env.example .env
# Edit .env and set PHANTOM_API_KEYS=your-secret-key

# Launch everything
docker compose up -d
```

| Service | URL | Default credentials |
| --- | --- | --- |
| MCP server | http://localhost:8080 | — |
| Prometheus | http://localhost:9090 | — |
| Grafana | http://localhost:3000 | `admin` / `phantom` |

> **Tip:** Always set `PHANTOM_API_KEYS` before running in any environment accessible from a network. If the key is unset, the server starts without authentication and logs a warning.

---

## Connect an Agent (MCP Config)

### Claude Desktop / Claude Code

Add to your MCP configuration file:

```json
{
  "mcpServers": {
    "phantom": {
      "url": "http://localhost:8080/mcp",
      "headers": { "X-API-Key": "your-key" }
    }
  }
}
```

### Run the container as an inline MCP command

```json
{
  "mcpServers": {
    "phantom": {
      "command": "docker",
      "args": [
        "run", "--rm",
        "-p", "8080:8080",
        "-e", "PHANTOM_API_KEYS=your-key",
        "polymit/phantom:v0.2.0"
      ],
      "headers": {
        "X-API-Key": "your-key"
      }
    }
  }
}
```

### Authentication

All endpoints except `/health` and `/metrics` require an `X-API-Key` header. Pass multiple keys as a comma-separated list:

```bash
-e PHANTOM_API_KEYS=key-one,key-two,key-three
```

A missing or invalid key returns `401 Unauthorized`:

```json
{ "error": { "code": "unauthorized", "message": "Valid X-API-Key header is required" } }
```

---

## First Agent Session

Every tool call is a JSON-RPC 2.0 `POST` to `/mcp`. The first call to `browser_navigate` creates a new session and returns a `session_id`. Pass that ID in every subsequent call to maintain state.

```python
import requests

BASE = "http://localhost:8080/mcp"
HEADERS = {"X-API-Key": "your-key", "Content-Type": "application/json"}

def call(tool, args, session_id=None):
    body = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "id": 1,
        "params": {"name": tool, "arguments": args}
    }
    if session_id:
        body["params"]["session_id"] = session_id
    return requests.post(BASE, json=body, headers=HEADERS).json()

# 1. Navigate — creates a new session automatically
r = call("browser_navigate", {"url": "https://example.com"})
session_id = r["result"]["session_id"]

# 2. Read the page as a CCT scene graph (~20 tokens/node)
r = call("browser_get_scene_graph", {"format": "cct"}, session_id)
print(r["result"]["scene_graph"])
# n_0|div|main|0,0,1280,720|b,v,1.0,a|Main|-|-|root|0
# n_1|lnk|lnk|356,288,292,18|b,v,1.0,a|More information...|...|c|n_0|1

# 3. Click a link by CSS selector
call("browser_click", {"selector": "a"}, session_id)

# 4. Type into an input field
call("browser_type", {"selector": "input[name='q']", "text": "phantom engine"}, session_id)

# 5. Evaluate JavaScript
r = call("browser_evaluate", {"script": "document.title"}, session_id)
print(r["result"]["result"])  # → "Example Domain"
```

> **Session lifecycle:** Sessions are scoped to their `session_id`. Each session holds its own DOM, cookie jar, tab list, and navigation history. Sessions are never shared between callers.

---

## CCT Format

CCT (Compact Context Text) is Phantom's native output format — designed specifically to reduce the tokens an LLM consumes when reading page structure.

Each visible DOM node is a single pipe-delimited line:

```
id | tag | role | x,y,w,h | display,visibility,opacity,pointer | accessible_name | visible_text | events | parent | flags
```

### Example output

```
n_0|div|main|0,0,1280,720|b,v,1.0,a|Main|-|-|root|0
n_1|lnk|lnk|356,288,292,18|b,v,1.0,a|More information...|More information...|c|n_0|1
n_2|btn|btn|600,400,120,40|b,v,1.0,a|Submit|-|c|n_0|0
```

### Field reference

| Field | Values | Description |
| --- | --- | --- |
| `id` | `n_0`, `n_1` … | Stable node identifier. Pinned across refreshes via `data-agent-id` or `data-testid` if present. |
| `tag` | `div`, `lnk`, `btn`, `inpt`, `frm`, `sel`, `canv`, `ifrm`, `span` | Abbreviated HTML tag |
| `role` | ARIA role string | `btn`, `lnk`, `ipt`, `nav`, `main`, `none` |
| `x,y,w,h` | integers | Bounding box in pixels at 1920×1080 viewport |
| `display` | `b` `i` `f` `g` `n` | block / inline / flex / grid / none |
| `visibility` | `v` `h` | visible / hidden |
| `opacity` | `0.0`–`1.0` | Computed opacity |
| `pointer` | `a` `n` | pointer-events active / none |
| `accessible_name` | string or `-` | `aria-label` → `aria-labelledby` → `title` → `alt` → `placeholder` |
| `visible_text` | string or `-` | Direct text content, truncated at 100 chars |
| `events` | `c`, `f`, `-` | Inferred events: `c`=click, `f`=focus |
| `parent` | id or `root` | Parent node ID |
| `flags` | integer bitmask | `2`=iframe, `4`=canvas, `8`=svg |

Optional `s:disabled,checked,selected,expanded,required` state field appended when any state bit is set.

**~20 tokens/node vs ~121 for raw JSON DOM — 6× reduction.** This directly cuts LLM costs and latency for every page an agent reads.

---

## Performance

| Metric | Phantom Engine | Chrome Headless | Lightpanda |
| --- | --- | --- | --- |
| Session startup | < 1ms (QJS) / < 50ms (V8) | ~300ms | ~50ms |
| Tokens per node | ~20 (CCT) | ~121 (JSON) | ~63 (JSON) |
| 1000 concurrent sessions | ✅ < 1 GB RAM | ❌ ~150 GB | ~4 GB |
| TLS fingerprint spoofing | ✅ JA4 + BoringSSL | ❌ Detectable | ❌ Detectable |
| CCT serialization (1000 nodes) | < 5ms | N/A | N/A |

---

## Architecture

Phantom Engine is a 5-layer Rust workspace with a cross-cutting anti-detection module:

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer 5 — MCP Server                phantom-mcp                │
│  Axum + Tokio · JSON-RPC 2.0 · 20 tools · Auth · Prometheus    │
├───────────────────────────┬─────────────────────────────────────┤
│  Layer 4 — Session Broker │  Storage                            │
│  phantom-session          │  phantom-storage                    │
│  Isolate pool · Circuit   │  SQLite IndexedDB · sled KV         │
│  breaker · Scheduler      │  Per-session isolation · Snapshots  │
├───────────────────────────┴─────────────────────────────────────┤
│  Layer 3 — Serializer                phantom-serializer         │
│  CCT encoder · 2-pass traversal · Rayon parallel extraction     │
│  Mutation coalescing · Delta streaming                          │
├───────────────────────────┬─────────────────────────────────────┤
│  Layer 2 — Core Engine    │  JS Engine                          │
│  phantom-core             │  phantom-js                         │
│  html5ever · indextree    │  QuickJS (Tier 1, < 1ms)           │
│  taffy layout · CSS       │  V8 (Tier 2, SPAs)                  │
│  cascade                  │  DOM bindings · Timer shims         │
├───────────────────────────┴─────────────────────────────────────┤
│  Layer 1 — Network                   phantom-net                │
│  rquest + BoringSSL · Chrome130 TLS impersonation · JA4 spoof  │
├─────────────────────────────────────────────────────────────────┤
│  Cross-cutting — Anti-Detection      phantom-anti-detect        │
│  Persona pool · Deterministic fingerprints · Canvas noise       │
│  Navigator shims · screen/hardware spoofing                     │
└─────────────────────────────────────────────────────────────────┘
```

### Key design decisions

- **No rendering pipeline — ever.** No wgpu, no Skia, no pixel buffers. Agents do not have eyeballs.
- **CCT output.** ~20 tokens/node. 6× more efficient than JSON DOM.
- **rquest over reqwest.** BoringSSL for JA4 TLS fingerprint spoofing. Impersonates Chrome 130 at the handshake level.
- **Two-tier JS.** QuickJS for ~80% of pages (forms, static content). V8 for SPAs (React/Vue/Angular). One runtime per task, burned down after use.
- **Deterministic anti-detection.** Each session UUID seeds its own RNG, producing a consistent Chrome 130 persona across its lifetime. Spoofs `navigator`, `screen`, canvas entropy, hardware concurrency, and device memory.
- **Per-task memory model.** Every session's DOM and JS isolate are fully dropped on destroy. No cross-session state leakage.
- **Persistence-first navigation.** Full history stack with `browser_go_back` and `browser_refresh`. Per-session cookie isolation via `cookie_store`.

---

## MCP Tools Reference

### ✅ Implemented and working

| Category | Tool | Description |
| --- | --- | --- |
| **Navigation** | `browser_navigate` | Fetch URL, parse HTML, compute layout. Exponential backoff on network errors. |
| | `browser_go_back` | Pop history stack and reload previous URL. |
| | `browser_refresh` | Reload the current active URL. |
| **Interaction** | `browser_click` | Resolve selector → coordinates, dispatch full mouse event sequence with delays. |
| | `browser_type` | Per-character keydown/keypress/input/keyup (~80ms/char). |
| | `browser_press_key` | Dispatch a single key event by name (`Enter`, `Tab`, `Escape`, …). |
| | `browser_wait_for_selector` | Poll DOM up to a configurable timeout (default 30s). |
| **Perception** | `browser_get_scene_graph` | Serialize DOM + layout to CCT format. |
| | `browser_evaluate` | Execute JavaScript in a fresh QuickJS isolate, return result. |
| **Tabs** | `browser_new_tab` | Create a new tab in the session. |
| | `browser_switch_tab` | Activate a tab by `tabId`. |
| | `browser_list_tabs` | Return all open tabs with URL and title. |
| | `browser_close_tab` | Close a tab; auto-creates a blank tab if the last one is closed. |
| **Storage** | `browser_get_cookies` | List all cookies in the session's isolated cookie jar. |
| | `browser_set_cookie` | Parse and store a cookie scoped to the active URL. |
| | `browser_clear_cookies` | Wipe the session's cookie jar. |

### 🚧 Partial / stub — coming in v0.2+

| Tool | Status | Notes |
| --- | --- | --- |
| `browser_go_forward` | Stub | Returns `{ success: true }` without navigating. History forward not yet tracked. |
| `browser_snapshot` | Stub | Returns a 1×1 transparent PNG placeholder. Full screenshot rendering is on the roadmap. |
| `browser_subscribe_dom` | Stub | Returns `{ stream_established: true }` but the `/mcp/stream` SSE endpoint returns `501`. Delta streaming is not yet active. |
| `browser_session_snapshot` | Partial | Generates a `snapshot_id` UUID but does not persist DOM to disk. |
| `browser_session_clone` | Partial | Returns a new `session_id` but does not fork via V8 snapshot. |

> Cookie integration in `browser_navigate` is present in the session model but not yet plumbed into the `PagePipeline` fetch. Cookies set via `browser_set_cookie` are stored per-session but are not forwarded as request headers on the next navigation.

---

## Configuration

All configuration via environment variables or CLI flags:

| Variable | Flag | Default | Description |
| --- | --- | --- | --- |
| `PHANTOM_PORT` | `--port` | `8080` | MCP / HTTP server port |
| `PHANTOM_METRICS_PORT` | `--metrics-port` | `9091` | Prometheus metrics port |
| `PHANTOM_MAX_SESSIONS` | `--max-sessions` | `1000` | Maximum concurrent sessions |
| `PHANTOM_API_KEYS` | `--api-keys` | *(unset)* | Comma-separated API keys. Unset = no auth (dev only). |
| `RUST_LOG` | `--log-level` | `info` | Log level: `error` `warn` `info` `debug` `trace` |
| `PHANTOM_JSON_LOGS` | `--json-logs` | `false` | Emit structured JSON logs (for log aggregators) |

Copy `.env.example` to `.env` and set at minimum `PHANTOM_API_KEYS` before running in any non-local environment.

---

## Development

### Prerequisites

- Rust 1.80+ — `rustup update stable`
- `cmake`, `clang`, `pkg-config`, `libssl-dev`

```bash
sudo apt-get install -y pkg-config libssl-dev cmake clang
```

### Build and test

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace -- --nocapture

# Run the 1000-session scale test
cargo test --package phantom-mcp test_1000_concurrent_sessions_scale -- --nocapture

# Lint (zero warnings enforced)
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --workspace
```

### Run locally

```bash
# Development mode (no auth, debug logs)
cargo run --bin phantom-mcp -- \
  --port 8080 \
  --api-keys my-dev-key \
  --log-level debug

# Production mode (JSON structured logs)
cargo run --bin phantom-mcp -- \
  --api-keys my-prod-key \
  --json-logs
```

### Project layout

```
phantom/
├── Cargo.toml                  # Workspace root
├── Dockerfile                  # Multi-stage build (Rust builder → bookworm-slim)
├── docker-compose.yml          # Engine + Prometheus + Grafana
├── prometheus.yml              # Prometheus scrape config
├── .env.example                # All environment variables
├── assets/
│   └── polymit_logo.png        # Polymit logo (used in README header)
├── crates/
│   ├── phantom-net/            # Layer 1: Network (rquest + BoringSSL)
│   ├── phantom-core/           # Layer 2: HTML, CSS, DOM tree, layout
│   ├── phantom-js/             # Layer 2: QuickJS + V8 JS engines
│   ├── phantom-serializer/     # Layer 3: CCT encoder + delta
│   ├── phantom-session/        # Layer 4: Session broker, pool, scheduler
│   ├── phantom-storage/        # Layer 4: Storage isolation + IndexedDB
│   ├── phantom-mcp/            # Layer 5: MCP server (20 tools)
│   └── phantom-anti-detect/    # Cross-cutting: Persona pool + JS shims
└── .github/workflows/ci.yml    # CI: build + clippy + test + docker
```

---

## Roadmap

### v0.2 (current)
- [x] Core navigation, interaction, perception, tabs, storage tools
- [x] CCT serialization with 2-pass traversal
- [x] Anti-detection persona pool
- [x] Per-session cookie isolation
- [ ] Cookie forwarding in `PagePipeline` fetch
- [ ] `browser_go_forward` — history forward navigation
- [ ] `browser_snapshot` — real screenshot output
- [ ] `browser_subscribe_dom` — live SSE delta streaming
- [ ] `browser_session_snapshot` / `browser_session_clone` — full persistence
- [ ] Python + TypeScript SDK wrappers
- [ ] `browser_fill_form` convenience tool

### v0.3
- [ ] CCT binary encoding (further token reduction)
- [ ] Cloud-hosted managed sessions
- [ ] Verified benchmark suite with CI-gated performance gates

### v1.0
- [ ] APEX: fully managed agent browser platform

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issues and PRs welcome.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE).

---

<p align="center">
  Phantom Engine is a product of <a href="https://github.com/polymit">Polymit</a>
</p>
