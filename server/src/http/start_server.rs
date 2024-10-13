use crate::infra::error::{test_error, ServerError};
use crate::middleware::logging_middleware::error_logging_middleware;
use crate::model::bundle::{BundleDescription, BundleIndex, BundleInfo, FunctionDescription};
use crate::model::model_facade::ModelFacade;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{middleware, Json, Router};
use felico_compiler::frontend::compile::compile_module;
use felico_compiler::infra::result::FelicoResult;
use felico_compiler::infra::source_file::SourceFile;
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
    let module = compile_module(SourceFile::from_path("bundles/test.felico")?)?;
    let model_facade = ModelFacade::new(vec![module]);
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
    Path((bundle_name, bundle_version)): Path<(String, String)>,
    State(model_facade): State<ModelFacade>,
) -> Result<Json<BundleDescription>, ServerError> {
    //    Err("foo".into())
    let found = model_facade
        .modules()
        .iter()
        .find(|module| module.name == bundle_name);
    let Some(bundle) = found else {
        return Err("Bundle not found".into());
    };
    Ok(Json(BundleDescription {
        info: BundleInfo {
            name: bundle_name,
            version: bundle_version,
            /*
            functions: vec![FunctionDescription {
                name: "debug_print".to_string(),
                signature: "fun debug_print(item: any)".to_string(),
            }],*/
        },
        functions: bundle
            .module_entries
            .values()
            .map(|entry| FunctionDescription {
                name: entry.name.to_string(),
                signature: entry.ty.to_string(),
            })
            .collect(),
        /*        functions: vec![FunctionDescription {
            name: "debug_print".to_string(),
            signature: "fun debug_print(item: any)".to_string(),
        }],*/
    }))
}

async fn get_bundles(State(model_facade): State<ModelFacade>) -> (StatusCode, Json<BundleIndex>) {
    let bundle_index = BundleIndex {
        bundles: model_facade
            .modules()
            .iter()
            .map(|module| BundleInfo {
                name: module.name.to_string(),
                version: "0.0.1".to_string(),
            })
            .collect::<Vec<_>>(),
    };
    /*    let bundle_index = BundleIndex {
        bundles: vec![
            BundleInfo {
                name: "std".to_string(),
                version: "0.0.2".to_string(),
            },
            BundleInfo {
                name: "test".to_string(),
                version: "0.0.1".to_string(),
            },
        ],
    };*/

    (StatusCode::CREATED, Json(bundle_index))
}
