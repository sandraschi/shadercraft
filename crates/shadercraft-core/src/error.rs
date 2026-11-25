//! Error types for ShaderCraft L1 core.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("Texture node or graph not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Feedback loop detected in texture graph: {0}")]
    CycleDetected(String),

    #[error("SQLite storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Image output error: {0}")]
    Image(String),
}
