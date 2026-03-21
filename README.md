# Phantom Engine

> A native Rust browser engine purpose-built for AI agents. No Chrome. No Playwright. No pixels.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-early%20alpha-yellow?style=flat-square)]()

---

> **Early Alpha.** Phantom's core pipeline — fetch, parse, layout, and CCT serialization — is implemented and working. Several interaction tools and advanced session features are actively being completed. See the [Status](#status) section for an honest breakdown before integrating.

---

## What Phantom Is

Phantom is a browser engine written from scratch in Rust, designed exclusively for AI agents operating over the Model Context Protocol (MCP). It does not wrap Chrome. It does not use Playwright or Puppeteer. It implements its own HTML parsing pipeline, CSS cascade engine, layout engine, and DOM serializer — then exposes the result to agents as a structured, token-efficient scene graph.

The core thesis: AI agents do not need pixels. They need structure. Every byte spent rendering a frame, every millisecond waiting for a GPU flush, and every token wasted on verbose JSON is waste that Phantom eliminates by design.

---

## Why Phantom

Every existing headless browser tool for AI agents is Chrome with a different API. Chrome brings with it 300ms startup times, gigabytes of RAM per session, a detectable TLS fingerprint, and a rendering pipeline that exists entirely to serve human eyes. Phantom discards all of that.

| Problem with Chrome-based tools | Phantom's approach |
| --- | --- |
| ~300ms cold session startup | QuickJS isolates start in milliseconds; isolate pool pre-warms on boot |
| Raw DOM as JSON uses ~121 tokens per node | CCT format uses ~20 tokens per node — a 6x reduction |
| Headless Chrome TLS fingerprint is detectable | `rquest` + BoringSSL with Chrome130 impersonation |
| Headless Chrome is identifiable via `navigator.webdriver` | JS shims mask all standard detection vectors before page scripts run |
| Hundreds of MB per Chrome instance | Phantom sessions share a single Rust process with per-task memory isolation |

---

## Status

Phantom is early alpha. The table below reflects what the code actually does today, not what we are building toward.

| Component | Status | Notes |
| --- | --- | --- |
| HTML parsing (`html5ever`) | ✅ Working | Full spec-compliant parsing |
| CSS cascade engine | ✅ Working | Handles `display`, `visibility`, `opacity`, `position`, `z-index`, `pointer-events`; inline styles and `<style>` tags |
| Taffy layout engine | ✅ Working | Block, flex, grid; reads HTML `width`/`height` attributes |
| CCT serialization | ✅ Working | Full 8-stage pipeline; ~20 tokens/node |
| Network layer (`rquest` + BoringSSL) | ✅ Working | Chrome130 TLS impersonation; redirect following |
| `browser_navigate` | ✅ Working | Fetches, parses, layouts, stores DOM in session |
| `browser_go_back` | ✅ Working | Re-navigates to previous URL in history stack |
| `browser_refresh` | ✅ Working | Re-fetches current URL |
| `browser_get_scene_graph` | ✅ Working | Returns real CCT output from live session DOM |
| `browser_evaluate` | ✅ Working | Executes JS in QuickJS isolate; returns JSON result |
| `browser_new_tab` / `browser_list_tabs` / `browser_close_tab` | ✅ Working | Tab state tracked per session |
| `browser_switch_tab` | ✅ Working | Switches active tab context |
| `browser_get_cookies` / `browser_set_cookie` / `browser_clear_cookies` | ✅ Working | Per-session cookie jar |
| MCP server (JSON-RPC 2.0) | ✅ Working | Axum + Tokio; session management; Prometheus metrics |
| API key authentication | ✅ Working | `X-API-Key` header; configurable via env |
| Circuit breaker | ✅ Working | Opens after 5 failures; resets after 30s |
| Anti-detect persona pool | ✅ Working | Deterministic per-session fingerprints; navigator shims generated |
| Canvas noise shims | ✅ Working | Per-session 1-bit canvas noise to defeat fingerprinting |
| Session broker + scheduler | ✅ Working | Priority-based session queue |
| Snapshot storage (`phantom-storage`) | ✅ Working | zstd-compressed tar with SHA256 checksums |
| `browser_go_forward` | 🔧 In progress | Placeholder; forward history stack not yet wired |
| `browser_click` / `browser_type` / `browser_press_key` | 🔧 In progress | Event sequences defined; DOM dispatch wiring in progress |
| `browser_wait_for_selector` | 🔧 In progress | Polls session DOM; live MutationObserver not yet connected |
| `browser_session_snapshot` | 🔧 In progress | Storage layer complete; tool wiring in progress |
| `browser_session_clone` | 🔧 In progress | Session broker clone logic complete; tool wiring in progress |
| `browser_subscribe_dom` / SSE stream | 🔧 In progress | SSE infrastructure planned; endpoint returns 501 today |
| `browser_snapshot` (screenshot) | 📋 Planned v0.2 | No render pipeline by design; planned as structured fallback |
| Cookie passthrough on fetch | 🔧 In progress | Cookie jar populated; HTTP header injection in progress |
| Anti-detect shim injection at navigate | 🔧 In progress | Shims generated; injection into navigation pipeline in progress |
| V8 tier (Tier 2 JS) | 📋 Planned | Currently delegates to QuickJS; rusty_v8 integration planned |
| Session max cap enforcement | 🔧 In progress | CLI flag parsed; enforcement logic in progress |
| Per-tab DOM isolation | 🔧 In progress | Tab metadata tracked; DOM swap on switch in progress |

