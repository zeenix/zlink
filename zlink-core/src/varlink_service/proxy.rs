//! Client-side proxy API for the `org.varlink.service` interface.
//!
//! This module provides the [`Proxy`] trait which offers convenient methods to call
//! the standard Varlink service interface methods on any connection.

use core::{fmt::Debug, future::Future};

use crate::{
    connection::{socket::Socket, Connection},
    idl::Interface,
    introspect::Type,
    Call,
};
use serde::{Deserialize, Serialize};

use super::{Error, Info};

/// Client-side proxy for the `org.varlink.service` interface.
///
/// This trait provides methods to call the standard Varlink service interface
/// methods on a connection.
pub trait Proxy {
    /// Get information about a Varlink service.
    ///
    /// # Returns
    ///
    /// Two-layer result: outer for connection errors, inner for method errors. On success, contains
    /// service information as [`Info`].
    fn get_info(&mut self) -> impl Future<Output = crate::Result<Result<Info<'_>, Error<'_>>>>;

    /// Get the IDL description of an interface.
    ///
    /// # Arguments
    ///
    /// * `interface` - The name of the interface to get the description for.
    ///
    /// # Returns
    ///
    /// Two-layer result: outer for connection errors, inner for method errors. On success, contains
    /// the unparsed interface definition as a [`InterfaceDescription`]. Use
    /// [`InterfaceDescription::parse`] to parse it into an [`Interface`].
    fn get_interface_description(
        &mut self,
        interface: &str,
    ) -> impl Future<Output = crate::Result<Result<InterfaceDescription, Error<'_>>>>;
}

impl<S> Proxy for Connection<S>
where
    S: Socket,
{
    async fn get_info(&mut self) -> crate::Result<Result<Info<'_>, Error<'_>>> {
        let call = Call::new(Method::GetInfo);
        match self.call_method(&call).await? {
            Ok(reply) => match reply.into_parameters() {
                Some(info) => Ok(Ok(info)),
                None => Ok(Err(Error::InvalidParameter {
                    parameter: "missing parameters in reply",
                })),
            },
            Err(error) => Ok(Err(error)),
        }
    }

    async fn get_interface_description(
        &mut self,
        interface: &str,
    ) -> crate::Result<Result<InterfaceDescription, Error<'_>>> {
        let call = Call::new(Method::GetInterfaceDescription { interface });
        let result = self
            .call_method::<_, InterfaceDescription, Error<'_>>(&call)
            .await?;

        match result {
            Ok(reply) => match reply.into_parameters() {
                Some(response) => Ok(Ok(response)),
                None => Ok(Err(Error::InvalidParameter {
                    parameter: "missing parameters in reply",
                })),
            },
            Err(error) => Ok(Err(error)),
        }
    }
}

/// Methods available in the `org.varlink.service` interface.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "parameters")]
enum Method<'a> {
    #[serde(rename = "org.varlink.service.GetInfo")]
    GetInfo,
    #[serde(rename = "org.varlink.service.GetInterfaceDescription")]
    GetInterfaceDescription { interface: &'a str },
}

/// The raw interface description string.
///
/// Use [`InterfaceDescription::parse`] to get the [`Interface`].
#[derive(Debug, Serialize, Deserialize, Type)]
#[zlink(crate = "crate")]
pub struct InterfaceDescription {
    description: String,
}

impl InterfaceDescription {
    /// Parse the interface description as an [`Interface`].
    pub fn parse(&self) -> crate::Result<Interface<'_>> {
        self.description.as_str().try_into()
    }

    /// The raw interface description.
    pub fn as_str(&self) -> &str {
        &self.description
    }
}

impl From<&Interface<'_>> for InterfaceDescription {
    fn from(description: &Interface<'_>) -> Self {
        Self {
            description: description.to_string(),
        }
    }
}
