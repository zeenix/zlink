use zlink_macros::ReplyError;

#[derive(ReplyError, Debug, PartialEq)]
#[zlink(interface = "com.example.Test")]
enum TestError<'a> {
    NotFound,
    PermissionDenied,
    InvalidInput { field: &'a str, reason: &'a str },
    Timeout { seconds: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_variant_serialization() {
        let error = TestError::NotFound;
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(json, r#"{"error":"com.example.Test.NotFound"}"#);
    }

    #[test]
    fn named_variant_serialization() {
        let error = TestError::InvalidInput {
            field: "username",
            reason: "too short",
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains(r#""error":"com.example.Test.InvalidInput""#));
        assert!(json.contains(r#""field":"username""#));
        assert!(json.contains(r#""reason":"too short""#));
    }

    #[test]
    fn round_trip() {
        // Test with error that has parameters
        let original = TestError::InvalidInput {
            field: "password",
            reason: "missing special character",
        };

        round_trip_serialize(&original);

        // Test with error that has no parameters
        let original = TestError::NotFound;
        round_trip_serialize(&original);
    }

    #[test]
    fn field_order_requirements_with_lifetimes() {
        // For enums with lifetimes, we require error before parameters
        let json_parameters_first = r#"{"parameters":{"field":"test","reason":"fail"},"error":"com.example.Test.InvalidInput"}"#;

        // Parameters-first JSON should fail to deserialize
        #[cfg(feature = "std")]
        {
            let result: Result<TestError, _> = serde_json::from_str(json_parameters_first);
            match result {
                Err(e) if e.is_data() => {
                    // Expected - our custom deserializer error becomes a "data" error in serde_json
                    // This confirms the field order validation is working
                }
                Err(_) => panic!("Expected a data error for field order violation"),
                Ok(_) => panic!("Expected deserialization to fail for parameters-first JSON"),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let result: Result<(TestError, usize), _> =
                serde_json_core::from_str(json_parameters_first);
            match result {
                Err(_) => {
                    // Expected - deserialization fails due to field order requirement
                    // serde_json_core error types are different from serde_json
                }
                Ok(_) => panic!("Expected deserialization to fail for parameters-first JSON"),
            }
        }

        // But error before parameters works fine
        let json_error_first = r#"{"error":"com.example.Test.InvalidInput","parameters":{"field":"test","reason":"fail"}}"#;

        #[cfg(feature = "std")]
        {
            let result: Result<TestError, _> = serde_json::from_str(json_error_first);
            assert!(result.is_ok());
            assert_eq!(
                result.unwrap(),
                TestError::InvalidInput {
                    field: "test",
                    reason: "fail"
                }
            );
        }
        #[cfg(not(feature = "std"))]
        {
            let (result, _): (TestError, usize) =
                serde_json_core::from_str(json_error_first).unwrap();
            assert_eq!(
                result,
                TestError::InvalidInput {
                    field: "test",
                    reason: "fail"
                }
            );
        }
    }

    #[derive(ReplyError, Debug, PartialEq)]
    #[zlink(interface = "com.example.Owned")]
    enum OwnedError {
        NotFound,
        InvalidInput { field: String, reason: String },
    }

    #[test]
    fn field_order_requirements_without_lifetimes() {
        // For enums without lifetimes, we now also require error field first for simplicity
        let json_parameters_first = r#"{"parameters":{"field":"test","reason":"fail"},"error":"com.example.Owned.InvalidInput"}"#;

        // Parameters-first JSON should fail to deserialize
        #[cfg(feature = "std")]
        {
            let result: Result<OwnedError, _> = serde_json::from_str(json_parameters_first);
            match result {
                Err(e) if e.is_data() => {
                    // Expected - our custom deserializer error becomes a "data" error in serde_json
                    // This confirms the field order validation is working
                }
                Err(_) => panic!("Expected a data error for field order violation"),
                Ok(_) => panic!("Expected deserialization to fail for parameters-first JSON"),
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let result: Result<(OwnedError, usize), _> =
                serde_json_core::from_str(json_parameters_first);
            match result {
                Err(_) => {
                    // Expected - deserialization fails due to field order requirement
                    // serde_json_core error types are different from serde_json
                }
                Ok(_) => panic!("Expected deserialization to fail for parameters-first JSON"),
            }
        }

        // But error before parameters works fine
        let json_error_first = r#"{"error":"com.example.Owned.InvalidInput","parameters":{"field":"test","reason":"fail"}}"#;

        #[cfg(feature = "std")]
        {
            let result: Result<OwnedError, _> = serde_json::from_str(json_error_first);
            assert!(result.is_ok());
            assert_eq!(
                result.unwrap(),
                OwnedError::InvalidInput {
                    field: "test".to_string(),
                    reason: "fail".to_string()
                }
            );
        }
        #[cfg(not(feature = "std"))]
        {
            let (result, _): (OwnedError, usize) =
                serde_json_core::from_str(json_error_first).unwrap();
            assert_eq!(
                result,
                OwnedError::InvalidInput {
                    field: "test".to_string(),
                    reason: "fail".to_string()
                }
            );
        }
    }

    // Helper function for round-trip serialization test, abstracting std vs nostd differences
    fn round_trip_serialize(original: &TestError) {
        #[cfg(feature = "std")]
        {
            let json = serde_json::to_string(original).unwrap();
            let deserialized: TestError = serde_json::from_str(&json).unwrap();
            assert_eq!(*original, deserialized);
        }
        #[cfg(not(feature = "std"))]
        {
            let mut buffer = [0u8; 512];
            let len = serde_json_core::to_slice(original, &mut buffer).unwrap();
            let json_bytes = &buffer[..len];
            let (deserialized, _): (TestError, usize) =
                serde_json_core::from_slice(json_bytes).unwrap();
            assert_eq!(*original, deserialized);
        }
    }
}
