use axum::{
    Router,
    body::Body,
    http::StatusCode,
    routing::{get, post},
};
use axum_validated_extractors::*;
use serde::Deserialize;
use tower::ServiceExt;
use validator::Validate;

// Test input structures
#[derive(Debug, Deserialize, Validate, Clone)]
pub struct JsonInput {
    #[validate(length(min = 2))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct FormInput {
    #[validate(length(min = 2))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct QueryInput {
    #[validate(length(min = 2))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

// Test handlers
#[axum::debug_handler]
pub async fn json_handler(ValidatedJson(_input): ValidatedJson<JsonInput>) -> &'static str {
    "ok"
}

#[axum::debug_handler]
pub async fn form_handler(ValidatedForm(_input): ValidatedForm<FormInput>) -> &'static str {
    "ok"
}

#[axum::debug_handler]
pub async fn query_handler(ValidatedQuery(_input): ValidatedQuery<QueryInput>) -> &'static str {
    "ok"
}

// JSON tests
#[tokio::test]
async fn test_valid_json() {
    let app = Router::new().route("/", post(json_handler));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"test","email":"test@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_json() {
    let app = Router::new().route("/", post(json_handler));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"a","email":"invalid-email"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Form tests
#[tokio::test]
async fn test_valid_form() {
    let app = Router::new().route("/", post(form_handler));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .method("POST")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("name=test&email=test@example.com"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_form() {
    let app = Router::new().route("/", post(form_handler));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .method("POST")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("name=a&email=invalid-email"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Query tests
#[tokio::test]
async fn test_valid_query() {
    let app = Router::new().route("/", get(query_handler));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/?name=test&email=test@example.com")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_query() {
    let app = Router::new().route("/", get(query_handler));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/?name=a&email=invalid-email")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─────────────────────────────────────────────────────────────────────────────
// Response bodies and the status Axum's own rejections carry.
// ─────────────────────────────────────────────────────────────────────────────

use axum::http::Request;

async fn call(app: Router, req: Request<Body>) -> (StatusCode, String) {
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

fn json_req(uri: &'static str, body: &'static str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// The validation message is flattened onto one line and names every failing field.
#[tokio::test]
async fn validation_error_body_is_single_line() {
    let app = Router::new().route("/", post(json_handler));
    let (status, body) = call(app, json_req("/", r#"{"name":"a","email":"nope"}"#)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.starts_with("Input validation error: ["), "{body}");
    assert!(body.contains("name") && body.contains("email"), "{body}");
    assert!(!body.contains('\n'), "{body}");
}

/// Deserialization never reached validation, so each rejection keeps the status Axum
/// chose for it rather than being flattened to 400.
#[tokio::test]
async fn deserialization_failures_keep_the_axum_rejection_status() {
    let no_content_type = Request::builder()
        .uri("/")
        .method("POST")
        .body(Body::from(r#"{"name":"test","email":"test@example.com"}"#))
        .unwrap();

    let cases = [
        (json_req("/", "{not json"), StatusCode::BAD_REQUEST),
        // Syntactically valid JSON, wrong shape.
        (
            json_req("/", r#"{"name":"test"}"#),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (no_content_type, StatusCode::UNSUPPORTED_MEDIA_TYPE),
    ];

    for (req, expected) in cases {
        let app = Router::new().route("/", post(json_handler));
        let (status, body) = call(app, req).await;
        assert_eq!(status, expected, "{body}");
        // A rejection is not a validation error, so it must not be dressed up as one.
        assert!(!body.contains("Input validation error"), "{body}");
        assert!(!body.is_empty());
    }
}

/// `ValidatedQuery` has only a `FromRequestParts` impl; Axum's blanket impl must cover
/// the `FromRequest` position. This fails to compile if that is not true.
#[axum::debug_handler]
async fn query_then_body_handler(
    ValidatedQuery(_): ValidatedQuery<QueryInput>,
    ValidatedJson(_): ValidatedJson<JsonInput>,
) -> &'static str {
    "ok"
}

#[tokio::test]
async fn query_composes_ahead_of_a_body_extractor() {
    let app = Router::new().route("/", post(query_then_body_handler));
    let req = json_req(
        "/?name=test&email=test@example.com",
        r#"{"name":"test","email":"test@example.com"}"#,
    );

    let (status, body) = call(app, req).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
