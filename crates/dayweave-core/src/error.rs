use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request itself is wrong. The message is written for the person using the application.
    #[error("{0}")]
    Invalid(String),
    #[error("{0} no longer exists")]
    NotFound(&'static str),
    #[error("This database was created by a newer version of Dayweave. Update Dayweave to open it")]
    DatabaseTooNew,
    #[error("Unknown operation: {0}")]
    UnknownMethod(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid data: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Error::Invalid(message.into())
    }

    pub fn code(&self) -> &'static str {
        match self {
            Error::Invalid(_) => "invalid",
            Error::NotFound(_) => "not_found",
            Error::DatabaseTooNew => "database_too_new",
            Error::UnknownMethod(_) => "unknown_method",
            Error::Database(_) => "database",
            Error::Io(_) => "io",
            Error::Json(_) => "json",
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Error", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
