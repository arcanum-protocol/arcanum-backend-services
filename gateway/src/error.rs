use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum AppError {
    InvalidPayloadSize,
    InvalidMpAddress,
    DbIsBusy,
    InvalidResolution,
    MultipoolNotCreated,
    MetadataAlreadySet,
    FailedToParse,
    Unknown(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{self:?}")).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Unknown(err.into().to_string())
    }
}
