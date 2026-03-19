use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Initialize the Prometheus metrics exporter and return its handle.
///
/// This sets up a global recorder that collects metrics from all crates.
pub fn init_metrics() -> PrometheusHandle {
    let builder = PrometheusBuilder::new();

    // Set a default bucket for histograms (e.g., latency in ms)
    let builder = builder
        .set_buckets(&[
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
        .expect("failed to set metrics buckets");

    builder
        .install_recorder()
        .expect("failed to install prometheus recorder")
}

/// Convenience macro for recording latency in milliseconds.
#[macro_export]
macro_rules! record_latency {
    ($name:expr, $start:expr) => {
        metrics::histogram!($name).record($start.elapsed().as_secs_f64() * 1000.0);
    };
}
