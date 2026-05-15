use thiserror::Error;

#[derive(Error, Debug)]
pub enum XUserInfoError {
    #[error("x-userinfo header is missing")]
    MissingHeader,

    #[error("invalid x-userinfo header")]
    InvalidHeader,

    #[error("x-userinfo header too large (max 16KB)")]
    HeaderTooLarge,

    #[error("x-userinfo decoded payload too large (max 16KB)")]
    PayloadTooLarge,

    #[error("invalid x-userinfo, base64 decode error")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("invalid x-userinfo, json decode error")]
    JsonDecodeError(#[from] serde_json::Error),
}
