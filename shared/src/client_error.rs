use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Other error: {0}")]
    Other(String),
}
