//! A client-side proxy to a service interface.

mod method;

use core::fmt::Debug;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};

use crate::{connection::Socket, reply, Call, Connection, Result};

use method::Method;

/// A client-side proxy to a service interface.
///
/// This is slightly higher-level API than offered by [`Connection`].
#[derive(Debug)]
pub struct Proxy<'interface, Sock: Socket> {
    connection: Connection<Sock>,
    interface: &'interface str,
}

impl<'interface, Sock> Proxy<'interface, Sock>
where
    Sock: Socket,
{
    /// Create a new proxy for the given connection and interface.
    pub fn new(connection: Connection<Sock>, interface: &'interface str) -> Self {
        Self {
            connection,
            interface,
        }
    }

    /// The connection to the proxy.
    pub fn connection(&self) -> &Connection<Sock> {
        &self.connection
    }

    /// The mutable connection to the proxy.
    pub fn connection_mut(&mut self) -> &mut Connection<Sock> {
        &mut self.connection
    }

    /// The interface of the proxy.
    pub fn interface(&self) -> &str {
        self.interface
    }

    /// Call a method call through the proxy.
    pub async fn call<'p, Params, ReplyError, ReplyParams>(
        &'p mut self,
        method_name: &str,
        params: Option<Params>,
    ) -> Result<reply::Result<ReplyParams, ReplyError>>
    where
        Params: Serialize + Debug,
        ReplyError: Deserialize<'p> + Debug,
        ReplyParams: Deserialize<'p> + Debug,
    {
        let method = Method::new(method_name, params)?;
        let call = Call::new(method);

        self.connection.call_method(&call).await
    }

    /// Call a method call through the proxy, requesting more than 1 reply.
    pub async fn call_more<'p, Params, ReplyParams, ReplyError>(
        &'p mut self,
        method_name: &str,
        params: Option<Params>,
    ) -> Result<impl Stream<Item = Result<reply::Result<ReplyParams, ReplyError>>> + 'p>
    where
        Params: Serialize + Debug,
        ReplyParams: Deserialize<'p> + Debug + 'p,
        ReplyError: Deserialize<'p> + Debug + 'p,
    {
        let method = Method::new(method_name, params)?;
        let call = Call::new(method).set_more(Some(true));

        // Use the chain API with a single call.
        self.connection
            .chain_call::<Method<Params>, ReplyParams, ReplyError>(&call)?
            .send()
            .await
    }
}