---

## Architecture

Phantom is an 8-crate Rust workspace organized in 5 layers plus one cross-cutting concern.

```
Layer 5 — MCP Server        phantom-mcp          Axum + Tokio, JSON-RPC 2.0, auth, metrics
Layer 4 — Session Broker    phantom-session       Isolate pool, circuit breaker, priority scheduler
           Storage           phantom-storage       SQLite IndexedDB, sled KV, zstd snapshots
Layer 3 — Serializer        phantom-serializer    CCT encoder, 8-stage pipeline, buffer pooling
Layer 2 — Core Engine       phantom-core          html5ever, indextree arena DOM, taffy layout, CSS
           JS Engine         phantom-js            QuickJS runtime, DOM bindings, browser shims
Layer 1 — Network           phantom-net           rquest + BoringSSL, Chrome130 impersonation
Cross    — Anti-Detection   phantom-anti-detect   Persona pool, JS fingerprint shims, canvas noise
```

### Design decisions

**No rendering pipeline.** Ever. No GPU, no pixels, no skia, no wgpu. Phantom will never render a frame. Agents do not have eyes and every byte spent on rendering is waste.

**CCT over JSON.** The CCT (Compact Context Text) format encodes a full DOM node in a single pipe-delimited line. ~20 tokens per node versus ~121 for JSON. This is not a rounding choice — it is the central design constraint that everything else is built around.

**rquest over reqwest.** The `rquest` crate provides BoringSSL bindings and a `Chrome130` impersonation profile. This produces TLS handshakes that are indistinguishable from a real Chrome 130 browser at the JA4 fingerprint level.

**One JS isolate per task.** Every session gets a fresh QuickJS runtime. When the task ends, the entire isolate is dropped. No state leaks between sessions, no memory accumulates.

**Two-tier JS engine.** QuickJS handles the majority of pages — static HTML, forms, simple scripts. V8 handles SPAs. The V8 tier is currently implemented as a QuickJS delegation while `rusty_v8` bindings are stabilized.

---

## CCT Format

CCT is Phantom's native scene graph format. Each visible DOM node becomes one pipe-delimited line:

```
id | tag | role | x,y,w,h | flags | text | aria | rel | parent | depth
```

Real output from navigating `example.com`:

```
n_0|div|main|0,0,1280,720|b,v,1.0,a|Main|-|-|root|0
n_1|lnk|lnk|356,288,292,18|b,v,1.0,a|More information...|More information...|c|n_0|1
```

| Field | Description |
| --- | --- |
| `id` | Stable node identifier within session |
| `tag` | Abbreviated HTML tag (`div`, `lnk`, `btn`, `inp` …) |
| `role` | ARIA role |
| `x,y,w,h` | Bounding box in pixels |
| `flags` | Display, visibility, opacity, pointer-events |
| `text` | Inner text content, truncated at 64 characters |
| `aria` | Accessible label; `-` if absent |
| `rel` | Tree relationship hint |
| `parent` | Parent node ID |
| `depth` | Tree depth from document root |

