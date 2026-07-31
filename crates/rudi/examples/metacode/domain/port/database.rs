#[derive(Debug, thiserror::Error)]
#[error("database error: {0}")]
pub struct DatabaseError(pub String);

pub trait DatabasePort: Send + Sync {
    fn ping(&self) -> Result<(), DatabaseError>;
}
