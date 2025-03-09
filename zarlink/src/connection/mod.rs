//! Contains connection related API.

mod socket;
use core::fmt::Debug;

use mayheap::Vec;
use serde::{Deserialize, Serialize};
pub use socket::Socket;

/// A connection.
///
/// The low-level API to send and receive messages.
#[derive(Debug)]
pub struct Connection<S: Socket> {
    socket: S,
    read_pos: usize,

    write_buffer: Vec<u8, BUFFER_SIZE>,
    read_buffer: Vec<u8, BUFFER_SIZE>,
}

impl<S: Socket> Connection<S> {
    /// Create a new connection.
    pub fn new(socket: S) -> Self {
        Self {
            socket,
            read_pos: 0,
            write_buffer: Vec::from_slice(&[0; BUFFER_SIZE]).unwrap(),
            read_buffer: Vec::from_slice(&[0; BUFFER_SIZE]).unwrap(),
        }
    }

    /// Sends a method call.
    ///
    /// The generic `M` is the type of the method name and its input parameters. This should be a
    /// type that can serialize itself to a complete method call message, i-e an object containing
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
    pub async fn send_call<M>(
        &mut self,
        method: M,
        one_way: Option<bool>,
        more: Option<bool>,
        upgrade: Option<bool>,
    ) -> crate::Result<()>
    where
        M: Serialize + Debug,
    {
        let call = Call {
            method,
            one_way,
            more,
            upgrade,
        };
        to_slice(&call, &mut self.write_buffer)?;

        self.socket.write(&self.write_buffer).await
    }

    /// Receives a method call reply.
    ///
    /// The generic parameters needs some explanation:
    ///
    /// * `R` is the type of the successful reply. This should be a type that can deserialize itself
    ///   from the `parameters` field of the reply.
    /// * `E` is the type of the error reply. This should be a type that can deserialize itself from
    ///   the whole reply object itself and must fail when there is no `error` field in the object.
    ///   This can be easily achieved using the `serde::Deserialize` derive:
    ///
    /// ```rust
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Deserialize, Serialize)]
    /// #[serde(tag = "error", content = "parameters")]
    /// enum MyError {
    ///    // The name needs to be the fully-qualified name of the error.
    ///    #[serde(rename = "org.example.ftl.Alpha")]
    ///    Alpha { param1: u32, param2: String },
    ///    #[serde(rename = "org.example.ftl.Bravo")]
    ///    Bravo,
    ///    #[serde(rename = "org.example.ftl.Charlie")]
    ///    Charlie { param1: String },
    /// }
    /// ```
    pub async fn receive_reply<'r, Params, ReplyError>(
        &'r mut self,
    ) -> crate::Result<Result<Reply<Params>, ReplyError>>
    where
        Params: Deserialize<'r>,
        ReplyError: Deserialize<'r>,
    {
        let buffer = self.read_message_bytes().await?;

        // First try to parse it as an error.
        // FIXME: This will mean the document will be parsed twice. We should instead try to
        // quickly check if `error` field is present and then parse to the appropriate type based on
        // that information. Perhaps a simple parser using `winnow`?
        match from_slice::<ReplyError>(buffer) {
            Ok(e) => Ok(Err(e)),
            Err(_) => from_slice::<Reply<_>>(buffer).map(Ok),
        }
    }

    /// Receive a method call over the socket.
    ///
    /// The generic `M` is the type of the method name and its input parameters. This should be a
    /// type that can deserialize itself from a complete method call message, i-e an object
    /// containing `method` and `parameter` fields. This can be easily achieved using the
    /// `serde::Deserialize` derive (See the code snippet in [`Connection::send_call`] documentation
    /// for an example).
    pub async fn receive_call<'m, M>(&'m mut self) -> crate::Result<Call<M>>
    where
        M: Deserialize<'m>,
    {
        let buffer = self.read_message_bytes().await?;

        from_slice::<Call<M>>(buffer)
    }

    // Reads at least one full message from the socket and return a single message bytes.
    async fn read_message_bytes(&mut self) -> crate::Result<&'_ [u8]> {
        self.read_from_socket().await?;

        // Unwrap is safe because `read_from_socket` call above ensures at least one null byte in
        // the buffer.
        let null_index = memchr::memchr(b'\0', &self.read_buffer[self.read_pos..]).unwrap();
        let buffer = &self.read_buffer[self.read_pos..null_index];
        if self.read_buffer[null_index + 1] == b'\0' {
            // This means we're reading the last message and can now reset the index.
            self.read_pos = 0;
        } else {
            self.read_pos = null_index + 1;
        }

        Ok(buffer)
    }

    // Reads at least one full message from the socket.
    async fn read_from_socket(&mut self) -> crate::Result<()> {
        if self.read_pos > 0 {
            // This means we already have at least one message in the buffer so no need to read.
            return Ok(());
        }

        let mut pos = self.read_pos;
        loop {
            let bytes_read = self.socket.read(&mut self.read_buffer[pos..]).await?;
            let total_read = pos + bytes_read;

            // This marks end of all messages. After this loop is finished, we'll have 2 consecutive
            // null bytes at the end. This is then used by the callers to determine that they've
            // read all messages and can now reset the `read_pos`.
            self.write_buffer[total_read] = b'\0';

            if self.write_buffer[total_read - 1] == b'\0' {
                // One or more full messages were read.
                break;
            }

            #[cfg(feature = "std")]
            if total_read >= self.write_buffer.len() {
                if total_read >= MAX_BUFFER_SIZE {
                    return Err(crate::Error::BufferOverflow);
                }

                self.write_buffer
                    .extend(core::iter::repeat(0).take(BUFFER_SIZE));
            }

            pos += bytes_read;
        }

        Ok(())
    }
}

