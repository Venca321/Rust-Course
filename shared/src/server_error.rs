use bincode::ErrorKind;
use sqlx::Error as SqlxError;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error")]
    Io(#[from] io::Error),

    #[error("SQLx error")]
    Sqlx(#[from] SqlxError),

    #[error("Bincode error")]
    Bincode(#[from] Box<ErrorKind>),

    #[error("Other error: {0}")]
    Other(String),
}
