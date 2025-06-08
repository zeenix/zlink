//! Member definitions for Varlink interfaces.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::{CustomType, Error, Method};

/// A member of a Varlink interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member<'a> {
    /// A custom type definition.
    Custom(CustomType<'a>),
    /// A method definition.
    Method(Method<'a>),
    /// An error definition.
    Error(Error<'a>),
}

impl<'a> Member<'a> {
    /// Returns the name of this member.
    pub fn name(&self) -> &'a str {
        match self {
            Member::Custom(custom) => custom.name,
            Member::Method(method) => method.name,
            Member::Error(error) => error.name,
        }
    }
}

impl<'a> fmt::Display for Member<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Member::Custom(custom) => write!(f, "{}", custom),
            Member::Method(method) => write!(f, "{}", method),
            Member::Error(error) => write!(f, "{}", error),
        }
    }
}

impl<'a> Serialize for Member<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de, 'a> Deserialize<'de> for Member<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_member(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{CustomType, Field, List, Parameter, Type, TypeRef};

    #[test]
    fn test_member_custom_type() {
        let fields = vec![
            Field::new("vendor", Type::String),
            Field::new("product", Type::String),
            Field::new("version", Type::String),
        ];
        let custom = CustomType::new("ServiceInfo", List::from(fields));
        let member = Member::Custom(custom);

        assert_eq!(member.name(), "ServiceInfo");
        assert_eq!(
            member.to_string(),
            "type ServiceInfo (vendor: string, product: string, version: string)"
        );
    }

    #[test]
    fn test_member_method() {
        let outputs = vec![
            Parameter::new("vendor", Type::String),
            Parameter::new("product", Type::String),
            Parameter::new("version", Type::String),
            Parameter::new("url", Type::String),
            Parameter::new("interfaces", Type::Array(TypeRef::new(Type::String))),
        ];
        let method = Method::new("GetInfo", List::default(), List::from(outputs));
        let member = Member::Method(method);

        assert_eq!(member.name(), "GetInfo");
        assert_eq!(
            member.to_string(),
            "method GetInfo() -> (vendor: string, product: string, version: string, url: string, interfaces: []string)"
        );
    }

    #[test]
    fn test_member_error() {
        let fields = vec![Field::new("interface", Type::String)];
        let error = Error::new("InterfaceNotFound", List::from(fields));
        let member = Member::Error(error);

        assert_eq!(member.name(), "InterfaceNotFound");
        assert_eq!(
            member.to_string(),
            "error InterfaceNotFound (interface: string)"
        );
    }

    #[test]
    fn test_member_serialization() {
        let error = Error::new("PermissionDenied", List::default());
        let member = Member::Error(error);

        let json = serde_json::to_string(&member).unwrap();
        assert_eq!(json, r#""error PermissionDenied ()""#);
    }
}
