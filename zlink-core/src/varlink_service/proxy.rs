//! Client-side proxy API for the `org.varlink.service` interface.
//!
//! This module provides the [`Proxy`] trait which offers convenient methods to call
//! the standard Varlink service interface methods on any connection.

use core::{fmt::Debug, future::Future};

use crate::{
    connection::{chain, socket::Socket, Connection},
    Call, Result,
};
use serde::Deserialize;

use super::{Error, Info, InterfaceDescription, Method};

/// Client-side proxy for the `org.varlink.service` interface.
///
/// This trait provides methods to call the standard Varlink service interface
/// methods on a connection.
///
/// # Chaining Calls
///
/// The trait is implemented for both [`Connection`] and [`Chain`], allowing you to
/// chain calls together for efficient batching. Use [`Connection::chain_get_info`] or
/// [`Connection::chain_get_interface_description`] to start a chain.
///
/// ## Example
///
/// ```no_run
/// use zlink_core::{Connection, varlink_service::{Proxy, Chain, Reply, Error}};
/// use serde::Deserialize;
/// use futures_util::{pin_mut, stream::StreamExt};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut conn: Connection<zlink_core::connection::socket::impl_for_doc::Socket> = todo!();
/// // For a single interface, use the provided Reply enum directly
/// let chain = conn
///     .chain_get_info::<Reply<'_>, Error<'_>>()?
///     .get_interface_description("org.example.interface")?
///     .get_info()?;
///
/// // Send the chain and process replies
/// let replies = chain.send().await?;
/// pin_mut!(replies);
///
/// // Process each reply in the order they were chained
/// while let Some(reply) = replies.next().await {
///     match reply??.parameters().unwrap() {
///         Reply::Info(info) => {
///             println!("Service: {} v{} by {}", info.product, info.version, info.vendor);
///             println!("URL: {}", info.url);
///             println!("Interfaces: {:?}", info.interfaces);
///         }
///         Reply::InterfaceDescription(desc) => {
///             println!("Interface description: {}", desc.as_raw().unwrap_or("<parsed>"));
///             // Parse the interface if needed
///             if let Ok(interface) = desc.parse() {
///                 println!("Interface name: {}", interface.name());
///             }
///         }
///     }
/// }
///
/// // For combining multiple interfaces, create a combined reply enum:
/// #[derive(Debug, Deserialize)]
/// #[serde(untagged)]
/// enum CombinedReply<'a> {
///     #[serde(borrow)]
///     VarlinkService(Reply<'a>),
///     // Add other interface reply types here
///     // OtherInterface(other_interface::Reply<'a>),
/// }
///
/// #[derive(Debug, Deserialize)]
/// #[serde(untagged)]
/// enum CombinedError<'a> {
///     #[serde(borrow)]
///     VarlinkService(Error<'a>),
///     // Add other interface error types here
///     // OtherInterface(other_interface::Error<'a>),
/// }
///
/// // Then use the combined types for cross-interface chaining
/// let combined_chain = conn
///     .chain_get_info::<CombinedReply<'_>, CombinedError<'_>>()?;
///     // .other_interface_method()?;  // Chain calls from other interfaces
///
/// let combined_replies = combined_chain.send().await?;
/// pin_mut!(combined_replies);
///
/// while let Some(reply) = combined_replies.next().await {
///     match reply? {
///         Ok(reply) => {
///             match reply.parameters().unwrap() {
///                 CombinedReply::VarlinkService(varlink_reply) => match varlink_reply {
///                     Reply::Info(info) => println!("Varlink service info: {:?}", info),
///                     Reply::InterfaceDescription(desc) => println!("Varlink interface: {:?}", desc),
///                 }
///                 // Handle other interface replies here
///             }
///         }
///         Err(error) => {
///             match error {
///                 CombinedError::VarlinkService(varlink_error) => {
///                     println!("Varlink service error: {:?}", varlink_error);
///                 }
///                 // Handle other interface errors here
///             }
///         }
///     }
/// }
///
/// # Ok(())
/// # }
/// ```
pub trait Proxy {
    /// Get information about a Varlink service.
    ///
    /// # Returns
    ///
    /// Two-layer result: outer for connection errors, inner for method errors. On success, contains
    /// service information as [`Info`].
    fn get_info(
        &mut self,
    ) -> impl Future<Output = crate::Result<core::result::Result<Info<'_>, Error<'_>>>>;

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
    ) -> impl Future<
        Output = crate::Result<core::result::Result<InterfaceDescription<'static>, Error<'_>>>,
    >;

    /// Start a chain with a GetInfo call.
    ///
    /// This creates a [`Chain`] that can be used to batch multiple calls together.
    /// The chain supports calling methods from multiple interfaces by using combined
    /// reply and error types.
    ///
    /// # Type Parameters
    ///
    /// * `ReplyParams` - Combined reply type (usually an untagged enum) that can deserialize
    ///   replies from all interfaces in the chain
    /// * `ReplyError` - Combined error type (usually an untagged enum) that can deserialize errors
    ///   from all interfaces in the chain
    fn chain_get_info<'c, ReplyParams, ReplyError>(
        &'c mut self,
    ) -> Result<chain::Chain<'c, Self::Socket, ReplyParams, ReplyError>>
    where
        ReplyParams: Deserialize<'c> + Debug,
        ReplyError: Deserialize<'c> + Debug;

    /// Start a chain with a GetInterfaceDescription call.
    ///
    /// This creates a [`chain::Chain`] that can be used to batch multiple calls together.
    /// The chain supports calling methods from multiple interfaces by using combined
    /// reply and error types.
    ///
    /// # Arguments
    ///
    /// * `interface` - The name of the interface to get the description for.
    ///
    /// # Type Parameters
    ///
    /// * `ReplyParams` - Combined reply type (usually an untagged enum) that can deserialize
    ///   replies from all interfaces in the chain
    /// * `ReplyError` - Combined error type (usually an untagged enum) that can deserialize errors
    ///   from all interfaces in the chain
    fn chain_get_interface_description<'c, ReplyParams, ReplyError>(
        &'c mut self,
        interface: &str,
    ) -> Result<chain::Chain<'c, Self::Socket, ReplyParams, ReplyError>>
    where
        ReplyParams: Deserialize<'c> + Debug,
        ReplyError: Deserialize<'c> + Debug;

    /// The socket type used by this proxy implementation.
    type Socket: Socket;
}

