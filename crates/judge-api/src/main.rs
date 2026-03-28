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
        version: std::env::var("RIFTBOUND_CARDS_VERSION").unwrap_or_else(|_| "unknown".to_string()),
    })
}

pub fn build_app(cards_path: Option<PathBuf>, riftbound_cards_path: Option<PathBuf>) -> Router {
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
            app = app.route(
                "/cards",
                get(|| async {
                    (
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        "cards file not found",
                    )
                }),
            );
        }
        None => {
            tracing::info!("CARDS_FILE not set — /cards endpoint disabled");
            app = app.route(
                "/cards",
                get(|| async {
                    (
                        axum::http::StatusCode::NOT_FOUND,
                        "cards file not configured",
                    )
                }),
            );
        }
    }

    match &riftbound_cards_path {
        Some(path) if path.exists() => {
            tracing::info!("serving riftbound cards from {}", path.display());
            app = app.route_service("/riftbound/cards", ServeFile::new(path));
        }
        Some(path) => {
            tracing::warn!("RIFTBOUND_CARDS_FILE set but not found: {}", path.display());
            app = app.route(
                "/riftbound/cards",
                get(|| async {
                    (
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        "riftbound cards file not found",
                    )
                }),
            );
        }
        None => {
            tracing::info!("RIFTBOUND_CARDS_FILE not set — /riftbound/cards endpoint disabled");
            app = app.route(
                "/riftbound/cards",
                get(|| async {
                    (
                        axum::http::StatusCode::NOT_FOUND,
                        "riftbound cards file not configured",
                    )
                }),
            );
        }
    }

    app
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
    let riftbound_cards_path: Option<PathBuf> = std::env::var("RIFTBOUND_CARDS_FILE")
        .ok()
        .map(PathBuf::from);

    let app = build_app(cards_path, riftbound_cards_path).layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_string(body: Body) -> String {
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn app() -> Router {
        build_app(None, None)
    }

    #[tokio::test]
    async fn test_health_returns_ok() {
        let response = app()
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response.into_body()).await;
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn test_hello_returns_json() {
        // if this doesnt work something is probably wrong
        let response = app()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["message"], "Hello from the Judge API");
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_version_returns_json() {
        let response = app()
            .oneshot(Request::get("/version").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_riftbound_version_returns_json() {
        let response = app()
            .oneshot(
                Request::get("/riftbound/version")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_cards_not_configured_returns_404() {
        let response = app()
            .oneshot(Request::get("/cards").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_riftbound_cards_not_configured_returns_404() {
        let response = app()
            .oneshot(
                Request::get("/riftbound/cards")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_cards_missing_file_returns_503() {
        let app = build_app(Some(PathBuf::from("/nonexistent/cards.json")), None);
        let response = app
            .oneshot(Request::get("/cards").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_riftbound_cards_missing_file_returns_503() {
        let app = build_app(None, Some(PathBuf::from("/nonexistent/riftbound.json")));
        let response = app
            .oneshot(
                Request::get("/riftbound/cards")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_cards_existing_file_returns_200() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"[]").unwrap();
        let app = build_app(Some(tmp.path().to_path_buf()), None);
        let response = app
            .oneshot(Request::get("/cards").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_env_var_reflected() {
        std::env::set_var("CARDS_VERSION", "test-1.2.3");
        let response = app()
            .oneshot(Request::get("/version").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = body_string(response.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["version"], "test-1.2.3");
        std::env::remove_var("CARDS_VERSION");
    }
}
