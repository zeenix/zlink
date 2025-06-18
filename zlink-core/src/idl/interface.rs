//! Interface definitions for Varlink IDL.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{List, Member};

/// A Varlink interface definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface<'a> {
    /// The name of the interface in reverse-domain notation.
    name: &'a str,
    /// The members of the interface (types, methods, errors).
    members: List<'a, Member<'a>>,
}

impl<'a> Interface<'a> {
    /// Creates a new interface with the given name and borrowed members.
    pub const fn new(name: &'a str, members: &'a [&'a Member<'a>]) -> Self {
        Self {
            name,
            members: List::Borrowed(members),
        }
    }

    /// Creates a new interface with the given name and owned members.
    #[cfg(feature = "std")]
    pub fn new_owned(name: &'a str, members: Vec<Member<'a>>) -> Self {
        Self {
            name,
            members: List::Owned(members),
        }
    }

    /// Returns the name of the interface.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns an iterator over the members of the interface.
    pub fn members(&self) -> impl Iterator<Item = &Member<'a>> {
        self.members.iter()
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

#[cfg(feature = "idl-parse")]
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
    use crate::idl::{Error, Field, Method, Parameter, Type, TypeInfo};

    #[test]
    fn org_varlink_service_interface() {
        use crate::idl::TypeRef;

        // Build the org.varlink.service interface as our test case
        let vendor_param = Parameter::new("vendor", <&str>::TYPE_INFO);
        let product_param = Parameter::new("product", <&str>::TYPE_INFO);
        let version_param = Parameter::new("version", <&str>::TYPE_INFO);
        let url_param = Parameter::new("url", <&str>::TYPE_INFO);
        let interfaces_type = Type::Array(TypeRef::borrowed(&Type::String));
        let interfaces_param = Parameter::new("interfaces", &interfaces_type);
        let get_info_outputs = [
            &vendor_param,
            &product_param,
            &version_param,
            &url_param,
            &interfaces_param,
        ];

        let get_interface_desc_input = Parameter::new("interface", <&str>::TYPE_INFO);
        let get_interface_desc_inputs = [&get_interface_desc_input];

        let get_interface_desc_output = Parameter::new("description", <&str>::TYPE_INFO);
        let get_interface_desc_outputs = [&get_interface_desc_output];

        let interface_not_found_field = Field::new("interface", <&str>::TYPE_INFO);
        let interface_not_found_fields = [&interface_not_found_field];

        let method_not_found_field = Field::new("method", <&str>::TYPE_INFO);
        let method_not_found_fields = [&method_not_found_field];

        let method_not_impl_field = Field::new("method", <&str>::TYPE_INFO);
        let method_not_impl_fields = [&method_not_impl_field];

        let invalid_param_field = Field::new("parameter", <&str>::TYPE_INFO);
        let invalid_param_fields = [&invalid_param_field];

        let get_info = Method::new("GetInfo", &[], &get_info_outputs);
        let get_interface_desc = Method::new(
            "GetInterfaceDescription",
            &get_interface_desc_inputs,
            &get_interface_desc_outputs,
        );
        let interface_not_found = Error::new("InterfaceNotFound", &interface_not_found_fields);
        let method_not_found = Error::new("MethodNotFound", &method_not_found_fields);
        let method_not_impl = Error::new("MethodNotImplemented", &method_not_impl_fields);
        let invalid_param = Error::new("InvalidParameter", &invalid_param_fields);
        let permission_denied = Error::new("PermissionDenied", &[]);
        let expected_more = Error::new("ExpectedMore", &[]);

        let members = &[
            &Member::Method(get_info),
            &Member::Method(get_interface_desc),
            &Member::Error(interface_not_found),
            &Member::Error(method_not_found),
            &Member::Error(method_not_impl),
            &Member::Error(invalid_param),
            &Member::Error(permission_denied),
            &Member::Error(expected_more),
        ];

        let interface = Interface::new("org.varlink.service", members);

        assert_eq!(interface.name(), "org.varlink.service");
        assert_eq!(interface.members().count(), 8);
        assert!(!interface.is_empty());

        // Check method count
        assert_eq!(interface.methods().count(), 2);

        // Check error count
        assert_eq!(interface.errors().count(), 6);

        // Test Display output
        use core::fmt::Write;
        let mut idl = mayheap::String::<2048>::new();
        write!(idl, "{}", interface).unwrap();
        assert!(idl.as_str().starts_with("interface org.varlink.service"));
        assert!(idl.as_str().contains("method GetInfo()"));
        assert!(idl
            .as_str()
            .contains("method GetInterfaceDescription(interface: string)"));
        assert!(idl
            .as_str()
            .contains("error InterfaceNotFound (interface: string)"));
        assert!(idl.as_str().contains("error PermissionDenied ()"));

        // Test parsing the official org.varlink.service IDL and compare with manually constructed
        #[cfg(feature = "idl-parse")]
        {
            use crate::idl::parse;

            const ORG_VARLINK_SERVICE_IDL: &str = r#"interface org.varlink.service

method GetInfo() -> (
  vendor: string,
  product: string,
  version: string,
  url: string,
  interfaces: []string
)

method GetInterfaceDescription(interface: string) -> (description: string)

error InterfaceNotFound (interface: string)

error MethodNotFound (method: string)

error MethodNotImplemented (method: string)

error InvalidParameter (parameter: string)

error PermissionDenied ()

error ExpectedMore ()
"#;

            let parsed_interface = parse::parse_interface(ORG_VARLINK_SERVICE_IDL)
                .expect("Failed to parse org.varlink.service IDL");

            // Compare the parsed interface with our manually constructed one
            assert_eq!(parsed_interface, interface);
        }
    }

    #[test]
    fn interface_serialization() {
        let simple_method = Method::new("Ping", &[], &[]);

        let members = [&Member::Method(simple_method)];
        let interface = Interface::new("com.example.ping", &members);
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&interface).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 64];
            let len = serde_json_core::to_slice(&interface, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 64>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<64>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""interface com.example.ping\n\nmethod Ping()""#);
    }

    #[test]
    fn empty_interface() {
        let interface = Interface::new("com.example.empty", &[]);
        assert!(interface.is_empty());
        assert_eq!(interface.methods().count(), 0);
        assert_eq!(interface.errors().count(), 0);
        assert_eq!(interface.custom_types().count(), 0);
    }
}
