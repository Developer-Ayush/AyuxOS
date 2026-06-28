#[derive(Debug, thiserror::Error)]
pub enum HalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Device not found: {0}")]
    NotFound(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type HalResult<T> = Result<T, HalError>;
