//! A runnable server exercising every extractor in this crate.
//!
//! ```bash
//! cargo run --example server
//! ```
//!
//! Then, in another shell:
//!
//! ```bash
//! curl -i localhost:3000/users -H 'content-type: application/json' \
//!      -d '{"username":"ferris","email":"ferris@rust-lang.org","age":30}'
//! curl -i localhost:3000/users -H 'content-type: application/json' \
//!      -d '{"username":"x","email":"nope","age":5}'
//! curl -i 'localhost:3000/search?query=rust&page=2'
//! ```

use axum::{
    Router,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_validated_extractors::{
    Valid, ValidatedForm, ValidatedJson, ValidatedQuery, ValidationError,
};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3, max = 20))]
    username: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 130))]
    age: u32,
}

#[derive(Debug, Deserialize, Validate)]
struct Login {
    #[validate(length(min = 3))]
    username: String,
    #[validate(length(min = 8))]
    password: String,
}

#[derive(Debug, Deserialize, Validate)]
struct Search {
    #[validate(length(min = 2))]
    query: String,
    #[validate(range(min = 1, max = 100))]
    page: Option<u32>,
}

/// JSON body. `user` cannot exist unless it validated.
async fn create_user(ValidatedJson(user): ValidatedJson<CreateUser>) -> String {
    format!(
        "created {} <{}>, age {}",
        user.username, user.email, user.age
    )
}

/// URL-encoded body.
async fn login(ValidatedForm(form): ValidatedForm<Login>) -> String {
    format!("logged in {}", form.username)
}

/// Query string.
async fn search(ValidatedQuery(params): ValidatedQuery<Search>) -> String {
    format!(
        "searching {} (page {})",
        params.query,
        params.page.unwrap_or(1)
    )
}

/// Taking the extractor as a `Result` hands you the error instead of letting the
/// crate's `IntoResponse` build the response.
///
/// Every arm returns a `Response`: a bare `String` would compile but respond `200 OK`,
/// reporting the failures as successes.
async fn create_user_custom_errors(
    user: Result<ValidatedJson<CreateUser>, ValidationError>,
) -> Response {
    match user {
        Ok(ValidatedJson(user)) => {
            (StatusCode::CREATED, format!("ok {}", user.username)).into_response()
        }
        Err(ValidationError::ValidationError(errors)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("{} invalid field(s)", errors.field_errors().len()),
        )
            .into_response(),
        // Extraction failed, so keep the status Axum chose (415, 422, 413, ...) rather
        // than flattening it to 400.
        Err(other) => other.into_response(),
    }
}

#[derive(Debug, Deserialize, Validate)]
struct UserPath {
    #[validate(range(min = 1))]
    id: u64,
}

/// `Valid<E>` composes over any extractor, so a path param can be validated too — which
/// the fixed `ValidatedJson`/`ValidatedForm`/`ValidatedQuery` wrappers cannot do.
async fn get_user(
    Valid(Path(user)): Valid<Path<UserPath>>,
    Valid(Query(page)): Valid<Query<Search>>,
) -> String {
    format!("user {} matching {}", user.id, page.query)
}

pub fn app() -> Router {
    Router::new()
        .route("/users", post(create_user))
        .route("/users/custom", post(create_user_custom_errors))
        .route("/users/{id}", get(get_user))
        .route("/login", post(login))
        .route("/search", get(search))
}

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app()).await.unwrap();
}
