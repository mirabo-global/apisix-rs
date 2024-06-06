use std::{
    future::{ready, Ready},
    ops::Deref,
};

use actix_web::{
    dev::Payload,
    http::header::{self, ContentType},
    FromRequest, HttpRequest, HttpResponse, ResponseError,
};
use base64::prelude::*;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::X_USER_INFO_HEADER;

#[derive(Error, Debug)]
pub enum XUserInfoError {
    #[error("x-userinfo header is missing")]
    MissingHeader,

    #[error("invalid x-userinfo header: {0}")]
    ToStringError(#[from] header::ToStrError),

    #[error("invalid x-userinfo, base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("invalid x-userinfo, json decode error: {0}")]
    JsonDecodeError(#[from] serde_json::Error),
}

impl ResponseError for XUserInfoError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(self.to_string())
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::BAD_REQUEST
    }
}

#[derive(Debug)]
pub struct XUserInfo<T>(T)
where
    T: DeserializeOwned;

impl<T> Deref for XUserInfo<T>
where
    T: DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromRequest for XUserInfo<T>
where
    T: DeserializeOwned,
{
    type Error = XUserInfoError;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(req.try_into())
    }
}

impl<T> TryFrom<&HttpRequest> for XUserInfo<T>
where
    T: DeserializeOwned,
{
    type Error = XUserInfoError;

    fn try_from(req: &HttpRequest) -> Result<Self, Self::Error> {
        let header = req
            .headers()
            .get(X_USER_INFO_HEADER)
            .ok_or(XUserInfoError::MissingHeader)?
            .to_str()?;

        let base64_decoded = BASE64_STANDARD.decode(header)?;

        Ok(XUserInfo(serde_json::from_slice(&base64_decoded)?))
    }
}

#[cfg(test)]
mod tests {
    use crate::X_USER_INFO_HEADER;

    use super::*;
    use actix_web::test::TestRequest;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Deserialize, Debug, PartialEq, Serialize)]
    #[serde(rename_all = "snake_case")]
    struct CustomXUserInfo {
        sub: String,
        name: String,
        iat: u64,
    }

    #[actix_rt::test]
    async fn test_x_user_info() {
        let header_raw = json!({
            "sub": "test sub",
            "name": "test name",
            "iat": 1516239022
        });
        let base64_encoded_header = BASE64_STANDARD.encode(header_raw.to_string().as_bytes());

        let req = TestRequest::default()
            .append_header((X_USER_INFO_HEADER, base64_encoded_header))
            .to_http_request();
        let mut payload = Payload::None;
        let x_user_info: XUserInfo<CustomXUserInfo> =
            XUserInfo::from_request(&req, &mut payload).await.unwrap();

        assert_eq!(x_user_info.0.sub, "test sub");
        assert_eq!(x_user_info.0.name, "test name");
        assert_eq!(x_user_info.0.iat, 1516239022);
    }
}
