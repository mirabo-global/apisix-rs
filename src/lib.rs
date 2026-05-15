mod error;
mod user_info;

#[cfg(feature = "actix")]
mod actix;

#[cfg(feature = "axum")]
mod axum;

const X_USER_INFO_HEADER: &str = "x-userinfo";

// Re-export public API
pub use error::XUserInfoError;
pub use user_info::XUserInfo;
