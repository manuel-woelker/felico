use axum::body::{Body, Bytes};
use axum::extract::{Path, Request};
use axum::middleware::Next;
use axum::response::{ErrorResponse, Response};
use axum::{http::StatusCode, middleware, routing::get, Json, Router};
use http::{HeaderName, HeaderValue};
use http_body_util::{BodyExt, Limited};
use log::{info, warn};
use serde::Serialize;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::Span;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();
    info!("Starting up;");
    let serve_dir = ServeDir::new("./docui/dist/assets")
        .not_found_service(ServeFile::new("./docui/dist/index.html"));
    // build our application with a route
    let app = Router::new()
        .route("/foo", get(|| async { "Hi from /foo" }))
        .nest_service("/assets", serve_dir.clone())
        .fallback_service(serve_dir)
        // `GET /` goes to `root`
        //        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/api/packages", get(get_packages))
        .route("/api/packages/:package_name/:package_version", get(get_package))
        .layer(middleware::from_fn(error_logging_middleware))
        .layer(TraceLayer::new_for_http()
            .on_response(|response: &Response<_>, _latency: Duration, _span: &Span| {
                if !response.status().is_success() {
                    warn!("HTTP Error {} for {:?}", response.status(), response)
                }
            })
            .on_failure(|_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                tracing::error!("something went wrong: {:?}", _error);
            }))
        // end
        ;

    // run our app with hyper, listening globally on port 3000
    let listen_address = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(listen_address).await.unwrap();
    info!("Listening on {listen_address}");
    axum::serve(listener, app).await.unwrap();
}

async fn error_logging_middleware(req: Request, next: Next) -> Response {
    let response = next.run(req).await;
    if !response.status().is_success() {
        warn!("HTTP Error {} for {:?}", response.status(), response);
        let (mut parts, body) = response.into_parts();
        let collect = Limited::new(body, 10000).collect().await.unwrap();
        let bytes = collect.to_bytes();
        warn!("Collected {:?}", bytes);
        let bytes1 = "f00".as_bytes();
        let len = bytes1.len();
        let new_body = Body::from(bytes1);
        parts.headers.remove(http::header::CONTENT_LENGTH);
        parts
            .headers
            .insert(http::header::CONTENT_LENGTH, HeaderValue::from(len));
        return Response::from_parts(parts, new_body);
    }
    response
}

async fn get_package(
    Path((package_name, package_version)): Path<(String, String)>,
) -> Result<Json<PackageDescription>, ErrorResponse> {
    return Err(ErrorResponse::from(
        Response::builder()
            .status(500)
            .body(Body::from("oh no!"))
            .unwrap(),
    ));
    /*
    Json(PackageDescription {
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
    })*/
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
