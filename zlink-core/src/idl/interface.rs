//! Interface definitions for Varlink IDL.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::{List, Member};

/// A Varlink interface definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface<'a> {
    /// The name of the interface in reverse-domain notation.
    pub name: &'a str,
    /// The members of the interface (types, methods, errors).
    pub members: List<'a, Member<'a>>,
}

impl<'a> Interface<'a> {
    /// Creates a new interface with the given name and members.
    pub fn new(name: &'a str, members: List<'a, Member<'a>>) -> Self {
        Self { name, members }
    }

    /// Returns true if the interface has no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns an iterator over the methods in this interface.
    pub fn methods(&self) -> impl Iterator<Item = &super::Method<'a>> {
        self.members.iter().filter_map(|member| match member {
            Member::Method(method) => Some(method),
            _ => None,
        })
    }

    /// Returns an iterator over the custom types in this interface.
    pub fn custom_types(&self) -> impl Iterator<Item = &super::CustomType<'a>> {
        self.members.iter().filter_map(|member| match member {
            Member::Custom(custom) => Some(custom),
            _ => None,
        })
    }

    /// Returns an iterator over the errors in this interface.
    pub fn errors(&self) -> impl Iterator<Item = &super::Error<'a>> {
        self.members.iter().filter_map(|member| match member {
            Member::Error(error) => Some(error),
            _ => None,
        })
    }
}

impl<'a> fmt::Display for Interface<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "interface {}", self.name)?;
        for member in self.members.iter() {
            write!(f, "\n\n{}", member)?;
        }
        Ok(())
    }
}

impl<'a> Serialize for Interface<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de, 'a> Deserialize<'de> for Interface<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_interface(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{Error, Field, Method, Parameter, Type, TypeRef};

    #[test]
    fn test_org_varlink_service_interface() {
        // Build the org.varlink.service interface as our test case
        // Can't use Box::new in const context, so we'll create parameters at runtime
        let get_info_outputs = vec![
            Parameter {
                name: "vendor",
                ty: Type::String,
            },
            Parameter {
                name: "product",
                ty: Type::String,
            },
            Parameter {
                name: "version",
                ty: Type::String,
            },
            Parameter {
                name: "url",
                ty: Type::String,
            },
            Parameter {
                name: "interfaces",
                ty: Type::Array(TypeRef::new(Type::String)),
            },
        ];

        const GET_INTERFACE_DESC_INPUT: Parameter<'_> = Parameter {
            name: "interface",
            ty: Type::String,
        };
        const GET_INTERFACE_DESC_INPUTS: [&Parameter<'_>; 1] = [&GET_INTERFACE_DESC_INPUT];

        const GET_INTERFACE_DESC_OUTPUT: Parameter<'_> = Parameter {
            name: "description",
            ty: Type::String,
        };
        const GET_INTERFACE_DESC_OUTPUTS: [&Parameter<'_>; 1] = [&GET_INTERFACE_DESC_OUTPUT];

        const INTERFACE_NOT_FOUND_FIELD: Field<'_> = Field {
            name: "interface",
            ty: Type::String,
        };
        const INTERFACE_NOT_FOUND_FIELDS: [&Field<'_>; 1] = [&INTERFACE_NOT_FOUND_FIELD];

        const METHOD_NOT_FOUND_FIELD: Field<'_> = Field {
            name: "method",
            ty: Type::String,
        };
        const METHOD_NOT_FOUND_FIELDS: [&Field<'_>; 1] = [&METHOD_NOT_FOUND_FIELD];

        const METHOD_NOT_IMPL_FIELD: Field<'_> = Field {
            name: "method",
            ty: Type::String,
        };
        const METHOD_NOT_IMPL_FIELDS: [&Field<'_>; 1] = [&METHOD_NOT_IMPL_FIELD];

        const INVALID_PARAM_FIELD: Field<'_> = Field {
            name: "parameter",
            ty: Type::String,
        };
        const INVALID_PARAM_FIELDS: [&Field<'_>; 1] = [&INVALID_PARAM_FIELD];

        let get_info = Method::new("GetInfo", List::default(), List::from(get_info_outputs));
        let get_interface_desc = Method::new(
            "GetInterfaceDescription",
            List::Borrowed(&GET_INTERFACE_DESC_INPUTS),
            List::Borrowed(&GET_INTERFACE_DESC_OUTPUTS),
        );
        let interface_not_found = Error::new(
            "InterfaceNotFound",
            List::Borrowed(&INTERFACE_NOT_FOUND_FIELDS),
        );
        let method_not_found =
            Error::new("MethodNotFound", List::Borrowed(&METHOD_NOT_FOUND_FIELDS));
        let method_not_impl = Error::new(
            "MethodNotImplemented",
            List::Borrowed(&METHOD_NOT_IMPL_FIELDS),
        );
        let invalid_param = Error::new("InvalidParameter", List::Borrowed(&INVALID_PARAM_FIELDS));
        let permission_denied = Error::new("PermissionDenied", List::default());
        let expected_more = Error::new("ExpectedMore", List::default());

        let members = vec![
            Member::Method(get_info),
            Member::Method(get_interface_desc),
            Member::Error(interface_not_found),
            Member::Error(method_not_found),
            Member::Error(method_not_impl),
            Member::Error(invalid_param),
            Member::Error(permission_denied),
            Member::Error(expected_more),
        ];

        let interface = Interface::new("org.varlink.service", List::from(members));

        assert_eq!(interface.name, "org.varlink.service");
        assert_eq!(interface.members.len(), 8);
        assert!(!interface.is_empty());

        // Check method count
        assert_eq!(interface.methods().count(), 2);

        // Check error count
        assert_eq!(interface.errors().count(), 6);

        // Test Display output
        let idl = interface.to_string();
        assert!(idl.starts_with("interface org.varlink.service"));
        assert!(idl.contains("method GetInfo()"));
        assert!(idl.contains("method GetInterfaceDescription(interface: string)"));
        assert!(idl.contains("error InterfaceNotFound (interface: string)"));
        assert!(idl.contains("error PermissionDenied ()"));
    }

    #[test]
    fn test_interface_serialization() {
        const SIMPLE_METHOD: Method<'_> = Method {
            name: "Ping",
            inputs: List::Borrowed(&[]),
            outputs: List::Borrowed(&[]),
        };

        let members = vec![Member::Method(SIMPLE_METHOD)];
        let interface = Interface::new("com.example.ping", List::from(members));

        let json = serde_json::to_string(&interface).unwrap();
        assert_eq!(json, r#""interface com.example.ping\n\nmethod Ping()""#);
    }

    #[test]
    fn test_empty_interface() {
        let interface = Interface::new("com.example.empty", List::default());
        assert!(interface.is_empty());
        assert_eq!(interface.methods().count(), 0);
        assert_eq!(interface.errors().count(), 0);
        assert_eq!(interface.custom_types().count(), 0);
    }
}
