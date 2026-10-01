use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io: {0}")]   Io(#[from] std::io::Error),
    #[error("json: {0}")] Json(#[from] serde_json::Error),
    #[error("not found: {0}")] NotFound(String),
    #[error("invalid input: {0}")] InvalidInput(String),
    #[error("schema version unsupported: {0}")] UnsupportedSchema(u32),
    #[error("dialog cancelled")] Cancelled,
}

pub type Result<T> = std::result::Result<T, Error>;
