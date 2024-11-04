mod buffer;
mod error;
mod metrics;
pub mod net;

pub use buffer::RingBuffer;
pub use error::{BrokerError, NetworkError};
pub use metrics::Metrics;
pub use net::{BrokerClient, BrokerServer};

pub(crate) const CACHE_LINE_SIZE: usize = 64;
pub(crate) const RING_BUFFER_SIZE: usize = 256 * 1024 * 1024; 
pub(crate) const BATCH_SIZE: usize = 1024; 
pub(crate) const BUFFER_CHUNK: usize = 128 * 1024; 
