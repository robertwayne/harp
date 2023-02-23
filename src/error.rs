use sqlx::migrate::MigrateError;

#[derive(Debug)]
pub enum HarpError {
    ConnectionFailed,
    QueueFull,
    BadIdentifier(String),
    Internal(std::io::Error),
    Database(sqlx::Error),
    Parse(serde_json::Error),
}

impl std::error::Error for HarpError {}

impl From<std::io::Error> for HarpError {
    fn from(err: std::io::Error) -> Self {
        HarpError::Internal(err)
    }
}

impl std::fmt::Display for HarpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarpError::ConnectionFailed => write!(f, "connection failed"),
            HarpError::QueueFull => write!(f, "action queue is full"),
            HarpError::BadIdentifier(e) => {
                write!(f, "[Bad Identifier]: {e}")
            }
            HarpError::Internal(e) => write!(f, "[Internal] {e}"),
            HarpError::Database(e) => write!(f, "[Database] {e}"),
            HarpError::Parse(e) => write!(f, "[Parse] {e}"),
        }
    }
}

impl From<MigrateError> for HarpError {
    fn from(err: MigrateError) -> Self {
        HarpError::Database(err.into())
    }
}

impl From<sqlx::Error> for HarpError {
    fn from(err: sqlx::Error) -> Self {
        HarpError::Database(err)
    }
}

impl From<serde_json::Error> for HarpError {
    fn from(err: serde_json::Error) -> Self {
        HarpError::Parse(err)
    }
}
