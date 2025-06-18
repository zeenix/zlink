//! Custom type, field and parameter definitions for Varlink IDL.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{Type, TypeRef};

/// A field in a custom type or method parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<'a> {
    /// The name of the field.
    name: &'a str,
    /// The type of the field.
    ty: TypeRef<'a>,
}

/// Type alias for method parameters, which have the same structure as fields.
pub type Parameter<'a> = Field<'a>;

impl<'a> Field<'a> {
    /// Creates a new field with the given name and type.
    pub const fn new(name: &'a str, ty: &'a Type<'a>) -> Self {
        Self {
            name,
            ty: TypeRef::borrowed(ty),
        }
    }

    /// Same as `new` but takes `ty` by value.
    #[cfg(feature = "std")]
    pub fn new_owned(name: &'a str, ty: Type<'a>) -> Self {
        Self {
            name,
            ty: TypeRef::new(ty),
        }
    }

    /// Returns the name of the field.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the type of the field.
    pub fn ty(&self) -> &Type<'a> {
        self.ty.inner()
    }
}

impl<'a> fmt::Display for Field<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.ty)
    }
}

impl<'a> Serialize for Field<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "idl-parse")]
impl<'de, 'a> Deserialize<'de> for Field<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_field(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{Type, TypeInfo};

    #[test]
    fn field_creation() {
        let field = Field::new("age", <i32>::TYPE_INFO);
        assert_eq!(field.name(), "age");
        assert_eq!(field.ty(), &Type::Int);
    }

    #[test]
    fn field_serialization() {
        let field = Field::new("count", <i32>::TYPE_INFO);
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&field).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 16];
            let len = serde_json_core::to_slice(&field, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 16>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<16>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""count: int""#);
    }

    #[test]
    fn parameter_alias() {
        let param: Parameter<'_> = Field::new("input", <&str>::TYPE_INFO);
        assert_eq!(param.name(), "input");
        assert_eq!(param.ty(), &Type::String);
    }
}
