use std::sync::LazyLock;
use std::time::Duration;

use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

/// The compiler that built the binary, recorded by build.rs.
const RUST_VERSION: &str = env!("RUSTC_VERSION");

/// Pages and assets are compiled into the binary, so the runtime image needs
/// only the application binary.
static INDEX: LazyLock<String> =
    LazyLock::new(|| include_str!("../web/index.html").replace("{{RUST_VERSION}}", RUST_VERSION));
const STYLES: &str = include_str!("../web/assets/styles.css");

/// How long a request may take before the server answers 408.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/assets/styles.css", get(styles))
        .route("/api/status", get(status))
        .route("/healthz", get(health))
        .fallback(not_found)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        // Log every request with its status and latency at the default level.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}

async fn index() -> Html<&'static str> {
    Html(INDEX.as_str())
}

async fn styles() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], STYLES)
}

#[derive(Serialize)]
struct Status {
    status: &'static str,
    runtime: &'static str,
    server: &'static str,
}

async fn status() -> Json<Status> {
    Json(Status {
        status: "ok",
        runtime: RUST_VERSION,
        server: "axum",
    })
}

/// The deployment health check. It avoids optional services, so the
/// application is healthy before any database or cache is linked.
async fn health() -> &'static str {
    "ok\n"
}

async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 page not found\n")
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::router;

    async fn send(method: Method, path: &str) -> (StatusCode, Option<String>, String) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let response = router().oneshot(request).await.unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|value| value.to_str().unwrap().to_owned());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            content_type,
            String::from_utf8(body.to_vec()).unwrap(),
        )
    }

    #[tokio::test]
    async fn serves_the_home_page() {
        let (status, content_type, body) = send(Method::GET, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/html; charset=utf-8"));
        assert!(body.contains("Your Rust app is running"));
        assert!(
            body.contains("rustc "),
            "the page shows the compiler version"
        );
    }

    #[tokio::test]
    async fn serves_embedded_assets() {
        let (status, content_type, body) = send(Method::GET, "/assets/styles.css").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/css; charset=utf-8"));
        assert!(body.contains(":root"));
    }

    #[tokio::test]
    async fn answers_health_checks() {
        let (status, content_type, body) = send(Method::GET, "/healthz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("text/plain; charset=utf-8"));
        assert_eq!(body, "ok\n");
    }

    #[tokio::test]
    async fn reports_status_as_json() {
        let (status, content_type, body) = send(Method::GET, "/api/status").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(content_type.as_deref(), Some("application/json"));
        let payload: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["server"], "axum");
    }

    #[tokio::test]
    async fn answers_unknown_paths_with_not_found() {
        let (status, _, body) = send(Method::GET, "/missing").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, "404 page not found\n");
    }

    #[tokio::test]
    async fn rejects_other_methods() {
        let (status, _, _) = send(Method::POST, "/").await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }
}
