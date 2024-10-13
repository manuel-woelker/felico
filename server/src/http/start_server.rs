use crate::infra::error::{test_error, ServerError};
use crate::middleware::logging_middleware::error_logging_middleware;
use crate::model::bundle::{FunctionDescription, PackageDescription, PackageIndex, PackageInfo};
use axum::extract::Path;
use axum::routing::get;
use axum::{middleware, Json, Router};
use http::StatusCode;
use log::info;
use tower_http::services::{ServeDir, ServeFile};

pub async fn start_server() {
    // initialize tracing
    tracing_subscriber::fmt::init();
    info!("Starting up;");
    let serve_index_html = ServeFile::new("./web-ui/dist/index.html");
    let serve_dir =
        ServeDir::new("./web-ui/dist/assets").not_found_service(serve_index_html.clone());

    // build our application with a route
    let app = Router::new()
        .nest_service("/", serve_index_html)
        .route("/foo", get(|| async { "Hi from /foo" }))
        .nest_service("/assets", serve_dir.clone())
        .fallback_service(serve_dir)
        .route("/api/packages", get(get_packages))
        .route("/api/test_error", get(test_error))
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
        packages: vec![
            PackageInfo {
                name: "std".to_string(),
                version: "0.0.2".to_string(),
            },
            PackageInfo {
                name: "test".to_string(),
                version: "0.0.1".to_string(),
            },
        ],
    };
    (StatusCode::CREATED, Json(package_index))
}
