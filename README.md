# axum-validated-extractors

[![Crates.io](https://img.shields.io/crates/v/axum-validated-extractors.svg)](https://crates.io/crates/axum-validated-extractors)
[![Documentation](https://docs.rs/axum-validated-extractors/badge.svg)](https://docs.rs/axum-validated-extractors)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/truehazker/axum-validated-extractors/actions/workflows/ci.yml/badge.svg)](https://github.com/truehazker/axum-validated-extractors/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/rust-1.88.0+-blue.svg)](https://www.rust-lang.org)
[![Dependency Status](https://deps.rs/repo/github/truehazker/axum-validated-extractors/status.svg)](https://deps.rs/repo/github/truehazker/axum-validated-extractors)

A collection of validated extractors for Axum that automatically validate the extracted data using the `validator` crate. This library provides type-safe, automatic validation for your Axum web applications, making it easier to handle and validate incoming requests.

## Features

- `ValidatedForm`: Validates URL-encoded form data
- `ValidatedJson`: Validates JSON data
- `ValidatedQuery`: Validates query parameters
- `Valid`: Validates whatever extractor you wrap, including `Path`
- Automatic validation using the `validator` crate
- Type-safe error handling

## Installation

Install the crate:

```bash
cargo add axum-validated-extractors validator
```

> ⚠️ `validator` crate is required for validation schema creation.

## Usage

### Basic Example

```rust
use axum::{
    routing::post,
    Router,
};
use axum_validated_extractors::{ValidatedJson, ValidatedForm, ValidatedQuery};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3))]
    username: String,
    #[validate(email)]
    email: String,
}

async fn create_user(
    ValidatedJson(user): ValidatedJson<CreateUser>,
) {
    // user is guaranteed to be valid
    println!("Creating user: {:?}", user);
}

let app: Router<()> = Router::new()
    .route("/users", post(create_user));
```

### Form Data

```rust
use axum::{
    routing::post,
    Router,
};
use axum_validated_extractors::ValidatedForm;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct LoginForm {
    #[validate(length(min = 3))]
    username: String,
    #[validate(length(min = 8))]
    password: String,
}

async fn login(
    ValidatedForm(form): ValidatedForm<LoginForm>,
) {
    // form is guaranteed to be valid
    println!("Logging in user: {:?}", form);
}

let app: Router<()> = Router::new()
    .route("/login", post(login));
```

### Query Parameters

```rust
use axum::{
    routing::get,
    Router,
};
use axum_validated_extractors::ValidatedQuery;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct SearchParams {
    #[validate(length(min = 2))]
    query: String,
    #[validate(range(min = 1, max = 100))]
    page: Option<u32>,
}

async fn search(
    ValidatedQuery(params): ValidatedQuery<SearchParams>,
) {
    // params is guaranteed to be valid
    println!("Searching with params: {:?}", params);
}

let app: Router<()> = Router::new()
    .route("/search", get(search));
```

## Validation Rules

The validation rules are provided by the `validator` crate. Here are some common validation rules:

```rust
#[derive(Debug, Deserialize, Validate)]
struct User {
    #[validate(length(min = 3, max = 20))]
    username: String,

    #[validate(email)]
    email: String,

    #[validate(range(min = 18))]
    age: u32,

    #[validate(url)]
    website: Option<String>,

    #[validate(length(min = 1))]
    nickname: String,
}
```

See the [validator documentation](https://docs.rs/validator) for more validation rules.

### Wrapping any extractor with `Valid`

`ValidatedJson`/`ValidatedForm`/`ValidatedQuery` are each fixed to one source. `Valid<E>`
is generic over the extractor it wraps, so it also covers `Path`:

```rust
use axum::extract::{Path, Query};
use axum_validated_extractors::Valid;

#[derive(Debug, Deserialize, Validate)]
struct UserPath {
    #[validate(range(min = 1))]
    id: u64,
}

// Path param and query string are both validated before the handler runs.
async fn get_user(
    Valid(Path(user)): Valid<Path<UserPath>>,
    Valid(Query(params)): Valid<Query<SearchParams>>,
) {
    println!("user {}", user.id);
}

let app: Router<()> = Router::new()
    .route("/users/{id}", get(get_user));
```

To use `Valid` with your own extractor, implement the one-method `HasValidate` trait:

```rust
use axum_validated_extractors::HasValidate;

impl<T: Validate> HasValidate for MyExtractor<T> {
    type Data = T;
    fn get_validate(&self) -> &T {
        &self.0
    }
}
```

Both styles are supported; neither is deprecated. `ValidatedJson<T>` destructures in one
step, `Valid<Json<T>>` goes anywhere.

## Error Handling

The extractors return a `ValidationError`, which is converted into a response for you. A
validation failure is `400 Bad Request`; a deserialization failure keeps the status of the
underlying Axum rejection (`415` on a missing content type, `422` on an unexpected shape).

To build the response yourself, take the extractor as a `Result`:

```rust
use axum::{
    routing::post,
    Router,
    response::IntoResponse,
};
use axum_validated_extractors::{ValidatedJson, ValidationError};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3))]
    username: String,
    #[validate(email)]
    email: String,
}

async fn create_user(
    user: Result<ValidatedJson<CreateUser>, ValidationError>,
) -> impl IntoResponse {
    match user {
        Ok(ValidatedJson(user)) => format!("created {}", user.username),
        Err(ValidationError::ValidationError(errors)) => format!("invalid: {errors}"),
        Err(other) => format!("bad request: {other}"),
    }
}

let app: Router<()> = Router::new()
    .route("/users", post(create_user));
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
