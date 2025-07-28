#[cfg(feature = "std")]
use serde::Deserialize;
use serde::Serialize;
#[cfg(not(feature = "std"))]
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize,
};

#[cfg(feature = "introspection")]
use crate::introspect;

use super::Info;
#[cfg(feature = "idl")]
use super::InterfaceDescription;

/// `org.varlink.service` interface methods.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[serde(tag = "method", content = "parameters")]
pub enum Method<'a> {
    /// Get information about the Varlink service.
    #[serde(rename = "org.varlink.service.GetInfo")]
    GetInfo,
    /// Get the description of the specified interface.
    #[serde(rename = "org.varlink.service.GetInterfaceDescription")]
    GetInterfaceDescription {
        /// The interface to get the description for.
        interface: &'a str,
    },
}

/// `org.varlink.service` interface replies.
///
/// This enum represents all possible replies from the varlink service interface methods.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "idl-parse", derive(Deserialize))]
#[serde(untagged)]
pub enum Reply<'a> {
    /// Reply for `GetInfo` method.
    #[serde(borrow)]
    Info(Info<'a>),
    /// Reply for `GetInterfaceDescription` method.
    /// Note: InterfaceDescription only supports 'static lifetime for deserialization.
    #[cfg(feature = "idl")]
    InterfaceDescription(InterfaceDescription<'static>),
}

/// Errors that can be returned by the `org.varlink.service` interface.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "introspection", derive(introspect::ReplyError))]
#[cfg_attr(feature = "introspection", zlink(crate = "crate"))]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

impl core::error::Error for Error<'_> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}

impl core::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::InterfaceNotFound { interface } => {
                write!(f, "Interface not found: {}", interface)
            }
            Error::MethodNotFound { method } => {
                write!(f, "Method not found: {}", method)
            }
            Error::InvalidParameter { parameter } => {
                write!(f, "Invalid parameter: {}", parameter)
            }
            Error::PermissionDenied => {
                write!(f, "Permission denied")
            }
            Error::ExpectedMore => {
                write!(f, "Expected more")
            }
            Error::MethodNotImplemented { method } => {
                write!(f, "Method not implemented: {}", method)
            }
        }
    }
}

/// Result type for Varlink service methods.
pub type Result<'a, T> = core::result::Result<T, Error<'a>>;

