//! Member definitions for Varlink interfaces.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

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
            Member::Custom(custom) => custom.name(),
            Member::Method(method) => method.name(),
            Member::Error(error) => error.name(),
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

#[cfg(feature = "idl-parse")]
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
    use crate::idl::{CustomObject, Error, Field, Method, Parameter, Type};

    #[test]
    fn member_custom_type() {
        let vendor_field = Field::new("vendor", &Type::String, &[]);
        let product_field = Field::new("product", &Type::String, &[]);
        let version_field = Field::new("version", &Type::String, &[]);
        let fields = [&vendor_field, &product_field, &version_field];

        let custom = CustomType::from(CustomObject::new("ServiceInfo", &fields, &[]));
        let member = Member::Custom(custom);

        assert_eq!(member.name(), "ServiceInfo");

        use core::fmt::Write;
        let mut buf = mayheap::String::<128>::new();
        write!(buf, "{}", member).unwrap();
        assert_eq!(
            buf.as_str(),
            "type ServiceInfo (vendor: string, product: string, version: string)"
        );
    }

    #[test]
    fn member_method() {
        #[cfg(feature = "std")]
        {
            use crate::idl::TypeRef;
            let interfaces_type = Type::Array(TypeRef::new(&Type::String));
            let outputs = vec![
                Parameter::new("vendor", &Type::String, &[]),
                Parameter::new("product", &Type::String, &[]),
                Parameter::new("version", &Type::String, &[]),
                Parameter::new("url", &Type::String, &[]),
                Parameter::new("interfaces", &interfaces_type, &[]),
            ];
            let method = Method::new_owned("GetInfo", vec![], outputs, vec![]);
            let member = Member::Method(method);

            assert_eq!(member.name(), "GetInfo");
            use core::fmt::Write;
            let mut buf = mayheap::String::<256>::new();
            write!(buf, "{}", member).unwrap();
            assert_eq!(
                buf.as_str(),
                "method GetInfo() -> (vendor: string, product: string, version: string, url: string, interfaces: []string)"
            );
        }

        #[cfg(not(feature = "std"))]
        {
            let vendor_param = Parameter::new("vendor", &Type::String, &[]);
            let product_param = Parameter::new("product", &Type::String, &[]);
            let version_param = Parameter::new("version", &Type::String, &[]);
            let url_param = Parameter::new("url", &Type::String, &[]);
            let outputs = [&vendor_param, &product_param, &version_param, &url_param];

            let method = Method::new("GetInfo", &[], &outputs, &[]);
            let member = Member::Method(method);

            assert_eq!(member.name(), "GetInfo");
            use core::fmt::Write;
            let mut buf = mayheap::String::<256>::new();
            write!(buf, "{}", member).unwrap();
            assert_eq!(
                buf.as_str(),
                "method GetInfo() -> (vendor: string, product: string, version: string, url: string)"
            );
        }
    }

    #[test]
    fn member_error() {
        let interface_field = Field::new("interface", &Type::String, &[]);
        let fields = [&interface_field];

        let error = Error::new("InterfaceNotFound", &fields, &[]);
        let member = Member::Error(error);

        assert_eq!(member.name(), "InterfaceNotFound");

        use core::fmt::Write;
        let mut buf = mayheap::String::<64>::new();
        write!(buf, "{}", member).unwrap();
        assert_eq!(buf.as_str(), "error InterfaceNotFound (interface: string)");
    }

    #[test]
    fn member_serialization() {
        let error = Error::new("PermissionDenied", &[], &[]);
        let member = Member::Error(error);
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&member).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 64];
            let len = serde_json_core::to_slice(&member, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 64>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<64>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, r#""error PermissionDenied ()""#);
    }
}
