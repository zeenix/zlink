//! Contains connection related API.

mod socket;
use core::fmt::Debug;

use serde::{Deserialize, Serialize};
pub use socket::Socket;

/// A connection.
///
/// The low-level API to send and receive messages.
#[derive(Debug)]
pub struct Connection<S: Socket> {
    socket: S,
    read_pos: usize,

    #[cfg(feature = "std")]
    write_buffer: Vec<u8>,
    #[cfg(not(feature = "std"))]
    write_buffer: [u8; BUFFER_SIZE],
    #[cfg(feature = "std")]
    method_name_buffer: String,
    #[cfg(not(feature = "std"))]
    method_name_buffer: heapless::String<METHOD_NAME_BUFFER_SIZE>,

    #[cfg(feature = "std")]
    read_buffer: Vec<u8>,
    #[cfg(not(feature = "std"))]
    read_buffer: [u8; BUFFER_SIZE],
}

impl<S: Socket> Connection<S> {
    /// Create a new connection.
    pub fn new(socket: S) -> Self {
        Self {
            socket,
            read_pos: 0,
            #[cfg(feature = "std")]
            write_buffer: vec![0; BUFFER_SIZE],
            #[cfg(feature = "std")]
            read_buffer: vec![0; BUFFER_SIZE],
            #[cfg(not(feature = "std"))]
            write_buffer: [0; BUFFER_SIZE],
            #[cfg(not(feature = "std"))]
            read_buffer: [0; BUFFER_SIZE],
            #[cfg(feature = "std")]
            method_name_buffer: String::with_capacity(METHOD_NAME_BUFFER_SIZE),
            #[cfg(not(feature = "std"))]
            method_name_buffer: heapless::String::new(),
        }
    }

    /// Sends a method call.
    pub async fn send_call<P>(
        &mut self,
        interface: &'static str,
        method: &'static str,
        parameters: P,
        one_way: Option<bool>,
        more: Option<bool>,
        upgrade: Option<bool>,
    ) -> crate::Result<()>
    where
        P: Serialize + Debug,
    {
        self.push_method_name(interface, method)?;

        let call = Call {
            method: &self.method_name_buffer,
            parameters,
            one_way,
            more,
            upgrade,
        };
        #[cfg(not(feature = "std"))]
        serde_json_core::to_slice(&call, &mut self.write_buffer)?;
        #[cfg(feature = "std")]
        serde_json::to_writer(&mut self.write_buffer, &call)?;

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
    ///
    /// This method is only available when the `std` feature is enabled. This means that embedded
    /// targets are limited to service implementations only, since this is only needed by clients.
    /// This limitation is likely going to be removed once [`serde-json-core` can handle complex
    /// enums](https://github.com/rust-embedded-community/serde-json-core/issues/94).
    pub async fn receive_reply<'r, Params, ReplyError>(
        &'r mut self,
    ) -> crate::Result<Result<Reply<Params>, ReplyError>>
    where
        Params: Deserialize<'r>,
        ReplyError: Deserialize<'r>,
    {
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

        // We try to deserialize into a miminal struct first to check if it's an error and then
        // deserialize into the user provided `ReplyError` if that's the case. This is to minimize
        // parsing as much as possible.
        #[derive(Debug, Serialize, Deserialize)]
        struct Error<'r> {
            error: &'r str,
        }
        #[cfg(feature = "std")]
        if serde_json::from_slice::<Error<'_>>(buffer).is_ok() {
            return Ok(Err(serde_json::from_slice::<ReplyError>(buffer)?));
        }
        #[cfg(not(feature = "std"))]
        if serde_json_core::from_slice::<Error<'_>>(buffer).is_ok() {
            return Ok(Err(serde_json_core::from_slice::<ReplyError>(buffer)?.0));
        }

        #[cfg(feature = "std")]
        {
            serde_json::from_slice::<Reply<_>>(buffer)
                .map_err(Into::into)
                .map(Ok)
        }

        #[cfg(not(feature = "std"))]
        {
            serde_json_core::from_slice::<Reply<_>>(buffer)
                .map_err(Into::into)
                .map(|(r, _)| Ok(r))
        }
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
                    .extend(std::iter::repeat(0).take(BUFFER_SIZE));
            }

            pos += bytes_read;
        }

        Ok(())
    }

    #[cfg(not(feature = "std"))]
    fn push_method_name(
        &mut self,
        interface: &'static str,
        method: &'static str,
    ) -> crate::Result<()> {
        self.method_name_buffer
            .push_str(interface)
            .map_err(|_| crate::Error::BufferOverflow)?;
        self.method_name_buffer
            .push('.')
            .map_err(|_| crate::Error::BufferOverflow)?;
        self.method_name_buffer
            .push_str(method)
            .map_err(|_| crate::Error::BufferOverflow)?;

        Ok(())
    }

    #[cfg(feature = "std")]
    fn push_method_name(
        &mut self,
        interface: &'static str,
        method: &'static str,
    ) -> crate::Result<()> {
        self.method_name_buffer.clear();
        self.method_name_buffer.push_str(interface);
        self.method_name_buffer.push('.');
        self.method_name_buffer.push_str(method);

        Ok(())
    }
}

/// A successful method call reply.
#[derive(Debug, Serialize, Deserialize)]
pub struct Reply<Params> {
    parameters: Params,
    continues: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Call<'c, P> {
    method: &'c str,
    parameters: P,
    one_way: Option<bool>,
    more: Option<bool>,
    upgrade: Option<bool>,
}

// TODO: Cargo features to customize buffer sizes.
const BUFFER_SIZE: usize = 1024;
#[cfg(feature = "std")]
const MAX_BUFFER_SIZE: usize = 1024 * 1024; // Don't allow buffers over 1MB.
const METHOD_NAME_BUFFER_SIZE: usize = 256;
