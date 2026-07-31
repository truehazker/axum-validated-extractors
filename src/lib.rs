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
//! - `Valid`: Validates whatever extractor you wrap, including `Path`
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
//! `ValidatedForm` and `ValidatedQuery` are used the same way. [`Valid`] is generic over
//! the extractor it wraps, so it also covers [`axum::extract::Path`] and your own
//! extractors via [`HasValidate`].
//!
//! # Error Handling
//!
//! A validation failure is rejected as `400 Bad Request`. A deserialization failure keeps
//! the status of the underlying Axum rejection (`415` on a missing content type, `422` on
//! an unexpected shape, and so on). Take the extractor as a `Result` to build the response
//! yourself instead:
//!
//! ```rust
//! use axum::{http::StatusCode, response::{IntoResponse, Response}};
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
//! ) -> Response {
//!     match user {
//!         Ok(ValidatedJson(user)) => format!("created {}", user.username).into_response(),
//!         Err(ValidationError::ValidationError(errors)) => {
//!             (StatusCode::UNPROCESSABLE_ENTITY, format!("invalid: {errors}")).into_response()
//!         }
//!         // Anything else is an extraction failure; keep the status Axum chose for it.
//!         Err(other) => other.into_response(),
//!     }
//! }
//! ```
//!
//! Every arm must produce a [`Response`]. Returning a bare `String` from each arm compiles,
//! but `String` responds `200 OK`, so the failures would be reported as successes.

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

/// Bridges an extractor to the value inside it that [`Valid`] should validate.
///
/// Implemented for [`axum::extract::Json`], [`axum::extract::Form`],
/// [`axum::extract::Query`] and [`axum::extract::Path`]. Implement it for your own
/// extractor to make it work with [`Valid`]:
///
/// ```rust
/// use axum_validated_extractors::HasValidate;
/// use validator::Validate;
///
/// struct MyExtractor<T>(T);
///
/// impl<T: Validate> HasValidate for MyExtractor<T> {
///     type Data = T;
///     fn get_validate(&self) -> &T {
///         &self.0
///     }
/// }
/// ```
pub trait HasValidate {
    /// The value that carries the validation rules.
    type Data: Validate;

    /// Borrows the value to validate.
    fn get_validate(&self) -> &Self::Data;
}

impl<T: Validate> HasValidate for axum::extract::Json<T> {
    type Data = T;
    fn get_validate(&self) -> &T {
        &self.0
    }
}

impl<T: Validate> HasValidate for axum::extract::Form<T> {
    type Data = T;
    fn get_validate(&self) -> &T {
        &self.0
    }
}

impl<T: Validate> HasValidate for axum::extract::Query<T> {
    type Data = T;
    fn get_validate(&self) -> &T {
        &self.0
    }
}

impl<T: Validate> HasValidate for axum::extract::Path<T> {
    type Data = T;
    fn get_validate(&self) -> &T {
        &self.0
    }
}

/// Validating wrapper around any extractor implementing [`HasValidate`]
///
/// Where [`ValidatedJson`] and friends are fixed to one source, `Valid` composes over
/// whichever extractor you put inside it — including [`axum::extract::Path`]:
///
/// ```rust
/// use axum::{extract::{Path, Query}, routing::get, Router};
/// use axum_validated_extractors::Valid;
/// use serde::Deserialize;
/// use validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct UserPath {
///     #[validate(range(min = 1))]
///     id: u64,
/// }
///
/// #[derive(Deserialize, Validate)]
/// struct Page {
///     #[validate(range(min = 1))]
///     page: u32,
/// }
///
/// async fn get_user(Valid(Path(user)): Valid<Path<UserPath>>, Valid(Query(p)): Valid<Query<Page>>) {
///     println!("user {} page {}", user.id, p.page);
/// }
///
/// let app: Router<()> = Router::new().route("/users/{id}", get(get_user));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Valid<E>(pub E);

/// Rejection for [`Valid`]: either the inner extractor rejected the request, or the value
/// it produced failed validation.
#[derive(Debug, Error)]
pub enum ValidRejection<R> {
    /// The value was extracted but failed `Validate::validate`.
    #[error(transparent)]
    Invalid(#[from] validator::ValidationErrors),

    /// The inner extractor rejected the request.
    #[error(transparent)]
    Inner(R),
}

impl<R: IntoResponse> IntoResponse for ValidRejection<R> {
    fn into_response(self) -> Response {
        match self {
            // Same body as the fixed wrappers produce.
            ValidRejection::Invalid(errors) => ValidationError::from(errors).into_response(),
            // The inner extractor picked its own status; keep it.
            ValidRejection::Inner(rejection) => rejection.into_response(),
        }
    }
}

impl<E, S> axum::extract::FromRequest<S> for Valid<E>
where
    E: axum::extract::FromRequest<S> + HasValidate,
    S: Send + Sync,
{
    type Rejection = ValidRejection<E::Rejection>;

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let inner = E::from_request(req, state)
            .await
            .map_err(ValidRejection::Inner)?;
        inner.get_validate().validate()?;
        Ok(Valid(inner))
    }
}

impl<E, S> axum::extract::FromRequestParts<S> for Valid<E>
where
    E: axum::extract::FromRequestParts<S> + HasValidate,
    S: Send + Sync,
{
    type Rejection = ValidRejection<E::Rejection>;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let inner = E::from_request_parts(parts, state)
            .await
            .map_err(ValidRejection::Inner)?;
        inner.get_validate().validate()?;
        Ok(Valid(inner))
    }
}