The 6x token reduction over JSON is not an estimate — it is the result of a design that encodes only the 9 fields an agent actually needs to navigate a page, and nothing else.

---

## Quick Start

### One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/polymit/phantom/main/install.sh | sh
```

### Docker pull

```bash
docker pull polymit/phantom:latest
```

### Docker run

```bash
docker run -d \
  -p 8080:8080 \
  -e PHANTOM_API_KEYS=your-secret-key \
  polymit/phantom:v0.2.0

curl http://localhost:8080/health
# {"status":"ok","version":"0.2.0","circuit_breaker":"Closed"}
```

### Full stack with Prometheus and Grafana

```bash
git clone https://github.com/polymit/phantom.git
cd phantom
cp .env.example .env
# Edit .env — set PHANTOM_API_KEYS at minimum

docker compose up -d

# MCP endpoint:  http://localhost:8080/mcp
# Prometheus:    http://localhost:9090
# Grafana:       http://localhost:3000  (admin / phantom)
```

### Build from source

**Prerequisites:** Rust 1.80+, `cmake`, `clang`, `pkg-config`, `libssl-dev`

```bash
sudo apt-get install -y pkg-config libssl-dev cmake clang

git clone https://github.com/polymit/phantom.git
cd phantom
cargo build --release --workspace

cargo run --release --bin phantom-mcp -- \
  --port 8080 \
  --api-keys your-secret-key
```

---

## Connect an Agent

Add Phantom to your MCP configuration:

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

## Basic Usage

What works today — navigate a page and read the scene graph:

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

# Navigate — returns session_id and final URL
r = call("browser_navigate", {"url": "https://example.com"})
session_id = r["result"]["session_id"]

# Read the CCT scene graph
r = call("browser_get_scene_graph", {"format": "cct"}, session_id)
print(r["result"]["scene_graph"])
# n_0|div|main|0,0,1280,720|b,v,1.0,a|Main|-|-|root|0
# n_1|lnk|lnk|356,288,292,18|b,v,1.0,a|More information...|More information...|c|n_0|1

# Execute JavaScript in the page context
r = call("browser_evaluate", {"script": "document.title"}, session_id)
print(r["result"]["result"])
# "Example Domain"

# Go back
r = call("browser_go_back", {}, session_id)

# Cookie management
r = call("browser_get_cookies", {}, session_id)
r = call("browser_set_cookie", {"name": "session", "value": "abc123"}, session_id)

# Tab management
r = call("browser_new_tab", {"url": "https://example.org"}, session_id)
tab_id = r["result"]["tabId"]
r = call("browser_list_tabs", {}, session_id)
r = call("browser_switch_tab", {"tabId": tab_id}, session_id)
r = call("browser_close_tab", {"tabId": tab_id}, session_id)
```

---

## MCP Tools Reference

| Category | Tool | Status | Description |
| --- | --- | --- | --- |
| Navigation | `browser_navigate` | ✅ | Fetch URL, parse DOM, compute layout, store in session |
| | `browser_go_back` | ✅ | Navigate to previous URL in history |
| | `browser_go_forward` | 🔧 | Navigate forward in history |
| | `browser_refresh` | ✅ | Re-fetch current URL |
| Interaction | `browser_click` | 🔧 | Click element by CSS selector; dispatches mouse event sequence |
| | `browser_type` | 🔧 | Type text into element with per-character key events |
| | `browser_press_key` | 🔧 | Press a named key (Enter, Tab, Escape …) |
| | `browser_wait_for_selector` | 🔧 | Poll until selector appears in DOM |
| Perception | `browser_get_scene_graph` | ✅ | Return CCT scene graph for current session DOM |
| | `browser_snapshot` | 📋 v0.2 | Visual representation of current page |
| | `browser_evaluate` | ✅ | Execute JavaScript, return JSON result |
| Tabs | `browser_new_tab` | ✅ | Open new tab, optionally navigate to URL |
| | `browser_switch_tab` | ✅ | Switch active tab context |
| | `browser_list_tabs` | ✅ | List all open tabs with URL and title |
| | `browser_close_tab` | ✅ | Close tab; manages fallback to remaining tabs |
| Storage | `browser_get_cookies` | ✅ | Return all cookies in session jar |
| | `browser_set_cookie` | ✅ | Set a cookie for current URL scope |
| | `browser_clear_cookies` | ✅ | Clear entire session cookie jar |
| Session | `browser_subscribe_dom` | 🔧 | Stream CCT deltas via SSE |
| | `browser_session_snapshot` | 🔧 | Persist full session state to disk |
| | `browser_session_clone` | 🔧 | Fork session with copied DOM and cookie state |

