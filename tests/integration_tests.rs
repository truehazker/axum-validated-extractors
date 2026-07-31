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
// Behaviour the original suite did not cover: response bodies, the status Axum's
// own rejections carry, and the `Valid` wrapper.
// ─────────────────────────────────────────────────────────────────────────────

use axum::extract::{Form, Json, Path, Query};
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

fn get_req(uri: &'static str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("GET")
        .body(Body::empty())
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

// ─── Valid<E> ───

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct IdPath {
    #[validate(range(min = 1))]
    pub id: u64,
}

#[axum::debug_handler]
async fn valid_json(Valid(Json(input)): Valid<Json<JsonInput>>) -> String {
    input.name
}

#[axum::debug_handler]
async fn valid_form(Valid(Form(input)): Valid<Form<FormInput>>) -> String {
    input.name
}

#[axum::debug_handler]
async fn valid_path_and_query(
    Valid(Path(path)): Valid<Path<IdPath>>,
    Valid(Query(q)): Valid<Query<QueryInput>>,
) -> String {
    format!("{} {}", path.id, q.name)
}

fn valid_app() -> Router {
    Router::new()
        .route("/json", post(valid_json))
        .route("/form", post(valid_form))
        .route("/{id}", get(valid_path_and_query))
}

#[tokio::test]
async fn valid_wraps_body_extractors() {
    let ok = json_req("/json", r#"{"name":"test","email":"test@example.com"}"#);
    let (status, body) = call(valid_app(), ok).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, "test");

    let bad = json_req("/json", r#"{"name":"a","email":"nope"}"#);
    let (status, body) = call(valid_app(), bad).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Input validation error"), "{body}");

    // The inner extractor's own status survives the wrapper.
    let wrong_shape = json_req("/json", r#"{"name":"test"}"#);
    let (status, _) = call(valid_app(), wrong_shape).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let form = Request::builder()
        .uri("/form")
        .method("POST")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("name=test&email=test@example.com"))
        .unwrap();
    let (status, body) = call(valid_app(), form).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Path params are reachable only through `Valid` — the fixed wrappers cannot do this.
#[tokio::test]
async fn valid_validates_path_params() {
    let (status, body) = call(valid_app(), get_req("/7?name=test&email=test@example.com")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, "7 test");

    // id = 0 violates range(min = 1).
    let (status, body) = call(valid_app(), get_req("/0?name=test&email=test@example.com")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Input validation error"), "{body}");
    assert!(body.contains("id"), "{body}");

    // A non-validation path failure keeps Axum's rejection instead.
    let (status, body) = call(
        valid_app(),
        get_req("/abc?name=test&email=test@example.com"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!body.contains("Input validation error"), "{body}");
}

/// `HasValidate` is the documented extension point, so an extractor defined outside this
/// crate must be able to opt in.
#[tokio::test]
async fn valid_works_with_a_user_defined_extractor() {
    struct Wrapper<T>(T);

    impl<T: Validate> HasValidate for Wrapper<T> {
        type Data = T;
        fn get_validate(&self) -> &T {
            &self.0
        }
    }

    impl<T, S> axum::extract::FromRequest<S> for Wrapper<T>
    where
        Json<T>: axum::extract::FromRequest<S>,
        S: Send + Sync,
    {
        type Rejection = <Json<T> as axum::extract::FromRequest<S>>::Rejection;

        async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
            let Json(value) = Json::<T>::from_request(req, state).await?;
            Ok(Wrapper(value))
        }
    }

    async fn handler(Valid(Wrapper(input)): Valid<Wrapper<JsonInput>>) -> String {
        input.name
    }

    let app = Router::new().route("/", post(handler));
    let ok = json_req("/", r#"{"name":"test","email":"test@example.com"}"#);
    let (status, body) = call(app.clone(), ok).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, "test");

    let bad = json_req("/", r#"{"name":"a","email":"nope"}"#);
    let (status, body) = call(app, bad).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("Input validation error"), "{body}");
}

/// Mirrors the `Result<Extractor, ValidationError>` pattern shown in the crate docs,
/// the README and `examples/server.rs`. Each arm must yield the right status: returning
/// a bare `String` from every arm compiles but responds `200 OK`, silently turning
/// failures into successes.
#[axum::debug_handler]
async fn custom_error_handler(
    input: Result<ValidatedJson<JsonInput>, ValidationError>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match input {
        Ok(ValidatedJson(input)) => (StatusCode::CREATED, input.name).into_response(),
        Err(ValidationError::ValidationError(errors)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("{} invalid field(s)", errors.field_errors().len()),
        )
            .into_response(),
        // Extraction failed, so keep the status Axum chose for it.
        Err(other) => other.into_response(),
    }
}

#[tokio::test]
async fn documented_result_pattern_keeps_each_status() {
    let no_content_type = Request::builder()
        .uri("/")
        .method("POST")
        .body(Body::from(r#"{"name":"test","email":"test@example.com"}"#))
        .unwrap();

    let cases = [
        (
            json_req("/", r#"{"name":"test","email":"test@example.com"}"#),
            StatusCode::CREATED,
        ),
        (
            json_req("/", r#"{"name":"a","email":"nope"}"#),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        // Deserialization failures keep the rejection's own status, not a blanket 400.
        (no_content_type, StatusCode::UNSUPPORTED_MEDIA_TYPE),
        (json_req("/", "{not json"), StatusCode::BAD_REQUEST),
    ];

    for (req, expected) in cases {
        let app = Router::new().route("/", post(custom_error_handler));
        let (status, body) = call(app, req).await;
        assert_eq!(status, expected, "{body}");
        // Never report a failure as a success.
        assert!(status != StatusCode::OK, "{body}");
    }
}
