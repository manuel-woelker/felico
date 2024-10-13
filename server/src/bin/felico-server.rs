use felico_server::http::start_server::start_server;

#[tokio::main]
async fn main() {
    start_server().await;
}
