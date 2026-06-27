use metrics_exporter_prometheus::{BuildError, PrometheusBuilder, PrometheusHandle};

pub fn init_prometheus() -> Result<PrometheusHandle, BuildError> {
    PrometheusBuilder::new().install_recorder()
}
