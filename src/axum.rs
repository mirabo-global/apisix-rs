use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Json, Response},
};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{X_USER_INFO_HEADER, error::XUserInfoError, user_info::XUserInfo};

impl IntoResponse for XUserInfoError {
    fn into_response(self) -> Response {
        let error_json = json!({
            "error": self.to_string()
        });
        (StatusCode::BAD_REQUEST, Json(error_json)).into_response()
    }
}

impl<T, S> FromRequestParts<S> for XUserInfo<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = XUserInfoError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(X_USER_INFO_HEADER)
            .ok_or(XUserInfoError::MissingHeader)?
            .to_str()
            .map_err(|_| XUserInfoError::InvalidHeader)?;

        XUserInfo::decode(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use base64::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::{error::XUserInfoError, user_info::XUserInfo};

    #[derive(Deserialize, Debug, PartialEq, Serialize)]
    #[serde(rename_all = "snake_case")]
    struct CustomXUserInfo {
        sub: String,
        name: String,
        iat: u64,
    }

    #[tokio::test]
    async fn test_x_user_info() {
        let header_raw = json!({
            "sub": "test sub",
            "name": "test name",
            "iat": 1516239022
        });
        let base64_encoded_header = BASE64_STANDARD.encode(header_raw.to_string().as_bytes());

        let req = Request::builder()
            .header(crate::X_USER_INFO_HEADER, base64_encoded_header)
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = req.into_parts();
        let x_user_info: XUserInfo<CustomXUserInfo> =
            XUserInfo::from_request_parts(&mut parts, &())
                .await
                .unwrap();

        assert_eq!(x_user_info.0.sub, "test sub");
        assert_eq!(x_user_info.0.name, "test name");
        assert_eq!(x_user_info.0.iat, 1516239022);
    }

    #[tokio::test]
    async fn test_x_user_info_missing_header() {
        let req = Request::builder().body(Body::empty()).unwrap();

        let (mut parts, _body) = req.into_parts();
        let result: Result<XUserInfo<CustomXUserInfo>, _> =
            XUserInfo::from_request_parts(&mut parts, &()).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), XUserInfoError::MissingHeader));
    }

    #[tokio::test]
    async fn test_invalid_base64() {
        let req = Request::builder()
            .header(crate::X_USER_INFO_HEADER, "not-valid-base64!!!")
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = req.into_parts();
        let result: Result<XUserInfo<CustomXUserInfo>, _> =
            XUserInfo::from_request_parts(&mut parts, &()).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::Base64DecodeError(_)
        ));
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let invalid_json = "not a json";
        let encoded = BASE64_STANDARD.encode(invalid_json.as_bytes());

        let req = Request::builder()
            .header(crate::X_USER_INFO_HEADER, encoded)
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = req.into_parts();
        let result: Result<XUserInfo<CustomXUserInfo>, _> =
            XUserInfo::from_request_parts(&mut parts, &()).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::JsonDecodeError(_)
        ));
    }

    #[tokio::test]
    async fn test_deref_works() {
        let header_raw = json!({
            "sub": "test sub",
            "name": "test name",
            "iat": 1516239022
        });
        let base64_encoded_header = BASE64_STANDARD.encode(header_raw.to_string().as_bytes());

        let req = Request::builder()
            .header(crate::X_USER_INFO_HEADER, base64_encoded_header)
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = req.into_parts();
        let x_user_info: XUserInfo<CustomXUserInfo> =
            XUserInfo::from_request_parts(&mut parts, &())
                .await
                .unwrap();

        // Test deref
        assert_eq!(x_user_info.sub, "test sub");
        assert_eq!(x_user_info.name, "test name");
    }

    #[tokio::test]
    async fn test_error_into_response() {
        use axum::response::IntoResponse;

        // Test MissingHeader error response
        let error = XUserInfoError::MissingHeader;
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        // Test InvalidHeader error response
        let error = XUserInfoError::InvalidHeader;
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }
}
