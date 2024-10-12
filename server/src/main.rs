use axum::body::{Body, Bytes};
use axum::extract::{Path, Request};
use axum::middleware::Next;
use axum::response::{ErrorResponse, IntoResponse, Response};
use axum::{http::StatusCode, middleware, routing::get, Json, Router};
use felico_compiler::infra::result::{FelicoError, FelicoResult};
use http::{HeaderName, HeaderValue};
use http_body_util::{BodyExt, Limited};
use log::{info, warn};
use serde::Serialize;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();
    info!("Starting up;");
    let serve_index_html = ServeFile::new("./docui/dist/index.html");
    let serve_dir =
        ServeDir::new("./docui/dist/assets").not_found_service(serve_index_html.clone());

    // build our application with a route
    let app = Router::new()
        .nest_service("/", serve_index_html)
        .route("/foo", get(|| async { "Hi from /foo" }))
        .nest_service("/assets", serve_dir.clone())
        .fallback_service(serve_dir)
        .route("/api/packages", get(get_packages))
        .route("/api/packages/:package_name/:package_version", get(get_package))
        .layer(middleware::from_fn(error_logging_middleware))
        // end
        ;

    // run our app with hyper, listening globally on port 3000
    let listen_address = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(listen_address).await.unwrap();
    info!("Listening on {listen_address}");
    axum::serve(listener, app).await.unwrap();
}

async fn error_logging_middleware(req: Request, next: Next) -> Response {
    let uri = req.uri().clone();
    let response = next.run(req).await;
    if response.status().is_success() || response.status() == StatusCode::NOT_MODIFIED {
        return response;
    }
    let error_message = response
        .extensions()
        .get::<ErrorMessage>()
        .map(|e| e.0.to_string())
        .unwrap_or_else(|| "Unknown error".to_string());
    warn!(
        "HTTP Error {} for {}: {}",
        response.status(),
        uri,
        error_message
    );
    response
}

#[derive(Debug)]
pub struct ServerError(FelicoError);

#[derive(Debug, Clone)]
pub struct ErrorMessage(String);

impl<T: Into<FelicoError>> From<T> for ServerError {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let mut response = (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response();
        response
            .extensions_mut()
            .insert(ErrorMessage(self.0.to_string()));
        response
    }
}

async fn get_package(
    Path((package_name, package_version)): Path<(String, String)>,
) -> Result<Json<PackageDescription>, ServerError> {
    //    Err("foo".into())
    Ok(Json(PackageDescription {
        info: PackageInfo {
            name: package_name,
            version: package_version,
            /*
            functions: vec![FunctionDescription {
                name: "debug_print".to_string(),
                signature: "fun debug_print(item: any)".to_string(),
            }],*/
        },
        functions: vec![FunctionDescription {
            name: "debug_print".to_string(),
            signature: "fun debug_print(item: any)".to_string(),
        }],
    }))
}

async fn get_packages() -> (StatusCode, Json<PackageIndex>) {
    let package_index = PackageIndex {
        packages: vec![PackageInfo {
            name: "std".to_string(),
            version: "0.0.1".to_string(),
        }],
    };
    (StatusCode::CREATED, Json(package_index))
}

#[derive(Serialize)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct PackageDescription {
    info: PackageInfo,
    functions: Vec<FunctionDescription>,
}

#[derive(Serialize)]
struct FunctionDescription {
    name: String,
    signature: String,
}

#[derive(Serialize)]
struct PackageIndex {
    packages: Vec<PackageInfo>,
}
