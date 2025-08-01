//! Contains connection related API.

use core::fmt::Debug;

use crate::Result;

#[cfg(feature = "std")]
use super::MAX_BUFFER_SIZE;
use super::{
    reply::{self, Reply},
    socket::ReadHalf,
    Call, BUFFER_SIZE,
};
use mayheap::Vec;
use memchr::memchr;
use serde::Deserialize;

/// A connection that can only be used for reading.
///
/// # Cancel safety
///
/// All async methods of this type are cancel safe unless explicitly stated otherwise in its
/// documentation.
#[derive(Debug)]
pub struct ReadConnection<Read: ReadHalf> {
    socket: Read,
    read_pos: usize,
    msg_pos: usize,
    buffer: Vec<u8, BUFFER_SIZE>,
    id: usize,
}

impl<Read: ReadHalf> ReadConnection<Read> {
    /// Create a new connection.
    pub(super) fn new(socket: Read, id: usize) -> Self {
        Self {
            socket,
            read_pos: 0,
            msg_pos: 0,
            id,
            buffer: Vec::from_slice(&[0; BUFFER_SIZE]).unwrap(),
        }
    }

    /// The unique identifier of the connection.
    #[inline]
    pub fn id(&self) -> usize {
        self.id
    }

    /// Receives a method call reply.
    ///
    /// The generic parameters needs some explanation:
    ///
    /// * `ReplyParams` is the type of the successful reply. This should be a type that can
    ///   deserialize itself from the `parameters` field of the reply.
    /// * `ReplyError` is the type of the error reply. This should be a type that can deserialize
    ///   itself from the whole reply object itself and must fail when there is no `error` field in
    ///   the object. This can be easily achieved using the `serde::Deserialize` derive:
    ///
    /// ```rust
    /// use serde::{Deserialize, Serialize};
    /// use serde_prefix_all::prefix_all;
    ///
    /// #[prefix_all("org.example.ftl.")]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// #[serde(tag = "error", content = "parameters")]
    /// enum MyError {
    ///     // The name needs to be the fully-qualified name of the error.
    ///     Alpha { param1: u32, param2: String },
    ///     Bravo,
    ///     Charlie { param1: String },
    /// }
    /// ```
    pub async fn receive_reply<'r, ReplyParams, ReplyError>(
        &'r mut self,
    ) -> Result<reply::Result<ReplyParams, ReplyError>>
    where
        ReplyParams: Deserialize<'r> + Debug,
        ReplyError: Deserialize<'r> + Debug,
    {
        let id = self.id;
        let buffer = self.read_message_bytes().await?;

        // First, check if the message has an "error" field to determine how to deserialize.
        // FIXME: This will mean the document will be parsed twice. We should instead try to
        // quickly check if `error` field is present and then parse to the appropriate type based on
        // that information. Perhaps a simple parser using `winnow`?
        if extract_error_name(buffer).is_some() {
            // It's an error response.
            return from_slice::<ReplyError>(buffer).map(Err);
        }

        // It's a success response.
        let ret = from_slice::<Reply<ReplyParams>>(buffer).map(Ok);
        trace!("connection {}: received reply: {:?}", id, ret);

        ret
    }

    /// Receive a method call over the socket.
    ///
    /// The generic `Method` is the type of the method name and its input parameters. This should be
    /// a type that can deserialize itself from a complete method call message, i-e an object
    /// containing `method` and `parameter` fields. This can be easily achieved using the
    /// `serde::Deserialize` derive (See the code snippet in [`super::WriteConnection::send_call`]
    /// documentation for an example).
    pub async fn receive_call<'m, Method>(&'m mut self) -> Result<Call<Method>>
    where
        Method: Deserialize<'m> + Debug,
    {
        let id = self.id;
        let buffer = self.read_message_bytes().await?;

        let call = from_slice::<Call<Method>>(buffer)?;
        trace!("connection {}: received a call: {:?}", id, call);

        Ok(call)
    }

    // Reads at least one full message from the socket and return a single message bytes.
    pub(super) async fn read_message_bytes(&mut self) -> Result<&'_ [u8]> {
        self.read_from_socket().await?;

        // Unwrap is safe because `read_from_socket` call above ensures at least one null byte in
        // the buffer.
        let null_index = memchr(b'\0', &self.buffer[self.msg_pos..]).unwrap() + self.msg_pos;
        let buffer = &self.buffer[self.msg_pos..null_index];
        if self.buffer[null_index + 1] == b'\0' {
            // This means we're reading the last message and can now reset the indices.
            self.read_pos = 0;
            self.msg_pos = 0;
        } else {
            self.msg_pos = null_index + 1;
        }

        Ok(buffer)
    }

    // Reads at least one full message from the socket.
    async fn read_from_socket(&mut self) -> Result<()> {
        if self.msg_pos > 0 {
            // This means we already have at least one message in the buffer so no need to read.
            return Ok(());
        }

        loop {
            let bytes_read = self.socket.read(&mut self.buffer[self.read_pos..]).await?;
            if bytes_read == 0 {
                #[cfg(not(feature = "std"))]
                return Err(crate::Error::SocketRead);
                #[cfg(feature = "std")]
                return Err(crate::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "unexpected EOF",
                )));
            }
            self.read_pos += bytes_read;

            #[cfg(feature = "std")]
            if self.read_pos == self.buffer.len() {
                if self.read_pos >= MAX_BUFFER_SIZE {
                    return Err(crate::Error::BufferOverflow);
                }

                self.buffer.extend(core::iter::repeat_n(0, BUFFER_SIZE));
            }

            // This marks end of all messages. After this loop is finished, we'll have 2 consecutive
            // null bytes at the end. This is then used by the callers to determine that they've
            // read all messages and can now reset the `read_pos`.
            self.buffer[self.read_pos] = b'\0';

            if self.buffer[self.read_pos - 1] == b'\0' {
                // One or more full messages were read.
                break;
            }
        }

        Ok(())
    }

    /// The underlying read half of the socket.
    pub fn read_half(&self) -> &Read {
        &self.socket
    }
}

fn from_slice<'a, T>(buffer: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    #[cfg(feature = "std")]
    {
        serde_json::from_slice::<T>(buffer).map_err(Into::into)
    }

    #[cfg(not(feature = "std"))]
    {
        serde_json_core::from_slice::<T>(buffer)
            .map_err(Into::into)
            .map(|(e, _)| e)
    }
}

/// If the buffer contains a JSON object with an "error" field, this function will fetch it.
fn extract_error_name(buffer: &[u8]) -> Option<&str> {
    #[derive(Deserialize)]
    struct Error<'a> {
        error: &'a str,
    }
    from_slice::<Error<'_>>(buffer)
        .ok()
        .map(|error| error.error)
}
