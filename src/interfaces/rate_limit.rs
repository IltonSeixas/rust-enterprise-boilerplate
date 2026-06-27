use std::time::Duration;

use anyhow::Context;
use governor::middleware::NoOpMiddleware;
use tower_governor::key_extractor::PeerIpKeyExtractor;
use tower_governor::{governor::GovernorConfig, governor::GovernorConfigBuilder};

/// Builds the per-IP token bucket configuration shared by the REST and gRPC
/// rate limiters, so both transports enforce the same policy.
pub fn build_governor_config(
    rate_limit_per_second: u64,
    rate_limit_burst: u32,
) -> anyhow::Result<GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware>> {
    let rate_limit_per_second = rate_limit_per_second.max(1) as u32;
    GovernorConfigBuilder::default()
        .key_extractor(PeerIpKeyExtractor)
        .period(Duration::from_secs(1) / rate_limit_per_second)
        .burst_size(rate_limit_burst)
        .finish()
        .context("invalid rate limiter configuration")
}
