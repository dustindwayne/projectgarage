#[tokio::main]
async fn main() {
    garage_server::run_server().await;
}
