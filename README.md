# Phantom Engine

> Purpose-built browser engine for AI agents — zero rendering, full MCP protocol, 1000+ concurrent sessions.

[![Docker Hub](https://img.shields.io/docker/pulls/polymit/phantom?style=flat-square)](https://hub.docker.com/r/polymit/phantom)
[![Docker Image Size](https://img.shields.io/docker/image-size/polymit/phantom/latest?style=flat-square)](https://hub.docker.com/r/polymit/phantom)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square)](https://www.rust-lang.org/)

Phantom Engine is not a Chrome wrapper. It is not Playwright with a different API.
It is a native Rust browser engine designed from the ground up for one purpose:
giving AI agents a fast, token-efficient, anti-detection-resistant browser.

## Why Phantom Engine

| Problem | Phantom's Answer |
| --- | --- |
| Chrome costs 300ms per session startup | QuickJS sessions start in **< 1ms** |
| Raw DOM JSON uses ~121 tokens/node | CCT uses **~20 tokens/node** — 6x more efficient |
| Headless Chrome has a detectable TLS fingerprint | `rquest` + BoringSSL produces **JA4-spoofed handshakes** |
| Chrome cannot run 1000 sessions on one machine | Phantom runs **1000+ concurrent sessions** with < 1GB RAM |

---

## Install

### One-line install
```bash
curl -fsSL https://raw.githubusercontent.com/polymit/phantom/main/install.sh | sh
```

### Docker
```bash
docker pull polymit/phantom:latest
```

### Build from source

**Prerequisites:** Rust 1.80+, `cmake`, `clang`, `pkg-config`, `libssl-dev`
```bash
# Install system dependencies (Debian/Ubuntu)
sudo apt-get install -y pkg-config libssl-dev cmake clang

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/polymit/phantom.git
cd phantom
cargo build --release --workspace

# Run
cargo run --release --bin phantom-mcp -- \
  --port 8080 \
  --api-keys your-secret-key
```
## Quick Start with Docker

```bash
# Pull and run
docker run -d \
  -p 8080:8080 \
  -e PHANTOM_API_KEYS=your-secret-key \
  polymit/phantom:v0.1.0

# Verify it's running
curl http://localhost:8080/health
# {"status":"ok","version":"0.1.0","circuit_breaker":"Closed"}
```

### Full stack with monitoring

```bash
# Clone and launch with Prometheus + Grafana
git clone https://github.com/polymit/phantom.git
cd phantom
cp .env.example .env   # Edit PHANTOM_API_KEYS
docker compose up -d

# MCP server:  http://localhost:8080
# Prometheus:  http://localhost:9090
# Grafana:     http://localhost:3000 (admin / phantom)
```

---

## Connect an Agent (MCP Config)

Add this to your agent's MCP configuration:

```json
{
  "mcpServers": {
    "phantom": {
      "command": "docker",
      "args": ["run", "--rm", "-p", "8080:8080",
               "-e", "PHANTOM_API_KEYS=your-key",
               "polymit/phantom:v0.1.0"],
      "headers": {
        "X-API-Key": "your-key"
      }
    }
  }
}
```

Or connect to a running instance:

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

---

## First Agent Session

```python
# Navigate to a page, read the CCT scene graph, click a button
import json, requests

BASE = "http://localhost:8080/mcp"
HEADERS = {"X-API-Key": "your-key", "Content-Type": "application/json"}

def call(tool, args, session_id=None):
    body = {
        "jsonrpc": "2.0", "method": "tools/call", "id": 1,
        "params": {"name": tool, "arguments": args}
    }
    if session_id:
        body["params"]["session_id"] = session_id
    return requests.post(BASE, json=body, headers=HEADERS).json()

# 1. Navigate
r = call("browser_navigate", {"url": "https://example.com"})
session_id = r["result"]["session_id"]

# 2. Get the scene graph in CCT format (~20 tokens/node)
r = call("browser_get_scene_graph", {"format": "cct"}, session_id)
print(r["result"]["scene_graph"])
# n_0|div|main|0,0,1280,720|b,v,1.0,a|Main|-|-|root|0
# n_1|lnk|lnk|356,288,292,18|b,v,1.0,a|More information...|More information...|c|n_0|1

# 3. Click a link by its CCT node
call("browser_click", {"selector": "a"}, session_id)
```

---

## CCT Format

CCT (Compact Context Text) is Phantom's native output format — designed specifically to minimize tokens consumed by LLMs reading page structure.

Each node is a single pipe-delimited line:

```
id | tag | role | x,y,w,h | flags | text | aria | rel | parent | depth
```

Example output:

```
n_0|div|main|0,0,1280,720|b,v,1.0,a|Main|-|-|root|0
n_1|lnk|lnk|356,288,292,18|b,v,1.0,a|More information...|More information...|c|n_0|1
```

| Field | Description |
| --- | --- |
| `id` | Unique node ID (`n_0`, `n_1` ...) |
| `tag` | Abbreviated HTML tag (`div`, `lnk`, `btn` ...) |
| `role` | ARIA role |
| `x,y,w,h` | Bounding box in pixels |
| `flags` | Visibility, opacity, interactivity (`b`=block, `v`=visible, `a`=active) |
| `text` | Inner text content (truncated at 64 chars) |
| `aria` | Accessible label (`-` if none) |
| `rel` | Relationship hint (`c`=child, `-`=none) |
| `parent` | Parent node ID (`root` for top-level) |
| `depth` | Tree depth (0 = root) |

**~20 tokens/node vs ~121 for raw JSON DOM — 6x reduction.** This directly cuts LLM costs and latency for every page an agent reads.

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

Phantom Engine is a 5-layer Rust workspace:

```
Layer 5 — MCP Server       phantom-mcp         Axum + Tokio, JSON-RPC 2.0
Layer 4 — Session Broker   phantom-session      Isolate pool, circuit breaker, scheduler
           Storage          phantom-storage      SQLite IndexedDB, sled KV, per-session dirs
Layer 3 — Serializer       phantom-serializer   CCT encoder, 2-pass traversal, coalescing
Layer 2 — Core Engine      phantom-core         html5ever, indextree DOM, taffy layout, CSS
           JS Engine        phantom-js           QuickJS (Tier 1) + V8 (Tier 2)
Layer 1 — Network          phantom-net          rquest + BoringSSL, cookie_store
Cross    — Anti-Detection  phantom-anti-detect  Persona pool, JS fingerprint shims
```

### Key Design Decisions

- **No rendering pipeline** — ever. No wgpu, no skia, no pixels. Agents do not have eyeballs.
- **CCT output** — ~20 tokens/node. 6x more efficient than JSON.
- **rquest over reqwest** — BoringSSL for JA4 TLS fingerprint spoofing.
- **Persistence-first navigation** — Full history tracking with `browser_go_back` and `browser_refresh`.
- **Realistic interaction** — `browser_click`, `browser_type`, and `browser_press_key` simulate full event sequences (keydown, mousedown, etc.) with configurable delays to bypass bot detection.
- **Native QuickJS evaluation** — `browser_evaluate` runs JavaScript directly in a native isolate for sub-ms execution.
- **Per-task memory model** — each session's DOM and JS isolate are dropped completely on destroy.
- **Two-tier JS** — QuickJS for 80% of pages (forms, static), V8 for SPAs (React/Vue/Angular).

---

## Development

### Prerequisites

- Rust 1.80+ (`rustup update stable`)
- `cmake`, `clang`, `pkg-config`, `libssl-dev` (for zstd and BoringSSL)

```bash
# Install system deps (Debian/Ubuntu)
sudo apt-get install -y pkg-config libssl-dev cmake clang
```

### Build and Test

```bash
# Build
cargo build --workspace

# Run all tests
cargo test --workspace -- --nocapture

# Run scale test (1000 concurrent sessions)
cargo test --package phantom-mcp test_1000_concurrent_sessions_scale -- --nocapture

# Lint (zero warnings)
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --workspace
```

### Run Locally

```bash
cargo run --bin phantom-mcp -- \
  --port 8080 \
  --api-keys my-dev-key \
  --log-level debug

# With JSON logs (production format)
cargo run --bin phantom-mcp -- \
  --api-keys my-dev-key \
  --json-logs
```

---

## Project Structure

```
phantom/
├── Cargo.toml                  # Workspace root (pinned versions)
├── Dockerfile                  # Multi-stage build (Rust builder → bookworm-slim)
├── docker-compose.yml          # Engine + Prometheus + Grafana
├── prometheus.yml              # Prometheus scrape config
├── .env.example                # All environment variables
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

## MCP Tools Reference

| Category | Tool | Description |
| --- | --- | --- |
| Navigation | `browser_navigate` | Navigate to URL, wait for load event |
| | `browser_go_back` | Browser history back |
| | `browser_go_forward` | Browser history forward |
| | `browser_refresh` | Reload current page |
| Interaction | `browser_click` | Click by selector or (x, y) coordinates |
| | `browser_type` | Type text with per-character key events |
| | `browser_press_key` | Press a specific key (Enter, Tab, Escape…) |
| | `browser_wait_for_selector` | Wait for element to appear (MutationObserver) |
| Perception | `browser_get_scene_graph` | Get CCT scene graph (~20 tokens/node) |
| | `browser_snapshot` | Visual screenshot base64 *(v0.2)* |
| | `browser_evaluate` | Execute JavaScript, get result |
| Tabs | `browser_new_tab` | Open a new tab |
| | `browser_switch_tab` | Switch active tab |
| | `browser_list_tabs` | List all open tabs |
| | `browser_close_tab` | Close a tab |
| Storage | `browser_get_cookies` | Get all cookies |
| | `browser_set_cookie` | Set a specific cookie |
| | `browser_clear_cookies` | Clear all cookies |
| Session | `browser_subscribe_dom` | Stream CCT deltas via SSE |
| | `browser_session_snapshot` | Persist session to disk |
| | `browser_session_clone` | Fork session via V8 snapshot |

---

## Configuration

All configuration via environment variables or CLI flags:

| Variable | Flag | Default | Description |
| --- | --- | --- | --- |
| `PHANTOM_PORT` | `--port` | `8080` | MCP server port |
| `PHANTOM_METRICS_PORT` | `--metrics-port` | `9091` | Prometheus metrics port |
| `PHANTOM_MAX_SESSIONS` | `--max-sessions` | `1000` | Maximum concurrent sessions |
| `PHANTOM_API_KEYS` | `--api-keys` | *(unset)* | Comma-separated API keys |
| `RUST_LOG` | `--log-level` | `info` | Log level |
| `PHANTOM_JSON_LOGS` | `--json-logs` | `false` | JSON structured logging |

Copy `.env.example` to `.env` and configure before running.

---

## Roadmap

- [ ] v0.2 — Python + TypeScript SDK wrappers
- [ ] v0.2 — `browser_fill_form` convenience tool
- [ ] v0.2 — Visual screenshot support (`browser_snapshot`)
- [ ] v0.3 — CCT binary encoding (further token reduction)
- [ ] v0.3 — Cloud-hosted managed sessions
- [ ] v1.0 — APEX: fully managed agent browser platform

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issues and PRs welcome.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE).

---

Phantom Engine is a product of [Polymit](https://github.com/polymit).
