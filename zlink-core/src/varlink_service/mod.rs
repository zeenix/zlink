//! Types for the `org.varlink.service` interface.
//!
//! This module provides types for methods and errors to be used for both client and server
//! implementations of the standard Varlink service interface.

mod info;
pub use info::Info;
mod api;
pub use api::{Error, Method, Reply, Result};

#[cfg(feature = "idl-parse")]
mod proxy;
#[cfg(feature = "idl-parse")]
pub use proxy::{Chain, Proxy};

#[cfg(feature = "idl")]
mod interface_description;
#[cfg(feature = "idl")]
pub use interface_description::InterfaceDescription;

/// The description of the `org.varlink.service` interface.
#[cfg(feature = "introspection")]
pub const DESCRIPTION: &crate::idl::Interface<'static> = &{
    use crate::{
        idl::{Comment, Interface, Method, Parameter},
        introspect::{ReplyError, Type},
    };

    const INTERFACE_PARAM: &Parameter<'static> = &Parameter::new("interface", <&str>::TYPE, &[]);
    const METHODS: &[&Method<'static>] = &[
        &Method::new(
            "GetInfo",
            &[],
            Info::TYPE.as_object().unwrap().as_borrowed().unwrap(),
            &[&Comment::new(
                "Get basic information about the Varlink service",
            )],
        ),
        &Method::new(
            "GetInterfaceDescription",
            &[INTERFACE_PARAM],
            &InterfaceDescription::TYPE
                .as_object()
                .unwrap()
                .as_borrowed()
                .unwrap(),
            &[&Comment::new("Get the description of an interface")],
        ),
    ];

    Interface::new(
        "org.varlink.service",
        METHODS,
        &[],
        Error::VARIANTS,
        &[&Comment::new("Varlink service interface")],
    )
};
