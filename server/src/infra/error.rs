use axum::response::{IntoResponse, Response};
use axum::Json;
use felico_compiler::infra::result::FelicoError;
use http::StatusCode;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ErrorMessage {
    pub error: String,
}

#[derive(Debug)]
pub struct ServerError(FelicoError);

impl<T: Into<FelicoError>> From<T> for ServerError {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let error_message = ErrorMessage {
            error: self.0.to_string(),
        };
        let mut response = (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(error_message.clone()),
        )
            .into_response();
        response.extensions_mut().insert(error_message);
        *response.status_mut() = StatusCode::from_u16(599).unwrap();
        response
    }
}

async fn test_error() -> Result<Json<crate::http::start_server::PackageDescription>, ServerError> {
    Err("test error, please ignore".into())
}
