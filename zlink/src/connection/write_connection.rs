//! Contains connection related API.

use core::fmt::Debug;

use mayheap::Vec;
use serde::Serialize;

use super::{socket::WriteHalf, Call, Reply, BUFFER_SIZE};

/// A connection.
///
/// The low-level API to send messages.
#[derive(Debug)]
pub struct WriteConnection<Write: WriteHalf> {
    socket: Write,
    buffer: Vec<u8, BUFFER_SIZE>,
    id: usize,
}

impl<Write: WriteHalf> WriteConnection<Write> {
    /// Create a new connection.
    pub(super) fn new(socket: Write, id: usize) -> Self {
        Self {
            socket,
            id,
            buffer: Vec::from_slice(&[0; BUFFER_SIZE]).unwrap(),
        }
    }

    /// The unique identifier of the connection.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Sends a method call.
    ///
    /// The generic `Method` is the type of the method name and its input parameters. This should be
    /// a type that can serialize itself to a complete method call message, i-e an object containing
    /// `method` and `parameter` fields. This can be easily achieved using the `serde::Serialize`
    /// derive:
    ///
    /// ```rust
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Debug, Serialize, Deserialize)]
    /// #[serde(tag = "method", content = "parameters")]
    /// enum MyMethods<'m> {
    ///    // The name needs to be the fully-qualified name of the error.
    ///    #[serde(rename = "org.example.ftl.Alpha")]
    ///    Alpha { param1: u32, param2: &'m str},
    ///    #[serde(rename = "org.example.ftl.Bravo")]
    ///    Bravo,
    ///    #[serde(rename = "org.example.ftl.Charlie")]
    ///    Charlie { param1: &'m str },
    /// }
    /// ```
    pub async fn send_call<Method>(
        &mut self,
        method: Method,
        oneway: Option<bool>,
        more: Option<bool>,
        upgrade: Option<bool>,
    ) -> crate::Result<()>
    where
        Method: Serialize + Debug,
    {
        let call = Call {
            method,
            oneway,
            more,
            upgrade,
        };
        let len = to_slice(&call, &mut self.buffer)?;
        self.buffer[len] = b'\0';

        self.socket.write(&self.buffer[..=len]).await
    }

    /// Send a reply over the socket.
    ///
    /// The generic parameter `Params` is the type of the successful reply. This should be a type
    /// that can serialize itself as the `parameters` field of the reply.
    pub async fn send_reply<Params>(
        &mut self,
        parameters: Option<Params>,
        continues: Option<bool>,
    ) -> crate::Result<()>
    where
        Params: Serialize + Debug,
    {
        let reply = Reply {
            parameters,
            continues,
        };
        let len = to_slice(&reply, &mut self.buffer)?;
        self.buffer[len] = b'\0';

        self.socket.write(&self.buffer[..=len]).await
    }

    /// Send an error reply over the socket.
    ///
    /// The generic parameter `ReplyError` is the type of the error reply. This should be a type
    /// that can serialize itself to the whole reply object, containing `error` and `parameter`
    /// fields. This can be easily achieved using the `serde::Serialize` derive (See the code
    /// snippet in [`super::ReadConnection::receive_reply`] documentation for an example).
    pub async fn send_error<ReplyError>(&mut self, error: ReplyError) -> crate::Result<()>
    where
        ReplyError: Serialize + Debug,
    {
        let len = to_slice(&error, &mut self.buffer)?;
        self.buffer[len] = b'\0';

        self.socket.write(&self.buffer[..=len]).await
    }
}

fn to_slice<T>(value: &T, buf: &mut [u8]) -> crate::Result<usize>
where
    T: Serialize + ?Sized,
{
    #[cfg(feature = "std")]
    {
        let mut buf = std::io::Cursor::new(buf);
        serde_json::to_writer(&mut buf, value)?;

        Ok(buf.position() as usize)
    }

    #[cfg(not(feature = "std"))]
    {
        serde_json_core::to_slice(value, buf).map_err(Into::into)
    }
}
