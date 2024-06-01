#[cfg(feature = "actix")]
mod actix;

#[cfg(feature = "actix")]
pub use actix::XUserInfo;

const X_USER_INFO_HEADER: &str = "x-userinfo";
