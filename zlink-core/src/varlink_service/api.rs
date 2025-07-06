#[cfg(feature = "std")]
use serde::Deserialize;
use serde::Serialize;

use crate::introspect;

/// Errors that can be returned by the `org.varlink.service` interface.
#[derive(Debug, Clone, PartialEq, Serialize, introspect::ReplyError)]
#[zlink(crate = "crate")]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[serde(tag = "error", content = "parameters")]
pub enum Error<'a> {
    /// The requested interface was not found.
    #[serde(rename = "org.varlink.service.InterfaceNotFound")]
    InterfaceNotFound {
        /// The interface that was not found.
        interface: &'a str,
    },
    /// The requested method was not found.
    #[serde(rename = "org.varlink.service.MethodNotFound")]
    MethodNotFound {
        /// The method that was not found.
        method: &'a str,
    },
    /// The interface defines the requested method, but the service does not implement it.
    #[serde(rename = "org.varlink.service.MethodNotImplemented")]
    MethodNotImplemented {
        /// The method that is not implemented.
        method: &'a str,
    },
    /// One of the passed parameters is invalid.
    #[serde(rename = "org.varlink.service.InvalidParameter")]
    InvalidParameter {
        /// The parameter that is invalid.
        parameter: &'a str,
    },
    /// Client is denied access.
    #[serde(rename = "org.varlink.service.PermissionDenied")]
    PermissionDenied,
    /// Method is expected to be called with 'more' set to true, but wasn't.
    #[serde(rename = "org.varlink.service.ExpectedMore")]
    ExpectedMore,
}

/// Result type for Varlink service methods.
pub type Result<'a, T> = core::result::Result<T, Error<'a>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serialization() {
        let err = Error::InterfaceNotFound {
            interface: "com.example.missing",
        };

        #[cfg(feature = "std")]
        let json = serde_json::to_string(&err).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            use mayheap::string::String;
            let mut buffer = [0u8; 256];
            let len = serde_json_core::to_slice(&err, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 256>::from_slice(&buffer[..len]).unwrap();
            String::<256>::from_utf8(vec).unwrap()
        };

        assert!(json.contains("org.varlink.service.InterfaceNotFound"));
        assert!(json.contains("com.example.missing"));

        let err = Error::PermissionDenied;

        #[cfg(feature = "std")]
        let json = serde_json::to_string(&err).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            use mayheap::string::String;
            let mut buffer = [0u8; 256];
            let len = serde_json_core::to_slice(&err, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 256>::from_slice(&buffer[..len]).unwrap();
            String::<256>::from_utf8(vec).unwrap()
        };

        assert!(json.contains("org.varlink.service.PermissionDenied"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn error_deserialization() {
        // Test error with parameter
        let json = r#"{"error":"org.varlink.service.InterfaceNotFound","parameters":{"interface":"com.example.missing"}}"#;

        let err: Error<'_> = serde_json::from_str(json).unwrap();
        assert_eq!(
            err,
            Error::InterfaceNotFound {
                interface: "com.example.missing"
            }
        );

        // Test error without parameters
        let json = r#"{"error":"org.varlink.service.PermissionDenied"}"#;

        let err: Error<'_> = serde_json::from_str(json).unwrap();
        assert_eq!(err, Error::PermissionDenied);

        // Test MethodNotFound error
        let json = r#"{"error":"org.varlink.service.MethodNotFound","parameters":{"method":"NonExistentMethod"}}"#;

        let err: Error<'_> = serde_json::from_str(json).unwrap();
        assert_eq!(
            err,
            Error::MethodNotFound {
                method: "NonExistentMethod"
            }
        );

        // Test InvalidParameter error
        let json = r#"{"error":"org.varlink.service.InvalidParameter","parameters":{"parameter":"invalid_param"}}"#;

        let err: Error<'_> = serde_json::from_str(json).unwrap();
        assert_eq!(
            err,
            Error::InvalidParameter {
                parameter: "invalid_param"
            }
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn error_round_trip_serialization() {
        // Test with error that has parameters
        let original = Error::InterfaceNotFound {
            interface: "com.example.missing",
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back from JSON
        let deserialized: Error<'_> = serde_json::from_str(&json).unwrap();

        // Verify they are equal
        assert_eq!(original, deserialized);

        // Test with error that has no parameters
        let original = Error::PermissionDenied;

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back from JSON
        let deserialized: Error<'_> = serde_json::from_str(&json).unwrap();

        // Verify they are equal
        assert_eq!(original, deserialized);
    }
}
