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
        let interfaces_type = Type::Array(TypeRef::new(&Type::String));
        let get_info_outputs = [
            &Parameter::new("vendor", <&str>::TYPE_INFO),
            &Parameter::new("product", <&str>::TYPE_INFO),
            &Parameter::new("version", <&str>::TYPE_INFO),
            &Parameter::new("url", <&str>::TYPE_INFO),
            &Parameter::new("interfaces", &interfaces_type),
        ];
        let get_info = Method::new("GetInfo", &[], &get_info_outputs);

        let get_interface_desc_inputs = [&Parameter::new("interface", <&str>::TYPE_INFO)];
        let get_interface_desc_outputs = [&Parameter::new("description", <&str>::TYPE_INFO)];
        let get_interface_desc = Method::new(
            "GetInterfaceDescription",
            &get_interface_desc_inputs,
            &get_interface_desc_outputs,
        );

        let interface_not_found_fields = [&Field::new("interface", <&str>::TYPE_INFO)];
        let interface_not_found = Error::new("InterfaceNotFound", &interface_not_found_fields);

        let method_not_found_fields = [&Field::new("method", <&str>::TYPE_INFO)];
        let method_not_found = Error::new("MethodNotFound", &method_not_found_fields);

        let method_not_impl_fields = [&Field::new("method", <&str>::TYPE_INFO)];
        let method_not_impl = Error::new("MethodNotImplemented", &method_not_impl_fields);

        let invalid_param_fields = [&Field::new("parameter", <&str>::TYPE_INFO)];
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

    #[cfg(feature = "idl-parse")]
    #[test]
    fn systemd_resolved_interface_parsing() {
        use crate::idl::{parse, CustomType, TypeRef};

        // Manually construct the systemd-resolved interface for comparison.

        // Define types used in the interface.
        let optional_int_type = Type::Optional(TypeRef::new(&Type::Int));
        let int_array_type = Type::Array(TypeRef::new(&Type::Int));
        let optional_string_type = Type::Optional(TypeRef::new(&Type::String));
        let optional_int_array_type = Type::Optional(TypeRef::new(&int_array_type));

        // Build ResolvedAddress custom type.
        let resolved_address_fields = [
            &Field::new("ifindex", &optional_int_type),
            &Field::new("family", <i64>::TYPE_INFO),
            &Field::new("address", &int_array_type),
        ];
        let resolved_address = CustomType::new("ResolvedAddress", &resolved_address_fields);

        // Build ResolvedName custom type.
        let resolved_name_fields = [
            &Field::new("ifindex", &optional_int_type),
            &Field::new("name", <&str>::TYPE_INFO),
        ];
        let resolved_name = CustomType::new("ResolvedName", &resolved_name_fields);

        // Build ResourceKey custom type.
        let resource_key_fields = [
            &Field::new("class", <i64>::TYPE_INFO),
            &Field::new("type", <i64>::TYPE_INFO),
            &Field::new("name", <&str>::TYPE_INFO),
        ];
        let resource_key = CustomType::new("ResourceKey", &resource_key_fields);

        // Build ResourceRecord custom type (references ResourceKey).
        let resource_key_type = Type::Custom("ResourceKey");
        let resource_record_fields = [
            &Field::new("key", &resource_key_type),
            &Field::new("priority", &optional_int_type),
            &Field::new("weight", &optional_int_type),
            &Field::new("port", &optional_int_type),
            &Field::new("name", &optional_string_type),
            &Field::new("address", &optional_int_array_type),
        ];
        let resource_record = CustomType::new("ResourceRecord", &resource_record_fields);

        // Build methods.
        let resolved_address_array_type =
            Type::Array(TypeRef::new(&Type::Custom("ResolvedAddress")));
        let resolved_name_array_type = Type::Array(TypeRef::new(&Type::Custom("ResolvedName")));

        let resolve_hostname_inputs = [
            &Parameter::new("ifindex", &optional_int_type),
            &Parameter::new("name", <&str>::TYPE_INFO),
            &Parameter::new("family", &optional_int_type),
            &Parameter::new("flags", &optional_int_type),
        ];
        let resolve_hostname_outputs = [
            &Parameter::new("addresses", &resolved_address_array_type),
            &Parameter::new("name", <&str>::TYPE_INFO),
            &Parameter::new("flags", <i64>::TYPE_INFO),
        ];
        let resolve_hostname = Method::new(
            "ResolveHostname",
            &resolve_hostname_inputs,
            &resolve_hostname_outputs,
        );

        let resolve_address_inputs = [
            &Parameter::new("ifindex", &optional_int_type),
            &Parameter::new("family", <i64>::TYPE_INFO),
            &Parameter::new("address", &int_array_type),
            &Parameter::new("flags", &optional_int_type),
        ];
        let resolve_address_outputs = [
            &Parameter::new("names", &resolved_name_array_type),
            &Parameter::new("flags", <i64>::TYPE_INFO),
        ];
        let resolve_address = Method::new(
            "ResolveAddress",
            &resolve_address_inputs,
            &resolve_address_outputs,
        );

        // Build errors.
        let no_name_servers = Error::new("NoNameServers", &[]);
        let query_timed_out = Error::new("QueryTimedOut", &[]);

        let dnssec_validation_failed_fields = [
            &Field::new("result", <&str>::TYPE_INFO),
            &Field::new("extendedDNSErrorCode", &optional_int_type),
            &Field::new("extendedDNSErrorMessage", &optional_string_type),
        ];
        let dnssec_validation_failed =
            Error::new("DNSSECValidationFailed", &dnssec_validation_failed_fields);

        let dns_error_fields = [
            &Field::new("rcode", <i64>::TYPE_INFO),
            &Field::new("extendedDNSErrorCode", &optional_int_type),
            &Field::new("extendedDNSErrorMessage", &optional_string_type),
        ];
        let dns_error = Error::new("DNSError", &dns_error_fields);

        // Build the complete interface.
        let members = &[
            &Member::Custom(resolved_address),
            &Member::Custom(resolved_name),
            &Member::Custom(resource_key),
            &Member::Custom(resource_record),
            &Member::Method(resolve_hostname),
            &Member::Method(resolve_address),
            &Member::Error(no_name_servers),
            &Member::Error(query_timed_out),
            &Member::Error(dnssec_validation_failed),
            &Member::Error(dns_error),
        ];

        let interface = Interface::new("io.systemd.Resolve", members);

        // Test parsing the IDL and compare with manually constructed interface.
        const SYSTEMD_RESOLVED_IDL: &str = r#"interface io.systemd.Resolve

type ResolvedAddress(
    ifindex: ?int,
    family: int,
    address: []int
)

type ResolvedName(
    ifindex: ?int,
    name: string
)

type ResourceKey(
    class: int,
    type: int,
    name: string
)

type ResourceRecord(
    key: ResourceKey,
    priority: ?int,
    weight: ?int,
    port: ?int,
    name: ?string,
    address: ?[]int
)

method ResolveHostname(
    ifindex: ?int,
    name: string,
    family: ?int,
    flags: ?int
) -> (
    addresses: []ResolvedAddress,
    name: string,
    flags: int
)

method ResolveAddress(
    ifindex: ?int,
    family: int,
    address: []int,
    flags: ?int
) -> (
    names: []ResolvedName,
    flags: int
)

error NoNameServers()

error QueryTimedOut()

error DNSSECValidationFailed(
    result: string,
    extendedDNSErrorCode: ?int,
    extendedDNSErrorMessage: ?string
)

error DNSError(
    rcode: int,
    extendedDNSErrorCode: ?int,
    extendedDNSErrorMessage: ?string
)
"#;

        let parsed_interface = parse::parse_interface(SYSTEMD_RESOLVED_IDL)
            .expect("Failed to parse systemd-resolved interface");

        // Verify basic interface properties match.
        assert_eq!(parsed_interface.name(), interface.name());
        assert_eq!(
            parsed_interface.members().count(),
            interface.members().count()
        );
        assert_eq!(
            parsed_interface.custom_types().count(),
            interface.custom_types().count()
        );
        assert_eq!(
            parsed_interface.methods().count(),
            interface.methods().count()
        );
        assert_eq!(
            parsed_interface.errors().count(),
            interface.errors().count()
        );

        // Check specific type validation - ResolvedAddress.
        let parsed_resolved_address = parsed_interface
            .custom_types()
            .find(|t| t.name() == "ResolvedAddress")
            .expect("ResolvedAddress type should exist");
        let manual_resolved_address = interface
            .custom_types()
            .find(|t| t.name() == "ResolvedAddress")
            .expect("ResolvedAddress type should exist in manual interface");

        // Verify field types in ResolvedAddress.
        let parsed_fields: Vec<_> = parsed_resolved_address.fields().collect();
        let manual_fields: Vec<_> = manual_resolved_address.fields().collect();
        assert_eq!(parsed_fields.len(), manual_fields.len());

        assert_eq!(parsed_fields[0].name(), "ifindex");
        assert_eq!(
            *parsed_fields[0].ty(),
            Type::Optional(TypeRef::new(&Type::Int))
        );
        assert_eq!(parsed_fields[1].name(), "family");
        assert_eq!(*parsed_fields[1].ty(), Type::Int);
        assert_eq!(parsed_fields[2].name(), "address");
        assert_eq!(
            *parsed_fields[2].ty(),
            Type::Array(TypeRef::new(&Type::Int))
        );

        // Check method parameter types - ResolveHostname.
        let parsed_resolve_hostname = parsed_interface
            .methods()
            .find(|m| m.name() == "ResolveHostname")
            .expect("ResolveHostname method should exist");
        let manual_resolve_hostname = interface
            .methods()
            .find(|m| m.name() == "ResolveHostname")
            .expect("ResolveHostname method should exist in manual interface");

        let parsed_inputs: Vec<_> = parsed_resolve_hostname.inputs().collect();
        let manual_inputs: Vec<_> = manual_resolve_hostname.inputs().collect();
        assert_eq!(parsed_inputs.len(), manual_inputs.len());

        // Verify input parameter types.
        assert_eq!(parsed_inputs[0].name(), "ifindex");
        assert_eq!(
            *parsed_inputs[0].ty(),
            Type::Optional(TypeRef::new(&Type::Int))
        );
        assert_eq!(parsed_inputs[1].name(), "name");
        assert_eq!(*parsed_inputs[1].ty(), Type::String);
        assert_eq!(parsed_inputs[2].name(), "family");
        assert_eq!(
            *parsed_inputs[2].ty(),
            Type::Optional(TypeRef::new(&Type::Int))
        );

        // Verify output parameter types.
        let parsed_outputs: Vec<_> = parsed_resolve_hostname.outputs().collect();
        assert_eq!(parsed_outputs[0].name(), "addresses");
        assert_eq!(
            *parsed_outputs[0].ty(),
            Type::Array(TypeRef::new(&Type::Custom("ResolvedAddress")))
        );
        assert_eq!(parsed_outputs[1].name(), "name");
        assert_eq!(*parsed_outputs[1].ty(), Type::String);
        assert_eq!(parsed_outputs[2].name(), "flags");
        assert_eq!(*parsed_outputs[2].ty(), Type::Int);

        // Check error field types - DNSError.
        let parsed_dns_error = parsed_interface
            .errors()
            .find(|e| e.name() == "DNSError")
            .expect("DNSError should exist");
        let dns_error_fields: Vec<_> = parsed_dns_error.fields().collect();

        assert_eq!(dns_error_fields[0].name(), "rcode");
        assert_eq!(*dns_error_fields[0].ty(), Type::Int);
        assert_eq!(dns_error_fields[1].name(), "extendedDNSErrorCode");
        assert_eq!(
            *dns_error_fields[1].ty(),
            Type::Optional(TypeRef::new(&Type::Int))
        );
        assert_eq!(dns_error_fields[2].name(), "extendedDNSErrorMessage");
        assert_eq!(
            *dns_error_fields[2].ty(),
            Type::Optional(TypeRef::new(&Type::String))
        );

        // Verify no-field errors work.
        let parsed_no_name_servers = parsed_interface
            .errors()
            .find(|e| e.name() == "NoNameServers")
            .expect("NoNameServers should exist");
        assert_eq!(parsed_no_name_servers.fields().count(), 0);

        // Compare the parsed interface with our manually constructed one.
        assert_eq!(parsed_interface, interface);
    }
}