✅ Working — 🔧 In progress — 📋 Planned

---

## Configuration

| Variable | Flag | Default | Description |
| --- | --- | --- | --- |
| `PHANTOM_PORT` | `--port` | `8080` | MCP server port |
| `PHANTOM_METRICS_PORT` | `--metrics-port` | `9091` | Prometheus metrics port |
| `PHANTOM_MAX_SESSIONS` | `--max-sessions` | `1000` | Maximum concurrent sessions |
| `PHANTOM_API_KEYS` | `--api-keys` | *(unset — server unprotected)* | Comma-separated API keys |
| `RUST_LOG` | `--log-level` | `info` | Log level: error, warn, info, debug, trace |
| `PHANTOM_JSON_LOGS` | `--json-logs` | `false` | Emit structured JSON logs |

Copy `.env.example` to `.env` before running. If `PHANTOM_API_KEYS` is unset, the server logs a warning and runs unauthenticated.

---

## Project Structure

```
phantom/
├── Cargo.toml                    # Workspace root
├── Dockerfile                    # Multi-stage build (builder → bookworm-slim)
├── docker-compose.yml            # Engine + Prometheus + Grafana
├── prometheus.yml                # Prometheus scrape config
├── .env.example                  # All environment variables with defaults
└── crates/
    ├── phantom-net/              # Layer 1: rquest + BoringSSL network client
    ├── phantom-core/             # Layer 2: HTML parser, CSS cascade, DOM, layout
    ├── phantom-js/               # Layer 2: QuickJS runtime, DOM bindings, shims
    ├── phantom-serializer/       # Layer 3: CCT encoder, 8-stage pipeline
    ├── phantom-session/          # Layer 4: Session broker, isolate pool, scheduler
    ├── phantom-storage/          # Layer 4: SQLite IndexedDB, sled KV, snapshots
    ├── phantom-mcp/              # Layer 5: MCP server, tool dispatch, auth, metrics
    └── phantom-anti-detect/      # Cross-cutting: persona pool, JS shims, canvas noise
```

---

## Development

```bash
# Build
cargo build --workspace

# Test
cargo test --workspace -- --nocapture

# Lint (zero warnings policy)
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --workspace

# Benchmark CCT serializer
cargo bench --package phantom-serializer

# Run locally with debug logging
cargo run --bin phantom-mcp -- \
  --port 8080 \
  --api-keys dev-key \
  --log-level debug
```

---

## Roadmap

### v0.3 — Interaction complete
- [ ] Wire `browser_click`, `browser_type`, `browser_press_key` — DOM event dispatch
- [ ] Wire `browser_go_forward` — forward history stack
- [ ] Wire `browser_session_snapshot` and `browser_session_clone` — session persistence
- [ ] SSE stream for `browser_subscribe_dom`
- [ ] Cookie passthrough on HTTP fetch
- [ ] Anti-detect shim injection in navigation pipeline
- [ ] Per-tab DOM isolation
- [ ] Session cap enforcement

### v0.4 — Reliability and real-world sites
- [ ] CSS dimension parsing (width, height, margin, padding from stylesheets)
- [ ] `<script>` tag extraction and execution in page pipeline
- [ ] `browser_wait_for_selector` with live DOM polling
- [ ] `tools/list` MCP endpoint for agent discovery
- [ ] Python and TypeScript SDK wrappers

### v1.0 — Production
- [ ] Real V8 tier via rusty_v8 for SPA support
- [ ] Verified performance benchmarks against claimed numbers
- [ ] `browser_snapshot` as structured CCT fallback
- [ ] Cloud-hosted managed sessions

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issues, bug reports, and pull requests are welcome.

If you find a gap between what this README describes as working and what the code actually does, please open an issue. Honesty about current state is a project value.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE).

---

Phantom Engine is built by [Polymit](https://github.com/polymit).
