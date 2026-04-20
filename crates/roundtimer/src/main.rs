use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TimerData {
    time_length: u64,
    running: bool,
    #[serde(default = "default_neg_one")]
    time_started: i64,
    #[serde(default = "default_neg_one")]
    time_remaining_when_started: i64,
}

fn default_neg_one() -> i64 {
    -1
}

#[derive(Serialize)]
struct CreateTimerResponse {
    id: String,
    time_length: u64,
    running: bool,
    time_started: i64,
    time_remaining_when_started: i64,
}

#[derive(Debug, Deserialize)]
struct StartTimerRequest {
    id: String,
    time_started: i64,
    #[serde(default = "default_neg_one")]
    time_remaining_when_started: i64,
}

#[derive(Serialize)]
struct StartTimerResponse {
    id: String,
    time_started: i64,
    time_remaining_when_started: i64,
    running: bool,
}

#[derive(Debug, Deserialize)]
struct StopTimerRequest {
    id: String,
    time_remaining_when_started: i64,
}

#[derive(Serialize)]
struct StopTimerResponse {
    id: String,
    time_remaining_when_started: i64,
    running: bool,
}

#[derive(Serialize)]
struct GetTimerResponse {
    id: String,
    running: bool,
    time_started: i64,
    time_length: u64,
    time_remaining_when_started: i64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn timers_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join("timers")
}

fn generate_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

fn timer_path(id: &str) -> std::path::PathBuf {
    timers_dir().join(format!("{}.txt", id))
}

fn save_timer(id: &str, data: &TimerData) -> Result<(), String> {
    let dir = timers_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create timers dir: {e}"))?;
    let contents = format!(
        "time_length={}\nrunning={}\ntime_started={}\ntime_remaining_when_started={}\n",
        data.time_length, data.running, data.time_started, data.time_remaining_when_started
    );
    fs::write(timer_path(id), contents).map_err(|e| format!("failed to write timer: {e}"))
}

fn load_timer(id: &str) -> Option<TimerData> {
    let contents = fs::read_to_string(timer_path(id)).ok()?;
    let mut time_length: Option<u64> = None;
    let mut running: Option<bool> = None;
    let mut time_started: i64 = -1;
    let mut time_remaining_when_started: i64 = -1;

    for line in contents.lines() {
        if let Some(val) = line.strip_prefix("time_length=") {
            time_length = val.parse().ok();
        } else if let Some(val) = line.strip_prefix("running=") {
            running = val.parse().ok();
        } else if let Some(val) = line.strip_prefix("time_started=") {
            time_started = val.parse().unwrap_or(-1);
        } else if let Some(val) = line.strip_prefix("time_remaining_when_started=") {
            time_remaining_when_started = val.parse().unwrap_or(-1);
        }
    }

    Some(TimerData {
        time_length: time_length?,
        running: running?,
        time_started,
        time_remaining_when_started,
    })
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_timer(
    Json(mut payload): Json<TimerData>,
) -> Result<Json<CreateTimerResponse>, (StatusCode, String)> {
    // Normalize unset optional fields to -1
    if payload.time_started == 0 {
        payload.time_started = -1;
    }
    if payload.time_remaining_when_started == 0 {
        payload.time_remaining_when_started = -1;
    }

    let id = generate_id();
    save_timer(&id, &payload).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(CreateTimerResponse {
        id,
        time_length: payload.time_length,
        running: payload.running,
        time_started: payload.time_started,
        time_remaining_when_started: payload.time_remaining_when_started,
    }))
}

async fn start_timer(
    Json(payload): Json<StartTimerRequest>,
) -> Result<Json<StartTimerResponse>, (StatusCode, String)> {
    let mut data = load_timer(&payload.id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("timer {} not found", payload.id)))?;

    data.running = true;
    data.time_started = payload.time_started;
    data.time_remaining_when_started = payload.time_remaining_when_started;

    save_timer(&payload.id, &data).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(StartTimerResponse {
        id: payload.id,
        time_started: data.time_started,
        time_remaining_when_started: data.time_remaining_when_started,
        running: true,
    }))
}

