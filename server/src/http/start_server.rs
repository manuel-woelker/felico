use crate::infra::error::{test_error, ServerError};
use crate::middleware::logging_middleware::error_logging_middleware;
use crate::model::bundle::{BundleDescription, BundleIndex, BundleInfo};
use crate::model::model_facade::ModelFacade;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{middleware, Json, Router};
use felico_compiler::frontend::compile::compile_module;
use felico_compiler::frontend::resolve::module_manifest::BundleManifest;
use felico_compiler::infra::arena::Arena;
use felico_compiler::infra::result::FelicoResult;
use felico_compiler::model::workspace::Workspace;
use http::StatusCode;
use log::{info, warn};
use tower_http::services::{ServeDir, ServeFile};

pub async fn start_server() {
    if let Err(report) = start_server_inner().await {
        warn!("Error: {:?}", report)
    };
}

pub async fn start_server_inner() -> FelicoResult<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();
    info!("Starting up;");
    let arena = Arena::new();
    let workspace = Workspace::new(&arena);
    let module = compile_module(
        workspace.source_file_from_path("bundles/test.felico")?,
        workspace,
    )?;
    arena.log_memory_usage();
    let bundle = BundleManifest {
        name: module.name,
        modules: vec![module],
    };
    let model_facade = ModelFacade::new(vec![bundle]);
    let serve_index_html = ServeFile::new("./web-ui/dist/index.html");
    let serve_dir =
        ServeDir::new("./web-ui/dist/assets").not_found_service(serve_index_html.clone());

    // build our application with a route
    let app = Router::new()
        .nest_service("/", serve_index_html)
        .route("/foo", get(|| async { "Hi from /foo" }))
        .nest_service("/assets", serve_dir.clone())
        .fallback_service(serve_dir)
        .route("/api/bundles", get(get_bundles))
        .route("/api/bundles/:bundle_name/:bundle_version", get(get_bundle))
        .route("/api/test_error", get(test_error))
        .layer(middleware::from_fn(error_logging_middleware))
        .with_state(model_facade)
        // end
        ;

    // run our app with hyper, listening globally on port 3000
    let listen_address = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(listen_address).await.unwrap();
    info!("Listening on {listen_address}");
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

async fn get_bundle(
    Path((bundle_name, _bundle_version)): Path<(String, String)>,
    State(model_facade): State<ModelFacade>,
) -> Result<Json<BundleDescription>, ServerError> {
    let found = model_facade
        .bundles()
        .iter()
        .find(|bundle| bundle.info.name == *bundle_name);
    let Some(bundle) = found else {
        return Err("Bundle not found".into());
    };
    Ok(Json(bundle.clone()))
}

async fn get_bundles(State(model_facade): State<ModelFacade>) -> (StatusCode, Json<BundleIndex>) {
    let bundle_index = BundleIndex {
        bundles: model_facade
            .bundles()
            .iter()
            .map(|module| BundleInfo {
                name: module.info.name.to_string(),
                version: module.info.version.to_string(),
            })
            .collect::<Vec<_>>(),
    };
    (StatusCode::CREATED, Json(bundle_index))
}
