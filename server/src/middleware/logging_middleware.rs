use crate::infra::error::ErrorMessage;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use http::StatusCode;
use log::warn;

pub async fn error_logging_middleware(req: Request, next: Next) -> Response {
    let uri = req.uri().clone();
    let response = next.run(req).await;
    if response.status().is_success() || response.status() == StatusCode::NOT_MODIFIED {
        return response;
    }
    let error_message = response
        .extensions()
        .get::<ErrorMessage>()
        .map(|e| e.error.to_string())
        .unwrap_or_else(|| "Unknown error".to_string());
    warn!(
        "HTTP Error {} for '{}': {}",
        response.status().as_u16(),
        uri,
        error_message
    );
    response
}