async fn stop_timer(
    Json(payload): Json<StopTimerRequest>,
) -> Result<Json<StopTimerResponse>, (StatusCode, String)> {
    let mut data = load_timer(&payload.id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("timer {} not found", payload.id)))?;

    data.running = false;
    data.time_started = -1;
    data.time_remaining_when_started = payload.time_remaining_when_started;

    save_timer(&payload.id, &data).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(StopTimerResponse {
        id: payload.id,
        time_remaining_when_started: data.time_remaining_when_started,
        running: false,
    }))
}

async fn get_timer(
    Path(id): Path<String>,
) -> Result<Json<GetTimerResponse>, (StatusCode, String)> {
    let data = load_timer(&id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("timer {id} not found")))?;

    Ok(Json(GetTimerResponse {
        id,
        running: data.running,
        time_started: data.time_started,
        time_length: data.time_length,
        time_remaining_when_started: data.time_remaining_when_started,
    }))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn build_app() -> Router {
    Router::new()
        .route("/timer", put(create_timer))
        .route("/timer/start", put(start_timer))
        .route("/timer/stop", put(stop_timer))
        .route("/timer/{id}", get(get_timer))
        .layer(CorsLayer::permissive())
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "roundtimer=debug,tower_http=debug".into()),
        )
        .init();

    let app = build_app().layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("roundtimer listening on {addr}");
    tracing::info!("timers stored in {}", timers_dir().display());

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(body: Body) -> serde_json::Value {
        let bytes = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_create_timer_returns_id_and_echo() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let body = serde_json::json!({
            "time_length": 300,
            "running": false
        });

        let response = build_app()
            .oneshot(
                Request::put("/timer")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response.into_body()).await;
        assert!(json["id"].is_string());
        assert_eq!(json["id"].as_str().unwrap().len(), 6);
        assert_eq!(json["time_length"], 300);
        assert_eq!(json["running"], false);
        assert_eq!(json["time_started"], -1);
        assert_eq!(json["time_remaining_when_started"], -1);
    }

    #[tokio::test]
    async fn test_create_timer_with_time_started() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let body = serde_json::json!({
            "time_length": 600,
            "running": true,
            "time_started": 1712345678
        });

        let response = build_app()
            .oneshot(
                Request::put("/timer")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response.into_body()).await;
        assert_eq!(json["time_started"], 1712345678i64);
    }

    #[tokio::test]
    async fn test_get_timer_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let response = build_app()
            .oneshot(
                Request::get("/timer/zzzzzz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_then_get_timer() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let create_body = serde_json::json!({
            "time_length": 120,
            "running": false
        });

        let create_resp = build_app()
            .oneshot(
                Request::put("/timer")
                    .header("content-type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let create_json = body_json(create_resp.into_body()).await;
        let id = create_json["id"].as_str().unwrap().to_string();

        let get_resp = build_app()
            .oneshot(
                Request::get(format!("/timer/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        let get_json = body_json(get_resp.into_body()).await;
        assert_eq!(get_json["id"], id);
        assert_eq!(get_json["time_length"], 120);
        assert_eq!(get_json["running"], false);
        assert_eq!(get_json["time_started"], -1);
    }

    #[tokio::test]
    async fn test_start_timer() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let create_body = serde_json::json!({
            "time_length": 90,
            "running": false
        });

        let create_resp = build_app()
            .oneshot(
                Request::put("/timer")
                    .header("content-type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let create_json = body_json(create_resp.into_body()).await;
        let id = create_json["id"].as_str().unwrap().to_string();

        let start_body = serde_json::json!({
            "id": id,
            "time_started": 9999999
        });

        let start_resp = build_app()
            .oneshot(
                Request::put("/timer/start")
                    .header("content-type", "application/json")
                    .body(Body::from(start_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(start_resp.status(), StatusCode::OK);
        let start_json = body_json(start_resp.into_body()).await;
        assert_eq!(start_json["running"], true);
        assert_eq!(start_json["time_started"], 9999999i64);
        assert_eq!(start_json["time_remaining_when_started"], -1);
    }

    #[tokio::test]
    async fn test_start_timer_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", tmp.path());

        let body = serde_json::json!({
            "id": "aaaaaa",
            "time_started": 1000
        });

        let response = build_app()
            .oneshot(
                Request::put("/timer/start")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