/// A successful method call reply.
#[derive(Debug, Serialize, Deserialize)]
pub struct Reply<Params> {
    parameters: Params,
    continues: Option<bool>,
}

impl<Params> Reply<Params> {
    /// The parameters of the reply.
    pub fn parameters(&self) -> &Params {
        &self.parameters
    }

    /// If there are more replies to come.
    pub fn continues(&self) -> Option<bool> {
        self.continues
    }
}

/// A method call.
#[derive(Debug, Serialize, Deserialize)]
pub struct Call<M> {
    #[serde(flatten)]
    method: M,
    one_way: Option<bool>,
    more: Option<bool>,
    upgrade: Option<bool>,
}

impl<M> Call<M> {
    /// The method call name and parameters.
    pub fn method(&self) -> &M {
        &self.method
    }

    /// If the method call doesn't want a reply.
    pub fn one_way(&self) -> Option<bool> {
        self.one_way
    }

    /// If the method call is requesting more replies.
    pub fn more(&self) -> Option<bool> {
        self.more
    }

    /// If the method call is requesting an upgrade to a different protocol.
    pub fn upgrade(&self) -> Option<bool> {
        self.upgrade
    }
}

#[cfg(feature = "io-buffer-1mb")]
const BUFFER_SIZE: usize = 1024 * 1024;
#[cfg(all(not(feature = "io-buffer-1mb"), feature = "io-buffer-16kb"))]
const BUFFER_SIZE: usize = 16 * 1024;
#[cfg(all(
    not(feature = "io-buffer-1mb"),
    not(feature = "io-buffer-16kb"),
    feature = "io-buffer-4kb"
))]
const BUFFER_SIZE: usize = 4 * 1024;

#[cfg(feature = "std")]
const MAX_BUFFER_SIZE: usize = 100 * 1024 * 1024; // Don't allow buffers over 100MB.

fn from_slice<'a, T>(buffer: &'a [u8]) -> crate::Result<T>
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

fn to_slice<T>(value: &T, buf: &mut [u8]) -> crate::Result<()>
where
    T: Serialize + ?Sized,
{
    #[cfg(feature = "std")]
    {
        serde_json::to_writer(buf, value).map_err(Into::into)
    }

    #[cfg(not(feature = "std"))]
    {
        serde_json_core::to_slice(value, buf)
            .map_err(Into::into)
            .map(|_| ())
    }
}
