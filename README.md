# apisix-rs

[![Crates.io](https://img.shields.io/crates/v/apisix-rs.svg)](https://crates.io/crates/apisix-rs)
[![Documentation](https://docs.rs/apisix-rs/badge.svg)](https://docs.rs/apisix-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rust utilities for working with Apache APISIX, providing easy integration with popular web frameworks.

## Features

- **Framework Support**: Built-in extractors for `actix-web` and `axum`
- **Type-Safe**: Decode and validate `x-userinfo` headers with type safety
- **Zero-Copy**: Efficient base64 decoding and JSON parsing
- **Error Handling**: Comprehensive error types with helpful messages

## Installation

Add this to your `Cargo.toml`:

### For actix-web

```toml
[dependencies]
apisix-rs = "0.1"
```

### For axum

```toml
[dependencies]
apisix-rs = { version = "0.1", default-features = false, features = ["axum"] }
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

The library provides detailed error types:

```rust
use apisix_rs::{XUserInfo, XUserInfoError};

match XUserInfo::<UserInfo>::decode(header_value) {
    Ok(user_info) => println!("User: {}", user_info.name),
    Err(XUserInfoError::MissingHeader) => println!("Header not found"),
    Err(XUserInfoError::Base64DecodeError(e)) => println!("Invalid base64: {}", e),
    Err(XUserInfoError::JsonDecodeError(e)) => println!("Invalid JSON: {}", e),
    Err(XUserInfoError::InvalidHeader) => println!("Invalid header format"),
}
```

## Features Flags

- `actix` (default): Enable actix-web integration
- `axum`: Enable axum integration

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
