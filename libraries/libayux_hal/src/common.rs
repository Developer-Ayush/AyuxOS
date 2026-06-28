use thiserror::Error;

#[derive(Debug, Error)]
pub enum HalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("IO error: {0}")]
    IOError(String),
    #[error("Device not found: {0}")]
    NotFound(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Hardware error: {0}")]
    HardwareError(String),
}

pub type HalResult<T> = Result<T, HalError>;
