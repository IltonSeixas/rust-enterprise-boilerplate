use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

pub fn init_prometheus() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
}
