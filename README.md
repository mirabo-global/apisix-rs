# apisix-rs

[![Crates.io](https://img.shields.io/crates/v/apisix-rs.svg)](https://crates.io/crates/apisix-rs)
[![Documentation](https://docs.rs/apisix-rs/badge.svg)](https://docs.rs/apisix-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rust utilities for working with Apache APISIX, providing easy integration with popular web frameworks.

## Features

- **Framework Support**: Built-in extractors for `actix-web` and `axum`
- **Type-Safe**: Decode and validate `x-userinfo` headers with type safety
- **Efficient**: Fast base64 decoding and JSON parsing with minimal overhead
- **Error Handling**: Comprehensive error types with helpful messages

## Installation

Add this to your `Cargo.toml`:

### For actix-web

```toml
[dependencies]
apisix-rs = { version = "1.1", features = ["actix"] }
```

### For axum

```toml
[dependencies]
apisix-rs = { version = "1.1", features = ["axum"] }
```

## Usage

### With actix-web

```rust
use actix_web::{get, web, App, HttpServer, Responder};
use apisix_rs::XUserInfo;
use serde::Deserialize;

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    name: String,
    email: String,
}

#[get("/profile")]
async fn profile(user: XUserInfo<UserInfo>) -> impl Responder {
    format!("Hello, {}! Email: {}", user.name, user.email)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(profile)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

### With axum

```rust
use axum::{
    routing::get,
    Router,
};
use apisix_rs::XUserInfo;
use serde::Deserialize;

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    name: String,
    email: String,
}

async fn profile(XUserInfo(user): XUserInfo<UserInfo>) -> String {
    format!("Hello, {}! Email: {}", user.name, user.email)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/profile", get(profile));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## How it works

Apache APISIX can be configured to forward user information via the `x-userinfo` header. This library provides extractors that:

1. Extract the `x-userinfo` header from the request
2. Decode the base64-encoded value
3. Parse the JSON payload into your custom type
4. Return comprehensive errors if any step fails

## Error Handling

### Default Error Response

By default, extraction errors return a JSON response with 400 Bad Request:

```json
{
  "error": "Missing x-userinfo header"
}
```

**Actix-web example:**
```rust
#[get("/profile")]
async fn profile(user: XUserInfo<UserInfo>) -> impl Responder {
    // If header is invalid, automatically returns:
    // HTTP 400 with {"error": "error message"}
    format!("Hello, {}!", user.name)
}
```

**Axum example:**
```rust
async fn profile(user: XUserInfo<UserInfo>) -> impl IntoResponse {
    // If header is invalid, automatically returns:
    // HTTP 400 with {"error": "error message"}
    Json(user)
}
```

### Custom Error Response

There are two supported approaches for custom error handling:

1. Keep using `XUserInfo<T>` and accept `Result<_, XUserInfoError>` in the handler.
2. Use `XUserInfoWith<T, R>` if you want extractor-style usage with an application-defined rejection type.

#### Option 1: Handle `Result<XUserInfo<T>, XUserInfoError>` in the handler

This is the existing approach and still works.

**Actix-web:**
```rust
use actix_web::{HttpResponse, get};
use apisix_rs::{XUserInfo, XUserInfoError};
use serde_json::json;

#[get("/profile")]
async fn profile(
    user: Result<XUserInfo<UserInfo>, XUserInfoError>
) -> HttpResponse {
    match user {
        Ok(user_info) => HttpResponse::Ok().json(&user_info.0),
        Err(e) => HttpResponse::Unauthorized().json(json!({
            "success": false,
            "message": e.to_string(),
            "code": "AUTH_FAILED"
        })),
    }
}
```

**Axum:**
```rust
use apisix_rs::{XUserInfo, XUserInfoError};
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

async fn profile(
    user: Result<XUserInfo<UserInfo>, XUserInfoError>
) -> impl IntoResponse {
    match user {
        Ok(user_info) => Json(&user_info.0).into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}
```

#### Option 2: Use `XUserInfoWith<T, R>` for custom rejection types

This keeps extractor-style handler signatures while letting the application choose its own rejection type.

**Actix-web:**
```rust
use actix_web::{HttpResponse, ResponseError, get};
use apisix_rs::{XUserInfoError, XUserInfoWith};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, serde::Serialize)]
struct UserInfo {
    sub: String,
    name: String,
    email: String,
}

#[derive(Debug)]
enum AppError {
    Unauthorized,
    BadRequest(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::BadRequest(message) => write!(f, "{message}"),
        }
    }
}

impl From<XUserInfoError> for AppError {
    fn from(err: XUserInfoError) -> Self {
        match err {
            XUserInfoError::MissingHeader => Self::Unauthorized,
            _ => Self::BadRequest(err.to_string()),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::Unauthorized => actix_web::http::StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => actix_web::http::StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(json!({
            "success": false,
            "message": self.to_string(),
        }))
    }
}

#[get("/profile")]
async fn profile(user: XUserInfoWith<UserInfo, AppError>) -> HttpResponse {
    HttpResponse::Ok().json(&user.0.0)
}
```

**Axum:**
```rust
use apisix_rs::{XUserInfoError, XUserInfoWith};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, serde::Serialize)]
struct UserInfo {
    sub: String,
    name: String,
    email: String,
}

#[derive(Debug)]
enum AppError {
    Unauthorized,
    BadRequest(String),
}

impl From<XUserInfoError> for AppError {
    fn from(err: XUserInfoError) -> Self {
        match err {
            XUserInfoError::MissingHeader => Self::Unauthorized,
            _ => Self::BadRequest(err.to_string()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "missing x-userinfo header".to_owned()),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
        };

        (status, Json(json!({
            "success": false,
            "message": message,
        }))).into_response()
    }
}

async fn profile(user: XUserInfoWith<UserInfo, AppError>) -> impl IntoResponse {
    Json(&user.0.0)
}
```

`XUserInfo<T>` remains the plug-and-play extractor with the built-in `XUserInfoError` response. `XUserInfoWith<T, R>` lets the application choose its own rejection type without reimplementing header parsing and decoding.

### Error Types

The library provides detailed error types:

```rust
pub enum XUserInfoError {
    MissingHeader,           // x-userinfo header not found
    InvalidHeader,           // Header contains invalid UTF-8
    HeaderTooLarge,          // Header exceeds size limit (DoS protection)
    PayloadTooLarge,         // Decoded payload exceeds size limit (DoS protection)
    Base64DecodeError(..),   // Base64 decoding failed
    JsonDecodeError(..),     // JSON parsing failed
}
```

## Configuration

### Size Limits

The library enforces size limits to prevent DoS attacks. By default:

- **Max header size**: 16KB (base64 encoded)
- **Max payload size**: 16KB (decoded JSON)

These defaults work well for most use cases and align with standard reverse proxy limits (Nginx: 8KB, AWS ALB/Cloudflare: 16KB).

### Custom Configuration

You can customize size limits globally at application startup:

**Actix-web:**
```rust
use apisix_rs::{XUserInfo, set_config, XUserInfoConfig};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set custom limits before starting server
    set_config(
        XUserInfoConfig::builder()
            .max_header_size(32_768)    // 32KB header limit
            .max_payload_size(32_768)   // 32KB payload limit
            .build()
    );
    
    HttpServer::new(|| {
        App::new().service(profile)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

**Axum:**
```rust
use apisix_rs::{XUserInfo, set_config, XUserInfoConfig};

#[tokio::main]
async fn main() {
    // Set custom limits before starting server
    set_config(
        XUserInfoConfig::builder()
            .max_header_size(32_768)    // 32KB header limit
            .max_payload_size(32_768)   // 32KB payload limit
            .build()
    );
    
    let app = Router::new().route("/profile", get(profile));
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Disable Size Limits (Not Recommended)

For trusted environments only:

```rust
set_config(
    XUserInfoConfig::builder()
        .no_limits()  // ⚠️ Removes DoS protection!
        .build()
);
```

**Warning**: Disabling size limits removes DoS protection. Only use in trusted environments where input is validated by other means.

### Configuration Builder Options

```rust
XUserInfoConfig::builder()
    .max_header_size(size)      // Set header limit in bytes
    .max_payload_size(size)     // Set payload limit in bytes
    .no_limits()                // Disable all limits (use usize::MAX)
    .build()
```

**Note**: Configuration must be set before processing any requests. Subsequent calls to `set_config()` are ignored.

## Security Considerations

### Trust Model

This library assumes:
1. **APISIX is the authentication boundary** - It performs OAuth/OIDC verification
2. **Secure communication** - APISIX → App uses internal network or mTLS
3. **Header integrity** - Only APISIX can set `x-userinfo` headers

### Best Practices

1. **Deploy behind APISIX only**
   - Do not expose your application directly to the internet
   - APISIX should be the only entry point

2. **Strip client headers**
   - Use middleware to remove any client-provided `x-userinfo` headers
   - Only trust headers from APISIX

3. **Use appropriate size limits**
   - Default 16KB works for most cases
   - Increase only if you have legitimate large user profiles
   - Never disable limits in production

4. **Network security**
   - Use internal network or VPC for APISIX ↔ App communication
   - Or use mTLS for encrypted communication
   - Prevent direct access to your application

5. **Monitor and log**
   - Log `HeaderTooLarge` and `PayloadTooLarge` errors
   - These may indicate attacks or misconfigurations

### Defense in Depth

While size limits provide application-level protection, you should also:
- Configure reverse proxy limits (Nginx, AWS ALB, etc.)
- Use rate limiting
- Implement request timeouts
- Monitor resource usage

### Security Reporting

Found a security issue? Please email: tuanla@mirabo-global.com

## Features Flags

- `actix`: Enable actix-web integration
- `axum`: Enable axum integration

**Note**: You must explicitly enable either `actix` or `axum` feature.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
