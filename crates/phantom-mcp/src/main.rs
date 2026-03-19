//! Phantom Engine v0.1.0 — MCP Server Entry Point.
//!
//! Initializes tracing, parses CLI arguments, starts the MCP server
//! and optional metrics endpoint. Prints the Phantom ASCII banner on startup.

use clap::Parser;
use phantom_mcp::server::McpServer;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ============================================================================
// ASCII Banner
// ============================================================================

const BANNER: &str = r#"
 ██████╗ ██╗  ██╗ █████╗ ███╗   ██╗████████╗ ██████╗ ███╗   ███╗
 ██╔══██╗██║  ██║██╔══██╗████╗  ██║╚══██╔══╝██╔═══██╗████╗ ████║
 ██████╔╝███████║███████║██╔██╗ ██║   ██║   ██║   ██║██╔████╔██║
 ██╔═══╝ ██╔══██║██╔══██║██║╚██╗██║   ██║   ██║   ██║██║╚██╔╝██║
 ██║     ██║  ██║██║  ██║██║ ╚████║   ██║   ╚██████╔╝██║ ╚═╝ ██║
 ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝     ╚═╝

 ███████╗███╗   ██╗ ██████╗ ██╗███╗   ██╗███████╗
 ██╔════╝████╗  ██║██╔════╝ ██║████╗  ██║██╔════╝
 █████╗  ██╔██╗ ██║██║  ███╗██║██╔██╗ ██║█████╗
 ██╔══╝  ██║╚██╗██║██║   ██║██║██║╚██╗██║██╔══╝
 ███████╗██║ ╚████║╚██████╔╝██║██║ ╚████║███████╗
 ╚══════╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝╚═╝  ╚═══╝╚══════╝

  v0.1.0 — Purpose-built browser engine for AI agents
  MCP Protocol · Zero rendering · ~20 tokens/node · 1000+ concurrent sessions
"#;

// ============================================================================
// CLI Arguments
// ============================================================================

/// Phantom Engine MCP Server — Agentic browser engine over MCP protocol.
#[derive(Parser, Debug)]
#[command(
    name = "phantom-mcp",
    author = "Phantom Engine Team",
    version = "0.1.0",
    about = "Purpose-built browser engine for AI agents",
    long_about = None
)]
struct Args {
    /// Port for the MCP/HTTP server.
    #[arg(short, long, default_value_t = 8080, env = "PHANTOM_PORT")]
    port: u16,

    /// Port for the Prometheus metrics endpoint.
    #[arg(long, default_value_t = 9091, env = "PHANTOM_METRICS_PORT")]
    metrics_port: u16,

    /// Maximum number of concurrent sessions.
    #[arg(long, default_value_t = 1000, env = "PHANTOM_MAX_SESSIONS")]
    max_sessions: usize,

    /// API keys for authentication (comma-separated).
    #[arg(short, long, default_value = "", env = "PHANTOM_API_KEYS")]
    api_keys: String,

    /// Log level: error, warn, info, debug, or trace.
    #[arg(short, long, default_value = "info", env = "RUST_LOG")]
    log_level: String,

    /// Emit logs as JSON (for production / log aggregators).
    #[arg(long, default_value_t = false, env = "PHANTOM_JSON_LOGS")]
    json_logs: bool,
}

// ============================================================================
// Entry Point
// ============================================================================

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize tracing with JSON or pretty format
    init_tracing(&args.log_level, args.json_logs);

    // Print banner to stdout (not tracing — this is intentional branding)
    println!("{BANNER}");

    tracing::info!(
        port = args.port,
        metrics_port = args.metrics_port,
        max_sessions = args.max_sessions,
        "Phantom Engine v0.1.0 initializing"
    );

    // Parse and validate API keys
    let keys: Vec<String> = args
        .api_keys
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if keys.is_empty() {
        tracing::warn!(
            "No API keys configured — server is UNPROTECTED. Set PHANTOM_API_KEYS for production."
        );
    } else {
        tracing::info!(key_count = keys.len(), "API key authentication enabled");
    }

    // Build server (initializes SessionBroker, metrics, isolate pool)
    let server = Arc::new(McpServer::new(keys));

    tracing::info!(max_sessions = args.max_sessions, "Session broker ready");

    // Start the MCP server (blocks until shutdown)
    tracing::info!(port = args.port, "MCP server starting");
    server.start(args.port).await;
}

// ============================================================================
// Tracing Initialization
// ============================================================================

/// Initialize the global tracing subscriber.
///
/// In JSON mode (production), emits machine-readable structured JSON.
/// In pretty mode (development), emits human-readable terminal output.
fn init_tracing(log_level: &str, json_logs: bool) {
    let env_filter = tracing_subscriber::EnvFilter::try_new(log_level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    if json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_ansi(true))
            .init();
    }
}
