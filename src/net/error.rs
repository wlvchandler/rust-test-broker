use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Broker error: {0}")]
    Broker(#[from] crate::BrokerError),
}

