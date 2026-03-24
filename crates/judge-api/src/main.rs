use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::services::ServeFile;
use tower_http::trace::TraceLayer;

#[derive(Serialize)]
struct HelloResponse {
    message: &'static str,
    version: &'static str,
}

async fn hello() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello from the Judge API",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "judge_api=debug,tower_http=debug".into()),
        )
        .init();

    let cards_path: Option<PathBuf> = std::env::var("CARDS_FILE").ok().map(PathBuf::from);

    let mut app = Router::new()
        .route("/", get(hello))
        .route("/health", get(|| async { "ok" }));

    match &cards_path {
        Some(path) if path.exists() => {
            tracing::info!("serving cards from {}", path.display());
            app = app.route_service("/cards", ServeFile::new(path));
        }
        Some(path) => {
            tracing::warn!("CARDS_FILE set but not found: {}", path.display());
            app = app.route("/cards", get(|| async {
                (axum::http::StatusCode::SERVICE_UNAVAILABLE, "cards file not found")
            }));
        }
        None => {
            tracing::info!("CARDS_FILE not set — /cards endpoint disabled");
            app = app.route("/cards", get(|| async {
                (axum::http::StatusCode::NOT_FOUND, "cards file not configured")
            }));
        }
    }

    app = app.layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
