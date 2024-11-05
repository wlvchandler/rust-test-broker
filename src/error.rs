use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum BrokerError {
    #[error("buffer full")]
    BufferFull,

    #[error("buffer empty")]
    BufferEmpty,

    #[error("system error: {0}")]
    SystemError(#[from] std::io::Error),

    #[error("buffer size too small")]
    BufferTooSmall,

    #[error("message size too large")]
    MessageTooLarge,
}

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Broker error: {0}")]
    Broker(#[from] BrokerError),

    #[error("Join error {0}")]
    Join(#[from] JoinError),
}
