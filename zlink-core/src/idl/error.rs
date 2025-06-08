//! Error definitions for Varlink IDL.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::{Field, List};

/// An error definition in Varlink IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error<'a> {
    /// The name of the error.
    pub name: &'a str,
    /// The fields of the error.
    pub fields: List<'a, Field<'a>>,
}

impl<'a> Error<'a> {
    /// Creates a new error with the given name and fields.
    pub fn new(name: &'a str, fields: List<'a, Field<'a>>) -> Self {
        Self { name, fields }
    }

    /// Returns true if the error has no fields.
    pub fn has_no_fields(&self) -> bool {
        self.fields.is_empty()
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error {} (", self.name)?;
        let mut first = true;
        for field in self.fields.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{}", field)?;
        }
        write!(f, ")")
    }
}

impl<'a> Serialize for Error<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de, 'a> Deserialize<'de> for Error<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_error(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::Type;

    #[test]
    fn test_error_creation() {
        const MESSAGE_FIELD: Field<'_> = Field {
            name: "message",
            ty: Type::String,
        };
        const CODE_FIELD: Field<'_> = Field {
            name: "code",
            ty: Type::Int,
        };
        const FIELDS: [&Field<'_>; 2] = [&MESSAGE_FIELD, &CODE_FIELD];

        let error = Error::new("InvalidInput", List::Borrowed(&FIELDS));
        assert_eq!(error.name, "InvalidInput");
        assert_eq!(error.fields.len(), 2);
        assert!(!error.has_no_fields());
    }

    #[test]
    fn test_error_no_fields() {
        let error = Error::new("UnknownError", List::default());
        assert_eq!(error.name, "UnknownError");
        assert!(error.has_no_fields());
    }

    #[test]
    fn test_error_serialization() {
        let fields = vec![
            Field::new("message", Type::String),
            Field::new("details", Type::Object),
        ];

        let error = Error::new("ValidationError", List::from(fields));
        let json = serde_json::to_string(&error).unwrap();

        assert_eq!(
            json,
            r#""error ValidationError (message: string, details: object)""#
        );
    }
}
