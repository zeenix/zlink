//! Type definition for Varlink IDL custom types.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{Enum, Object};

/// A custom type definition in Varlink IDL.
///
/// This can be either a struct-like object type with named fields,
/// or an enum-like type with named variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'a> {
    /// A struct-like custom type with named fields.
    Object(Object<'a>),
    /// An enum-like custom type with named variants.
    Enum(Enum<'a>),
}

impl<'a> Type<'a> {
    /// Returns the name of the custom type.
    pub fn name(&self) -> &'a str {
        match self {
            Type::Object(obj) => obj.name(),
            Type::Enum(enm) => enm.name(),
        }
    }

    /// Returns true if this is an object custom type.
    pub fn is_object(&self) -> bool {
        matches!(self, Type::Object(_))
    }

    /// Returns true if this is an enum custom type.
    pub fn is_enum(&self) -> bool {
        matches!(self, Type::Enum(_))
    }

    /// Returns the object if this is an object custom type.
    pub fn as_object(&self) -> Option<&Object<'a>> {
        match self {
            Type::Object(obj) => Some(obj),
            Type::Enum(_) => None,
        }
    }

    /// Returns the enum if this is an enum custom type.
    pub fn as_enum(&self) -> Option<&Enum<'a>> {
        match self {
            Type::Object(_) => None,
            Type::Enum(enm) => Some(enm),
        }
    }
}

impl<'a> From<Object<'a>> for Type<'a> {
    fn from(obj: Object<'a>) -> Self {
        Type::Object(obj)
    }
}

impl<'a> From<Enum<'a>> for Type<'a> {
    fn from(enm: Enum<'a>) -> Self {
        Type::Enum(enm)
    }
}

impl<'a> fmt::Display for Type<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Object(obj) => write!(f, "{}", obj),
            Type::Enum(enm) => write!(f, "{}", enm),
        }
    }
}

impl<'a> Serialize for Type<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "idl-parse")]
impl<'de, 'a> Deserialize<'de> for Type<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::super::parse::parse_custom_type(s).map_err(serde::de::Error::custom)
    }
}
