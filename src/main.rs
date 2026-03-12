use open_suite_rs::{config::Conns, get_router, storage::get_bucket};
use s3::Region;
use sea_orm::{ConnectOptions, Database};
use std::env;
use std::future::pending;
use tokio::net::TcpListener;
use tokio::signal;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let region = Region::Custom {
        region: env::var("S3_REGION").expect("S3_REGION env var is required"),
        endpoint: env::var("S3_ENDPOINT").expect("S3_ENDPOINT env var is required"),
    };

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL env var is required");
    let db_options = ConnectOptions::new(db_url);
    let db = Database::connect(db_options)
        .await
        .expect("failed to connect to postgres");

    let bucket = get_bucket("test", region).await.unwrap();
    let app = get_router().with_state(Conns { bucket, db });
    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind TCP listener on 0.0.0.0:3000");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("axum server failed");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("shutdown signal received, terminating gracefully");
}
