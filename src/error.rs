use thiserror::Error;

#[derive(Error, Debug)]
pub enum XUserInfoError {
    #[error("x-userinfo header is missing")]
    MissingHeader,

    #[error("invalid x-userinfo header")]
    InvalidHeader,

    #[error("invalid x-userinfo, base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("invalid x-userinfo, json decode error: {0}")]
    JsonDecodeError(#[from] serde_json::Error),
}
