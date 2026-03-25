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

#[derive(Serialize)]
struct VersionResponse {
    version: String,
}

async fn cards_version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: std::env::var("CARDS_VERSION").unwrap_or_else(|_| "unknown".to_string()),
    })
}

async fn riftbound_cards_version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: std::env::var("RIFTBOUND_CARDS_VERSION")
            .unwrap_or_else(|_| "unknown".to_string()),
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
    let riftbound_cards_path: Option<PathBuf> =
        std::env::var("RIFTBOUND_CARDS_FILE").ok().map(PathBuf::from);

    let mut app = Router::new()
        .route("/", get(hello))
        .route("/health", get(|| async { "ok" }))
        .route("/version", get(cards_version))
        .route("/riftbound/version", get(riftbound_cards_version));

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

    match &riftbound_cards_path {
        Some(path) if path.exists() => {
            tracing::info!("serving riftbound cards from {}", path.display());
            app = app.route_service("/riftbound/cards", ServeFile::new(path));
        }
        Some(path) => {
            tracing::warn!("RIFTBOUND_CARDS_FILE set but not found: {}", path.display());
            app = app.route("/riftbound/cards", get(|| async {
                (axum::http::StatusCode::SERVICE_UNAVAILABLE, "riftbound cards file not found")
            }));
        }
        None => {
            tracing::info!("RIFTBOUND_CARDS_FILE not set — /riftbound/cards endpoint disabled");
            app = app.route("/riftbound/cards", get(|| async {
                (axum::http::StatusCode::NOT_FOUND, "riftbound cards file not configured")
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
