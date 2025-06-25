//! Error definitions for Varlink IDL.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{Field, List};

/// An error definition in Varlink IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error<'a> {
    /// The name of the error.
    name: &'a str,
    /// The fields of the error.
    fields: List<'a, Field<'a>>,
}

impl<'a> Error<'a> {
    /// Creates a new error with the given name and borrowed fields.
    pub const fn new(name: &'a str, fields: &'a [&'a Field<'a>]) -> Self {
        Self {
            name,
            fields: List::Borrowed(fields),
        }
    }

    /// Creates a new error with the given name and owned fields.
    #[cfg(feature = "std")]
    pub fn new_owned(name: &'a str, fields: Vec<Field<'a>>) -> Self {
        Self {
            name,
            fields: List::Owned(fields),
        }
    }

    /// Returns the name of the error.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns an iterator over the fields of the error.
    pub fn fields(&self) -> impl Iterator<Item = &Field<'a>> {
        self.fields.iter()
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

#[cfg(feature = "idl-parse")]
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
    use crate::idl::{Type, TypeInfo};

    #[test]
    fn error_creation() {
        let message_field = Field::new("message", <&str>::TYPE_INFO);
        let code_field = Field::new("code", <i32>::TYPE_INFO);
        let fields = [&message_field, &code_field];

        let error = Error::new("InvalidInput", &fields);
        assert_eq!(error.name(), "InvalidInput");
        assert_eq!(error.fields().count(), 2);
        assert!(!error.has_no_fields());

        // Check the fields individually - order and values.
        let fields_vec: mayheap::Vec<_, 8> = error.fields().collect();
        assert_eq!(fields_vec[0].name(), "message");
        assert_eq!(fields_vec[0].ty(), &Type::String);
        assert_eq!(fields_vec[1].name(), "code");
        assert_eq!(fields_vec[1].ty(), &Type::Int);
    }

    #[test]
    fn error_no_fields() {
        let error = Error::new("UnknownError", &[]);
        assert_eq!(error.name(), "UnknownError");
        assert!(error.has_no_fields());
    }

    #[test]
    fn error_serialization() {
        let message_field = Field::new("message", <&str>::TYPE_INFO);
        let details_field = Field::new("details", &Type::ForeignObject);
        let fields = [&message_field, &details_field];

        let error = Error::new("ValidationError", &fields);

        // Check the fields individually - order and values.
        let fields_vec: mayheap::Vec<_, 8> = error.fields().collect();
        assert_eq!(fields_vec[0].name(), "message");
        assert_eq!(fields_vec[0].ty(), &Type::String);
        assert_eq!(fields_vec[1].name(), "details");
        assert_eq!(fields_vec[1].ty(), &Type::ForeignObject);

        #[cfg(feature = "std")]
        let json = serde_json::to_string(&error).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 128];
            let len = serde_json_core::to_slice(&error, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 128>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<128>::from_utf8(vec).unwrap()
        };
        assert_eq!(
            json,
            r#""error ValidationError (message: string, details: object)""#
        );
    }
}
