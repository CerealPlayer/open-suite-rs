use open_suite_rs::{config::Conns, get_router, storage::get_bucket};
use s3::Region;
use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let region = Region::Custom {
        region: env::var("S3_REGION").expect("S3_REGION env var is required"),
        endpoint: env::var("S3_ENDPOINT").expect("S3_ENDPOINT env var is required"),
    };
    let bucket = get_bucket("test", region).await.unwrap();
    let app = get_router().with_state(Conns { bucket });
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind TCP listener on 0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("axum server failed");
}
