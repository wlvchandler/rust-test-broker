use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrokerError {
    #[error("buffer full")]
    BufferFull,
    
    #[error("system error: {0}")]
    SystemError(#[from] std::io::Error),
    
    #[error("buffer size too small")]
    BufferTooSmall,
    
    #[error("message size too large")]
    MessageTooLarge,
}
