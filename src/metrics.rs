use std::time::Duration;

#[derive(Debug)]
pub struct Metrics {
    pub min: Duration,
    pub p50: Duration,
    pub p99: Duration,
    pub p99_9: Duration,
    pub max: Duration,
    pub msgs_per_sec: f64,
    pub mb_per_sec: f64,
}

impl Metrics {
    /// summary from raw latency measurements
    pub fn from_measurements(latencies: &mut Vec<Duration>, total_bytes: usize, duration: Duration) -> Self {
        latencies.sort_unstable();
        let len = latencies.len();
        let bytes_per_sec = (total_bytes as f64) / duration.as_secs_f64();
        let mb_per_sec = bytes_per_sec / (1024.0 * 1024.0);
        Metrics {
            min: latencies[0],
            p50: latencies[len / 2],
            p99: latencies[len * 99 / 100],
            p99_9: latencies[len * 999 / 1000],
            max: latencies[len - 1],
            msgs_per_sec: len as f64 / duration.as_secs_f64(),
            mb_per_sec,
        }
    }
}