impl<S> Proxy for Connection<S>
where
    S: Socket,
{
    type Socket = S;

    async fn get_info(&mut self) -> Result<core::result::Result<Info<'_>, Error<'_>>> {
        let call = Call::new(Method::GetInfo);
        match self.call_method::<_, Info<'_>, Error<'_>>(&call).await? {
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
    ) -> Result<core::result::Result<InterfaceDescription<'static>, Error<'_>>> {
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

    fn chain_get_info<'c, ReplyParams, ReplyError>(
        &'c mut self,
    ) -> Result<chain::Chain<'c, S, ReplyParams, ReplyError>>
    where
        ReplyParams: Deserialize<'c> + Debug,
        ReplyError: Deserialize<'c> + Debug,
    {
        let call = Call::new(Method::GetInfo);
        self.chain_call(&call)
    }

    fn chain_get_interface_description<'c, ReplyParams, ReplyError>(
        &'c mut self,
        interface: &str,
    ) -> Result<chain::Chain<'c, S, ReplyParams, ReplyError>>
    where
        ReplyParams: Deserialize<'c> + Debug,
        ReplyError: Deserialize<'c> + Debug,
    {
        let call = Call::new(Method::GetInterfaceDescription { interface });
        self.chain_call(&call)
    }
}

/// Extension trait for adding varlink service proxy calls to any chain.
///
/// This trait provides methods to add varlink service calls to a chain of method calls. It is
/// implemented for [`chain::Chain`] to enable chaining of varlink service calls with calls from
/// other interfaces.
pub trait Chain<'c, S, ReplyParams, ReplyError>
where
    S: Socket,
    ReplyParams: Deserialize<'c> + Debug,
    ReplyError: Deserialize<'c> + Debug,
{
    /// Add a GetInfo call to this chain.
    ///
    /// This method allows chaining varlink service calls with calls from other interfaces.
    /// The chain must be created with combined reply and error types that can handle
    /// responses from all interfaces.
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining, or an error if the call could not be enqueued.
    fn get_info(self) -> Result<chain::Chain<'c, S, ReplyParams, ReplyError>>;

    /// Add a GetInterfaceDescription call to this chain.
    ///
    /// This method allows chaining varlink service calls with calls from other interfaces.
    /// The chain must be created with combined reply and error types that can handle
    /// responses from all interfaces.
    ///
    /// # Arguments
    ///
    /// * `interface` - The name of the interface to get the description for.
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining, or an error if the call could not be enqueued.
    fn get_interface_description(
        self,
        interface: &str,
    ) -> Result<chain::Chain<'c, S, ReplyParams, ReplyError>>;
}

impl<'c, S, ReplyParams, ReplyError> Chain<'c, S, ReplyParams, ReplyError>
    for chain::Chain<'c, S, ReplyParams, ReplyError>
