use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

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
        .route("/api/modules", get(get_modules))
        .route("/users", post(create_user));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn get_modules() -> (StatusCode, Json<ModuleIndex>) {
    let module_index = ModuleIndex {
        modules: vec!["foo".to_string(), "bar".to_string()],
    };
    (StatusCode::CREATED, Json(module_index))
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

#[derive(Serialize)]
struct ModuleIndex {
    modules: Vec<String>,
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
