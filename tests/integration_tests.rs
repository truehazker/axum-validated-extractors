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
