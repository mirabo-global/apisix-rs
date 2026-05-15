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

For custom error handling, accept `Result` instead:

**Actix-web:**
```rust
use actix_web::{HttpResponse, web::Json};
use serde_json::json;

#[get("/profile")]
async fn profile(
    user: Result<XUserInfo<UserInfo>, XUserInfoError>
) -> HttpResponse {
    match user {
        Ok(user_info) => HttpResponse::Ok().json(user_info),
        Err(e) => {
            // Custom error response
            HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": e.to_string(),
                "code": "AUTH_FAILED"
            }))
        }
    }
}
```

**Axum:**
```rust
use axum::{http::StatusCode, response::{IntoResponse, Json}};
use serde_json::json;

async fn profile(
    user: Result<XUserInfo<UserInfo>, XUserInfoError>
) -> impl IntoResponse {
    match user {
        Ok(user_info) => Json(user_info).into_response(),
        Err(e) => {
            // Custom error response
            let error_response = json!({
                "success": false,
                "message": e.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            (StatusCode::UNAUTHORIZED, Json(error_response)).into_response()
        }
    }
}
```

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
