use base64::prelude::*;
use serde::de::DeserializeOwned;
use std::ops::Deref;

use crate::error::XUserInfoError;

#[derive(Debug)]
pub struct XUserInfo<T>(pub T)
where
    T: DeserializeOwned;

impl<T> XUserInfo<T>
where
    T: DeserializeOwned,
{
    /// Decode x-userinfo from base64 encoded header value
    pub fn decode(header_value: &str) -> Result<Self, XUserInfoError> {
        let base64_decoded = BASE64_STANDARD.decode(header_value)?;
        Ok(XUserInfo(serde_json::from_slice(&base64_decoded)?))
    }
}

impl<T> Deref for XUserInfo<T>
where
    T: DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Debug, PartialEq, Serialize)]
    struct TestUserInfo {
        sub: String,
        name: String,
    }

    #[test]
    fn test_decode_success() {
        let data = r#"{"sub":"user123","name":"John Doe"}"#;
        let encoded = BASE64_STANDARD.encode(data.as_bytes());

        let result = XUserInfo::<TestUserInfo>::decode(&encoded);
        assert!(result.is_ok());

        let user_info = result.unwrap();
        assert_eq!(user_info.sub, "user123");
        assert_eq!(user_info.name, "John Doe");
    }

    #[test]
    fn test_decode_invalid_base64() {
        let result = XUserInfo::<TestUserInfo>::decode("not-valid-base64!!!");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::Base64DecodeError(_)
        ));
    }

    #[test]
    fn test_decode_invalid_json() {
        let invalid_json = "not a json object";
        let encoded = BASE64_STANDARD.encode(invalid_json.as_bytes());

        let result = XUserInfo::<TestUserInfo>::decode(&encoded);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::JsonDecodeError(_)
        ));
    }

    #[test]
    fn test_decode_valid_base64_but_invalid_json_structure() {
        let invalid_structure = r#"{"wrong":"fields"}"#;
        let encoded = BASE64_STANDARD.encode(invalid_structure.as_bytes());

        let result = XUserInfo::<TestUserInfo>::decode(&encoded);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::JsonDecodeError(_)
        ));
    }

    #[test]
    fn test_deref() {
        let data = r#"{"sub":"user123","name":"John Doe"}"#;
        let encoded = BASE64_STANDARD.encode(data.as_bytes());
        let user_info = XUserInfo::<TestUserInfo>::decode(&encoded).unwrap();

        // Test deref works
        assert_eq!(user_info.sub, "user123");
        assert_eq!(user_info.name, "John Doe");
    }
}