#[cfg(not(feature = "std"))]
impl<'de> Deserialize<'de> for Error<'de> {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ErrorVisitor;

        impl<'de> Visitor<'de> for ErrorVisitor {
            type Value = Error<'de>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a varlink service error")
            }

            fn visit_map<M>(self, mut map: M) -> core::result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                // NOTE: For nostd, we require "error" field to be first.
                // First, read the error type.
                let key = map.next_key::<&str>()?;
                if key != Some("error") {
                    return Err(de::Error::custom("expected 'error' field first"));
                }
                let error_type: &str = map.next_value()?;

                // Helper struct to deserialize parameters
                struct ParamsMap<'de> {
                    interface: Option<&'de str>,
                    method: Option<&'de str>,
                    parameter: Option<&'de str>,
                }

                impl<'de> Deserialize<'de> for ParamsMap<'de> {
                    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
                    where
                        D: serde::Deserializer<'de>,
                    {
                        struct ParamsMapVisitor;

                        impl<'de> Visitor<'de> for ParamsMapVisitor {
                            type Value = ParamsMap<'de>;

                            fn expecting(
                                &self,
                                formatter: &mut core::fmt::Formatter<'_>,
                            ) -> core::fmt::Result {
                                formatter.write_str("parameters object")
                            }

                            fn visit_map<A>(
                                self,
                                mut map: A,
                            ) -> core::result::Result<Self::Value, A::Error>
                            where
                                A: MapAccess<'de>,
                            {
                                let mut interface = None;
                                let mut method = None;
                                let mut parameter = None;

                                while let Some(key) = map.next_key::<&str>()? {
                                    match key {
                                        "interface" => interface = Some(map.next_value()?),
                                        "method" => method = Some(map.next_value()?),
                                        "parameter" => parameter = Some(map.next_value()?),
                                        _ => {
                                            let _: de::IgnoredAny = map.next_value()?;
                                        }
                                    }
                                }

                                Ok(ParamsMap {
                                    interface,
                                    method,
                                    parameter,
                                })
                            }
                        }

                        deserializer.deserialize_map(ParamsMapVisitor)
                    }
                }

                let params_map = loop {
                    let Some(key) = map.next_key::<&str>()? else {
                        break ParamsMap {
                            interface: None,
                            method: None,
                            parameter: None,
                        };
                    };
                    if key == "parameters" {
                        break map.next_value::<ParamsMap<'_>>()?;
                    }
                    // Unknown field, skip it.
                    let _: de::IgnoredAny = map.next_value()?;
                };

                match error_type {
                    "org.varlink.service.PermissionDenied" => return Ok(Error::PermissionDenied),
                    "org.varlink.service.ExpectedMore" => return Ok(Error::ExpectedMore),
                    "org.varlink.service.InterfaceNotFound" => {
                        let interface = params_map
                            .interface
                            .ok_or_else(|| de::Error::missing_field("interface"))?;
                        return Ok(Error::InterfaceNotFound { interface });
                    }
                    "org.varlink.service.MethodNotFound" => {
                        let method = params_map
                            .method
                            .ok_or_else(|| de::Error::missing_field("method"))?;
                        return Ok(Error::MethodNotFound { method });
                    }
                    "org.varlink.service.MethodNotImplemented" => {
                        let method = params_map
                            .method
                            .ok_or_else(|| de::Error::missing_field("method"))?;
                        return Ok(Error::MethodNotImplemented { method });
                    }
                    "org.varlink.service.InvalidParameter" => {
                        let parameter = params_map
                            .parameter
                            .ok_or_else(|| de::Error::missing_field("parameter"))?;
                        return Ok(Error::InvalidParameter { parameter });
                    }
                    _ => {}
                }

                Err(de::Error::unknown_variant(
                    error_type,
                    &[
                        "org.varlink.service.InterfaceNotFound",
                        "org.varlink.service.MethodNotFound",
                        "org.varlink.service.MethodNotImplemented",
                        "org.varlink.service.InvalidParameter",
                        "org.varlink.service.PermissionDenied",
                        "org.varlink.service.ExpectedMore",
                    ],
                ))
            }
        }

        deserializer.deserialize_map(ErrorVisitor)
    }
}

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
        #[cfg(not(feature = "std"))]
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
        #[cfg(not(feature = "std"))]
        let json = {
            use mayheap::string::String;
            let mut buffer = [0u8; 256];
            let len = serde_json_core::to_slice(&err, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 256>::from_slice(&buffer[..len]).unwrap();
            String::<256>::from_utf8(vec).unwrap()
        };

        assert!(json.contains("org.varlink.service.PermissionDenied"));
    }

    #[test]
    fn error_deserialization() {
        // Test error with parameter
        let json = r#"{"error":"org.varlink.service.InterfaceNotFound","parameters":{"interface":"com.example.missing"}}"#;
        let err = deserialize_error(json);
        assert_eq!(
            err,
            Error::InterfaceNotFound {
                interface: "com.example.missing"
            }
        );

        // Test error without parameters
        let json = r#"{"error":"org.varlink.service.PermissionDenied"}"#;
        let err = deserialize_error(json);
        assert_eq!(err, Error::PermissionDenied);

        // Test MethodNotFound error
        let json = r#"{"error":"org.varlink.service.MethodNotFound","parameters":{"method":"NonExistentMethod"}}"#;
        let err = deserialize_error(json);
        assert_eq!(
            err,
            Error::MethodNotFound {
                method: "NonExistentMethod"
            }
        );

        // Test InvalidParameter error
        let json = r#"{"error":"org.varlink.service.InvalidParameter","parameters":{"parameter":"invalid_param"}}"#;
        let err = deserialize_error(json);
        assert_eq!(
            err,
            Error::InvalidParameter {
                parameter: "invalid_param"
            }
        );

        // Test MethodNotImplemented error
        let json = r#"{"error":"org.varlink.service.MethodNotImplemented","parameters":{"method":"UnimplementedMethod"}}"#;
        let err = deserialize_error(json);
        assert_eq!(
            err,
            Error::MethodNotImplemented {
                method: "UnimplementedMethod"
            }
        );

        // Test ExpectedMore error
        let json = r#"{"error":"org.varlink.service.ExpectedMore"}"#;
        let err = deserialize_error(json);
        assert_eq!(err, Error::ExpectedMore);
    }

    #[test]
    fn error_round_trip_serialization() {
        // Test with error that has parameters
        let original = Error::InterfaceNotFound {
            interface: "com.example.missing",
        };

        test_round_trip_serialize(&original);

        // Test with error that has no parameters
        let original = Error::PermissionDenied;

        test_round_trip_serialize(&original);
    }

    // Helper function to deserialize JSON string to Error, abstracting std vs nostd differences
    fn deserialize_error(json: &str) -> Error<'_> {
        #[cfg(feature = "std")]
        {
            serde_json::from_str(json).unwrap()
        }
        #[cfg(not(feature = "std"))]
        {
            let (err, _): (Error<'_>, usize) = serde_json_core::from_str(json).unwrap();
            err
        }
    }

    // Helper function for round-trip serialization test, abstracting std vs nostd differences
    fn test_round_trip_serialize<'a>(original: &Error<'a>) {
        #[cfg(feature = "std")]
        {
            let json = serde_json::to_string(original).unwrap();
            let deserialized: Error<'_> = serde_json::from_str(&json).unwrap();
            assert_eq!(*original, deserialized);
        }
        #[cfg(not(feature = "std"))]
        {
            let mut buffer = [0u8; 256];
            let len = serde_json_core::to_slice(original, &mut buffer).unwrap();
            let json_bytes = &buffer[..len];
            let (deserialized, _): (Error<'_>, usize) =
                serde_json_core::from_slice(json_bytes).unwrap();
            assert_eq!(*original, deserialized);
        }
    }
}
