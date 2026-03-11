use open_suite_rs::get_router;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = get_router();
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind TCP listener on 0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("axum server failed");
}
