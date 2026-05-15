use base64::prelude::*;
use serde::de::DeserializeOwned;
use std::ops::Deref;
use std::sync::OnceLock;

use crate::error::XUserInfoError;

/// Default maximum size for base64 encoded header (16KB)
const DEFAULT_MAX_HEADER_SIZE: usize = 16_384;

/// Default maximum size for decoded JSON payload (16KB)
const DEFAULT_MAX_PAYLOAD_SIZE: usize = 16_384;

/// Global configuration instance
static CONFIG: OnceLock<XUserInfoConfig> = OnceLock::new();

/// Configuration for XUserInfo size limits
#[derive(Debug, Clone, Copy)]
pub struct XUserInfoConfig {
    max_header_size: usize,
    max_payload_size: usize,
}

impl Default for XUserInfoConfig {
    fn default() -> Self {
        Self {
            max_header_size: DEFAULT_MAX_HEADER_SIZE,
            max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
        }
    }
}

impl XUserInfoConfig {
    /// Create a new configuration builder
    pub fn builder() -> XUserInfoConfigBuilder {
        XUserInfoConfigBuilder::default()
    }

    /// Get the maximum header size
    pub fn max_header_size(&self) -> usize {
        self.max_header_size
    }

    /// Get the maximum payload size
    pub fn max_payload_size(&self) -> usize {
        self.max_payload_size
    }
}

/// Builder for XUserInfoConfig
#[derive(Debug, Default)]
pub struct XUserInfoConfigBuilder {
    max_header_size: Option<usize>,
    max_payload_size: Option<usize>,
}

impl XUserInfoConfigBuilder {
    /// Set maximum header size (base64 encoded)
    pub fn max_header_size(mut self, size: usize) -> Self {
        self.max_header_size = Some(size);
        self
    }

    /// Set maximum payload size (decoded JSON)
    pub fn max_payload_size(mut self, size: usize) -> Self {
        self.max_payload_size = Some(size);
        self
    }

    /// Disable size limits (use usize::MAX)
    ///
    /// # Warning
    ///
    /// This removes DoS protection. Only use in trusted environments.
    pub fn no_limits(mut self) -> Self {
        self.max_header_size = Some(usize::MAX);
        self.max_payload_size = Some(usize::MAX);
        self
    }

    /// Build the configuration
    pub fn build(self) -> XUserInfoConfig {
        XUserInfoConfig {
            max_header_size: self.max_header_size.unwrap_or(DEFAULT_MAX_HEADER_SIZE),
            max_payload_size: self.max_payload_size.unwrap_or(DEFAULT_MAX_PAYLOAD_SIZE),
        }
    }
}

/// Set global configuration for XUserInfo
///
/// This must be called before any request processing, typically in main().
/// Can only be set once - subsequent calls are ignored.
///
/// # Example
///
/// ```ignore
/// apisix_rs::set_config(
///     apisix_rs::XUserInfoConfig::builder()
///         .max_header_size(32_768)
///         .max_payload_size(32_768)
///         .build()
/// );
/// ```
pub fn set_config(config: XUserInfoConfig) {
    CONFIG.set(config).ok();
}

/// Get the current configuration, or default if not set
fn get_config() -> &'static XUserInfoConfig {
    CONFIG.get_or_init(XUserInfoConfig::default)
}

#[derive(Debug)]
pub struct XUserInfo<T>(pub T)
where
    T: DeserializeOwned;

impl<T> XUserInfo<T>
where
    T: DeserializeOwned,
{
    /// Decode x-userinfo from base64 encoded header value
    ///
    /// # Security
    ///
    /// This function enforces size limits to prevent DoS attacks.
    /// Default limits (configurable via [`set_config`]):
    /// - Max header size: 16KB (before base64 decode)
    /// - Max payload size: 16KB (after base64 decode)
    pub fn decode(header_value: &str) -> Result<Self, XUserInfoError> {
        Self::decode_with_config(header_value, get_config())
    }

    /// Decode with explicit config (internal helper, also useful for testing)
    fn decode_with_config(
        header_value: &str,
        config: &XUserInfoConfig,
    ) -> Result<Self, XUserInfoError> {
        // Check header size before decoding to prevent DoS
        if header_value.len() > config.max_header_size() {
            return Err(XUserInfoError::HeaderTooLarge);
        }

        let base64_decoded = BASE64_STANDARD.decode(header_value)?;

        // Check decoded payload size to prevent memory exhaustion
        if base64_decoded.len() > config.max_payload_size() {
            return Err(XUserInfoError::PayloadTooLarge);
        }

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

    #[test]
    fn test_header_too_large() {
        // Create a header larger than default 16KB
        let large_data = "x".repeat(20_000);
        let encoded = BASE64_STANDARD.encode(large_data.as_bytes());

        let result = XUserInfo::<TestUserInfo>::decode(&encoded);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::HeaderTooLarge
        ));
    }

    #[test]
    fn test_payload_too_large() {
        // Create a payload that decodes to larger than limit
        // Use custom config with large header limit but small payload limit
        let config = XUserInfoConfig::builder()
            .max_header_size(100_000) // Large enough for base64 encoded data
            .max_payload_size(10_000) // Small payload limit
            .build();

        let large_json = format!(
            r#"{{"sub":"user123","name":"{}"}}"#,
            "x".repeat(15_000) // > 10KB payload
        );
        let encoded = BASE64_STANDARD.encode(large_json.as_bytes());

        let result = XUserInfo::<TestUserInfo>::decode_with_config(&encoded, &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            XUserInfoError::PayloadTooLarge
        ));
    }

    #[test]
    fn test_config_builder_default() {
        let config = XUserInfoConfig::builder().build();
        assert_eq!(config.max_header_size(), 16_384);
        assert_eq!(config.max_payload_size(), 16_384);
    }

    #[test]
    fn test_config_builder_custom() {
        let config = XUserInfoConfig::builder()
            .max_header_size(32_768)
            .max_payload_size(65_536)
            .build();
        assert_eq!(config.max_header_size(), 32_768);
        assert_eq!(config.max_payload_size(), 65_536);
    }

    #[test]
    fn test_config_builder_no_limits() {
        let config = XUserInfoConfig::builder().no_limits().build();
        assert_eq!(config.max_header_size(), usize::MAX);
        assert_eq!(config.max_payload_size(), usize::MAX);
    }

    #[test]
    fn test_config_builder_partial() {
        let config = XUserInfoConfig::builder().max_header_size(8_192).build();
        assert_eq!(config.max_header_size(), 8_192);
        assert_eq!(config.max_payload_size(), 16_384); // default
    }
}