where
    S: Socket,
    ReplyParams: Deserialize<'c> + Debug,
    ReplyError: Deserialize<'c> + Debug,
{
    fn get_info(self) -> Result<chain::Chain<'c, S, ReplyParams, ReplyError>> {
        let call = Call::new(Method::GetInfo);
        self.append(&call)
    }

    fn get_interface_description(
        self,
        interface: &str,
    ) -> Result<chain::Chain<'c, S, ReplyParams, ReplyError>> {
        let call = Call::new(Method::GetInterfaceDescription { interface });
        self.append(&call)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::socket::{ReadHalf, WriteHalf};
    // Note: These imports are used in the full implementation examples
    use mayheap::Vec;

    // Mock socket implementation for testing.
    #[derive(Debug)]
    struct MockSocket {
        read_data: Vec<u8, 1024>,
        read_pos: usize,
    }

    impl MockSocket {
        fn new(responses: &[&str]) -> Self {
            let mut data = Vec::new();

            for response in responses {
                data.extend_from_slice(response.as_bytes()).unwrap();
                data.push(b'\0').unwrap();
            }
            // Add an extra null byte to mark end of all messages
            data.push(b'\0').unwrap();

            Self {
                read_data: data,
                read_pos: 0,
            }
        }
    }

    impl Socket for MockSocket {
        type ReadHalf = MockReadHalf;
        type WriteHalf = MockWriteHalf;

        fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
            (
                MockReadHalf {
                    data: self.read_data,
                    pos: self.read_pos,
                },
                MockWriteHalf {
                    written: Vec::new(),
                },
            )
        }
    }

    #[derive(Debug)]
    struct MockReadHalf {
        data: Vec<u8, 1024>,
        pos: usize,
    }

    impl ReadHalf for MockReadHalf {
        async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
            let remaining = self.data.len().saturating_sub(self.pos);
            if remaining == 0 {
                return Ok(0);
            }

            let to_read = remaining.min(buf.len());
            buf[..to_read].copy_from_slice(&self.data[self.pos..self.pos + to_read]);
            self.pos += to_read;
            Ok(to_read)
        }
    }

    #[derive(Debug)]
    struct MockWriteHalf {
        written: Vec<u8, 1024>,
    }

    impl WriteHalf for MockWriteHalf {
        async fn write(&mut self, buf: &[u8]) -> crate::Result<()> {
            self.written.extend_from_slice(buf).unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_chain_api_creation() -> crate::Result<()> {
        // Test that we can create chains with the new API
        let responses = [
            r#"{"parameters":{"vendor":"Test","product":"TestProduct","version":"1.0","url":"https://test.com","interfaces":["org.varlink.service"]}}"#,
            r#"{"parameters":{"description":"interface org.varlink.service {}"}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        // Use the provided Reply enum from the varlink service module
        use super::{super::Reply, Error};

        // Test that we can create the chain APIs
        let _chain1 = conn.chain_get_info::<Reply<'_>, Error<'_>>()?;
        let _chain2 =
            conn.chain_get_interface_description::<Reply<'_>, Error<'_>>("org.varlink.service")?;

        Ok(())
    }

    #[tokio::test]
    async fn test_chain_extension_methods() -> crate::Result<()> {
        // Test that we can use chain extension methods
        let responses = [
            r#"{"parameters":{"vendor":"Test","product":"TestProduct","version":"1.0","url":"https://test.com","interfaces":["org.varlink.service"]}}"#,
            r#"{"parameters":{"description":"interface org.varlink.service {}"}}"#,
            r#"{"parameters":{"vendor":"Test","product":"TestProduct","version":"1.0","url":"https://test.com","interfaces":["org.varlink.service"]}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        use super::{super::Reply, Error};

        // Test that we can chain calls using extension methods and actually read replies
        let chained = conn
            .chain_get_info::<Reply<'_>, Error<'_>>()?
            .get_interface_description("org.varlink.service")?
            .get_info()?;

        let replies = chained.send().await?;
        use futures_util::{pin_mut, stream::StreamExt};
        pin_mut!(replies);

        // Read first reply (GetInfo)
        let first_reply = replies.next().await.unwrap()?.unwrap();
        match first_reply.parameters().unwrap() {
            Reply::Info(info) => {
                assert_eq!(info.vendor, "Test");
                assert_eq!(info.product, "TestProduct");
                assert_eq!(info.version, "1.0");
                assert_eq!(info.url, "https://test.com");
                assert_eq!(info.interfaces, ["org.varlink.service"]);
            }
            _ => panic!("Expected Info reply"),
        }

        // Read second reply (GetInterfaceDescription)
        let second_reply = replies.next().await.unwrap()?.unwrap();
        match second_reply.parameters().unwrap() {
            Reply::InterfaceDescription(desc) => {
                assert_eq!(desc.as_raw().unwrap(), "interface org.varlink.service {}");
            }
            _ => panic!("Expected InterfaceDescription reply"),
        }

        // Read third reply (GetInfo again)
        let third_reply = replies.next().await.unwrap()?.unwrap();
        match third_reply.parameters().unwrap() {
            Reply::Info(info) => {
                assert_eq!(info.vendor, "Test");
            }
            _ => panic!("Expected Info reply"),
        }

        // No more replies
        assert!(replies.next().await.is_none());

        Ok(())
    }
}
