use actix_web::{http::StatusCode, ResponseError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not found")]
    NotFound,

    #[error("Resource already exists and overwrite=false")]
    AlreadyExists,

    #[error("Invalid ics/vcf input: {0}")]
    InvalidData(String),

    #[error("Read-only")]
    ReadOnly,

    #[error("Error generating password hash")]
    PasswordHash,

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Unauthorized")]
    UserNotFound,

    #[error(transparent)]
    ParserError(#[from] ical::parser::ParserError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::AlreadyExists => StatusCode::CONFLICT,
            Self::InvalidData(_) => StatusCode::BAD_REQUEST,
            Self::ReadOnly => StatusCode::FORBIDDEN,
            Self::UserNotFound => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
