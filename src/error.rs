use sqlx::migrate::MigrateError;

// TODO: Rewrite error handling completely.
#[derive(Debug)]
pub enum HarpError {
    ConnectionFailed,
    QueueFull,
    BadIdentifier(String),
    Internal(std::io::Error),
    Database(sqlx::Error),
    Json(serde_json::Error),
    Time(time::error::Parse),
    AddrParse(std::net::AddrParseError),
    FileNotFound,
    #[cfg(feature = "bin")]
    ArgsParse(pico_args::Error),
    #[cfg(feature = "bin")]
    Toml(toml::de::Error),
}

impl std::error::Error for HarpError {}

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
            HarpError::Json(e) => write!(f, "[JSON] {e}"),
            HarpError::Time(e) => write!(f, "[Time] {e}"),
                        HarpError::AddrParse(e) => write!(f, "[AddrParse] {e}"),
            HarpError::FileNotFound => write!(f, "No config file found. The default location is /etc/harp.toml. You can specify a different location with the -c flag."),
            #[cfg(feature = "bin")]
            HarpError::Toml(e) => write!(f, "[Toml] {e}"),
            #[cfg(feature = "bin")]
            HarpError::ArgsParse(e) => write!(f, "[ArgsParse] {e}"),
        }
    }
}

impl From<std::io::Error> for HarpError {
    fn from(err: std::io::Error) -> Self {
        HarpError::Internal(err)
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
        HarpError::Json(err)
    }
}

impl From<time::error::Parse> for HarpError {
    fn from(err: time::error::Parse) -> Self {
        HarpError::Time(err)
    }
}

impl From<std::net::AddrParseError> for HarpError {
    fn from(err: std::net::AddrParseError) -> Self {
        HarpError::AddrParse(err)
    }
}

#[cfg(feature = "bin")]
impl From<toml::de::Error> for HarpError {
    fn from(err: toml::de::Error) -> Self {
        HarpError::Toml(err)
    }
}

#[cfg(feature = "bin")]
impl From<pico_args::Error> for HarpError {
    fn from(err: pico_args::Error) -> Self {
        HarpError::ArgsParse(err)
    }
}
