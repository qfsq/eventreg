use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Error type returned by HTTP handlers.
///
/// Internal details are logged but never sent to the client, which avoids
/// leaking SQL statements or connection strings in API responses.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl ApiError {
    pub fn internal(context: &'static str, error: impl std::fmt::Display) -> Self {
        tracing::error!(%error, context, "request failed");
        Self::Internal("internal server error".to_owned())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::internal("database error", error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
