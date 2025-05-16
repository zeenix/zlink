//! A client-side proxy to a service interface.

use core::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};
use futures_util::stream::Stream;
use mayheap::String;
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};

use crate::{
    connection::{socket::ReadHalf, ReadConnection, Socket},
    reply, Call, Connection, Result,
};

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

        self.connection.send_call(&call).await?;

        let read_conn = self.connection.read_mut();
        let stream = ReplyStream::new(
            read_conn,
            ReadConnection::receive_reply::<ReplyParams, ReplyError>,
        );

        Ok(stream)
    }
}

#[derive(Debug, Serialize)]
struct Method<Params> {
    method: String<MAX_METHOD_NAME_LEN>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<Params>,
}

impl<Params> Method<Params>
where
    Params: Serialize + Debug,
{
    fn new(name: &str, parameters: Option<Params>) -> Result<Self> {
        let mut method_name: String<MAX_METHOD_NAME_LEN> = String::new();
        method_name.push('.')?;
        method_name.push_str(name)?;

        Ok(Method {
            method: method_name,
            parameters,
        })
    }
}

pin_project! {
    /// A stream of replies.
    ///
    /// This is the type returned by [`Proxy::call_more`].
    ///
    /// This would be useful for external use as well but we keep it internal because
    /// [`ReadConnection::receive_reply`] gives us anonymous futures so we have to keep the future
    /// type generic here. This can be solved with Boxing but we want to avoid all allocations in
    /// the core library.
    #[derive(Debug)]
    struct ReplyStream<'c, Read: ReadHalf, F, Fut> {
        #[pin]
        state: ReplyStreamState<Fut>,
        conn: &'c mut ReadConnection<Read>,
        func: F,
    }
}

impl<'c, Read, F, Fut, Params, ReplyError> ReplyStream<'c, Read, F, Fut>
where
    Read: ReadHalf,
    F: FnMut(&'c mut ReadConnection<Read>) -> Fut,
    Fut: Future<Output = Result<reply::Result<Params, ReplyError>>>,
    Params: Deserialize<'c>,
    ReplyError: Deserialize<'c>,
{
    fn new(conn: &'c mut ReadConnection<Read>, func: F) -> Self {
        ReplyStream {
            state: ReplyStreamState::Init,
            conn,
            func,
        }
    }
}

impl<'c, Read, F, Fut, Params, ReplyError> Stream for ReplyStream<'c, Read, F, Fut>
where
    Read: ReadHalf,
    F: FnMut(&'c mut ReadConnection<Read>) -> Fut,
    Fut: Future<Output = Result<reply::Result<Params, ReplyError>>>,
    Params: Deserialize<'c>,
    ReplyError: Deserialize<'c>,
{
    type Item = Result<reply::Result<Params, ReplyError>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if this.state.as_mut().check_init() {
            let conn = unsafe { &mut *(*this.conn as *mut _) };
            this.state.set(ReplyStreamState::Future {
                future: (this.func)(conn),
            });
        }

        let item = match this.state.as_mut().project_future() {
            Some(fut) => ready!(fut.poll(cx)),
            None => panic!("Unfold must not be polled after it returned `Poll::Ready(None)`"),
        };

        this.state.set(ReplyStreamState::Init);
        Poll::Ready(Some(item))
    }
}

pin_project! {
    /// State for `ReplyStream`.
    ///
    /// Based on the [`futures::stream::unfold`] implementation.
    #[project = ReplyStreamStateProj]
    #[project_replace = ReplyStreamStateProjReplace]
    #[derive(Debug)]
    enum ReplyStreamState<R> {
        Init,
        Future {
            #[pin]
            future: R,
        },
        Empty,
    }
}

impl<R> ReplyStreamState<R> {
    fn project_future(self: Pin<&mut Self>) -> Option<Pin<&mut R>> {
        match self.project() {
            ReplyStreamStateProj::Future { future } => Some(future),
            _ => None,
        }
    }

    fn check_init(self: Pin<&mut Self>) -> bool {
        match &*self {
            Self::Init => match self.project_replace(Self::Empty) {
                ReplyStreamStateProjReplace::Init => true,
                _ => unreachable!(),
            },
            _ => false,
        }
    }
}

const MAX_METHOD_NAME_LEN: usize = 32;
