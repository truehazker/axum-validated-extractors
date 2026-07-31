//! A collection of validated extractors for Axum
//!
//! This crate provides a set of extractors for Axum that automatically validate
//! the extracted data using the `validator` crate.
//!
//! # Features
//!
//! - `ValidatedForm`: Validates URL-encoded form data
//! - `ValidatedJson`: Validates JSON data
//! - `ValidatedQuery`: Validates query parameters
//! - Automatic validation using the `validator` crate
//! - Type-safe error handling
//!
//! # Examples
//!
//! ```rust
//! use axum::{routing::post, Router};
//! use axum_validated_extractors::ValidatedJson;
//! use serde::Deserialize;
//! use validator::Validate;
//!
//! #[derive(Debug, Deserialize, Validate)]
//! struct CreateUser {
//!     #[validate(length(min = 3))]
//!     username: String,
//!     #[validate(email)]
//!     email: String,
//! }
//!
//! async fn create_user(ValidatedJson(user): ValidatedJson<CreateUser>) {
//!     // user is guaranteed to be valid
//!     println!("Creating user: {user:?}");
//! }
//!
//! let app: Router<()> = Router::new()
//!     .route("/users", post(create_user));
//! ```
//!
//! `ValidatedForm` and `ValidatedQuery` are used the same way.
//!
//! # Error Handling
//!
//! A validation failure is rejected as `400 Bad Request`. A deserialization failure keeps
//! the status of the underlying Axum rejection (`415` on a missing content type, `422` on
//! an unexpected shape, and so on). Take the extractor as a `Result` to build the response
//! yourself instead:
//!
//! ```rust
//! use axum::response::IntoResponse;
//! use axum_validated_extractors::{ValidatedJson, ValidationError};
//! # use serde::Deserialize;
//! # use validator::Validate;
//! # #[derive(Debug, Deserialize, Validate)]
//! # struct CreateUser {
//! #     #[validate(length(min = 3))]
//! #     username: String,
//! # }
//!
//! async fn create_user(
//!     user: Result<ValidatedJson<CreateUser>, ValidationError>,
//! ) -> impl IntoResponse {
//!     match user {
//!         Ok(ValidatedJson(user)) => format!("created {}", user.username),
//!         Err(ValidationError::ValidationError(errors)) => format!("invalid: {errors}"),
//!         Err(other) => format!("bad request: {other}"),
//!     }
//! }
//! ```

use axum::{
    extract::{rejection::FormRejection, rejection::JsonRejection, rejection::QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use thiserror::Error;
use validator::Validate;

/// Error type for validated extractors
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Error variant for validation errors
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),

    /// Error variant for form rejection errors
    #[error(transparent)]
    FormRejection(#[from] FormRejection),

    /// Error variant for JSON rejection errors
    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),

    /// Error variant for query rejection errors
    #[error(transparent)]
    QueryRejection(#[from] QueryRejection),
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        match self {
            ValidationError::ValidationError(_) => {
                let message = format!("Input validation error: [{self}]").replace('\n', ", ");
                (StatusCode::BAD_REQUEST, message).into_response()
            }
            // Axum's rejections already carry the right status (415 on a missing content
            // type, 413 on an oversized body, ...) — do not flatten them all to 400.
            ValidationError::FormRejection(rejection) => rejection.into_response(),
            ValidationError::JsonRejection(rejection) => rejection.into_response(),
            ValidationError::QueryRejection(rejection) => rejection.into_response(),
        }
    }
}

/// Validated form data extractor
///
/// Validates URL-encoded form data using the `validator` crate. See the [crate] docs.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedForm<T>(pub T);

/// Validated JSON data extractor
///
/// Validates JSON data using the `validator` crate. See the [crate] docs.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

/// Validated query parameters extractor
///
/// Validates query parameters using the `validator` crate. See the [crate] docs.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedQuery<T>(pub T);

/// Helper function to validate and wrap a value
fn validate_and_wrap<T: Validate>(value: T) -> Result<T, ValidationError> {
    value.validate()?;
    Ok(value)
}

impl<T, S> axum::extract::FromRequest<S> for ValidatedForm<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    axum::extract::Form<T>: axum::extract::FromRequest<S, Rejection = FormRejection>,
{
    type Rejection = ValidationError;

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::extract::Form(value) = axum::extract::Form::<T>::from_request(req, state).await?;
        Ok(ValidatedForm(validate_and_wrap(value)?))
    }
}

impl<T, S> axum::extract::FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    axum::extract::Json<T>: axum::extract::FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ValidationError;

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::extract::Json(value) = axum::extract::Json::<T>::from_request(req, state).await?;
        Ok(ValidatedJson(validate_and_wrap(value)?))
    }
}

// Only `FromRequestParts` — Axum's blanket impl derives `FromRequest` from it, so a
// hand-written one would be dead code.
impl<T, S> axum::extract::FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    axum::extract::Query<T>: axum::extract::FromRequestParts<S, Rejection = QueryRejection>,
{
    type Rejection = ValidationError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let axum::extract::Query(value) =
            axum::extract::Query::<T>::from_request_parts(parts, state).await?;
        Ok(ValidatedQuery(validate_and_wrap(value)?))
    }
}
