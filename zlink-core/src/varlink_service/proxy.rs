//! Client-side proxy API for the `org.varlink.service` interface.
//!
//! This module provides the [`Proxy`] trait which offers convenient methods to call
//! the standard Varlink service interface methods on any connection.

use core::future::Future;

use crate::{
    connection::{socket::Socket, Connection},
    Call,
};

use super::{Error, Info, InterfaceDescription, Method};

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
    /// [`InterfaceDescription::parse`] to parse it.
    fn get_interface_description(
        &mut self,
        interface: &str,
    ) -> impl Future<Output = crate::Result<Result<InterfaceDescription<'static>, Error<'_>>>>;
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
    ) -> crate::Result<Result<InterfaceDescription<'static>, Error<'_>>> {
        let call = Call::new(Method::GetInterfaceDescription { interface });
        let result = self
            .call_method::<_, InterfaceDescription<'static>, Error<'_>>(&call)
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
