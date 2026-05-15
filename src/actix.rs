use std::future::{Ready, ready};

use actix_web::{FromRequest, HttpRequest, HttpResponse, ResponseError, dev::Payload};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{X_USER_INFO_HEADER, error::XUserInfoError, user_info::XUserInfo};

impl ResponseError for XUserInfoError {
    fn error_response(&self) -> HttpResponse {
        let error_json = json!({
            "error": self.to_string()
        });
        HttpResponse::build(self.status_code()).json(error_json)
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::BAD_REQUEST
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
            .to_str()
            .map_err(|_| XUserInfoError::InvalidHeader)?;

        XUserInfo::decode(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use base64::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::{X_USER_INFO_HEADER, error::XUserInfoError, user_info::XUserInfo};

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

    #[actix_rt::test]
    async fn test_missing_header() {
        let req = TestRequest::default().to_http_request();
        let mut payload = Payload::None;
        let result: Result<XUserInfo<CustomXUserInfo>, _> =
            XUserInfo::from_request(&req, &mut payload).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), XUserInfoError::MissingHeader));
    }

    #[actix_rt::test]
    async fn test_invalid_base64() {
        let req = TestRequest::default()
            .append_header((X_USER_INFO_HEADER, "not-valid-base64!!!"))
            .to_http_request();
        let mut payload = Payload::None;
        let result: Result<XUserInfo<CustomXUserInfo>, _> =
            XUserInfo::from_request(&req, &mut payload).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::Base64DecodeError(_)
        ));
    }

    #[actix_rt::test]
    async fn test_invalid_json() {
        let invalid_json = "not a json";
        let encoded = BASE64_STANDARD.encode(invalid_json.as_bytes());

        let req = TestRequest::default()
            .append_header((X_USER_INFO_HEADER, encoded))
            .to_http_request();
        let mut payload = Payload::None;
        let result: Result<XUserInfo<CustomXUserInfo>, _> =
            XUserInfo::from_request(&req, &mut payload).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::JsonDecodeError(_)
        ));
    }

    #[actix_rt::test]
    async fn test_deref_works() {
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

        // Test deref
        assert_eq!(x_user_info.sub, "test sub");
        assert_eq!(x_user_info.name, "test name");
    }

    #[actix_rt::test]
    async fn test_error_response() {
        use actix_web::ResponseError;

        // Test MissingHeader error response
        let error = XUserInfoError::MissingHeader;
        let response = error.error_response();
        assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);

        // Test InvalidHeader error response
        let error = XUserInfoError::InvalidHeader;
        let response = error.error_response();
        assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }
}
