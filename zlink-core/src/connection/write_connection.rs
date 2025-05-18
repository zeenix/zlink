//! Contains connection related API.

use core::fmt::Debug;

use mayheap::Vec;
use serde::Serialize;

use super::{socket::WriteHalf, Call, Reply, BUFFER_SIZE};

/// A connection.
///
/// The low-level API to send messages.
///
/// # Cancel safety
///
/// All async methods of this type are cancel safe unless explicitly stated otherwise in its
/// documentation.
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
    #[inline]
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
    pub async fn send_call<Method>(&mut self, call: &Call<Method>) -> crate::Result<()>
    where
        Method: Serialize + Debug,
    {
        trace!("connection {}: sending call: {:?}", self.id, call);
        self.write(call).await
    }

    /// Send a reply over the socket.
    ///
    /// The generic parameter `Params` is the type of the successful reply. This should be a type
    /// that can serialize itself as the `parameters` field of the reply.
    pub async fn send_reply<Params>(&mut self, reply: &Reply<Params>) -> crate::Result<()>
    where
        Params: Serialize + Debug,
    {
        trace!("connection {}: sending reply: {:?}", self.id, reply);
        self.write(reply).await
    }

    /// Send an error reply over the socket.
    ///
    /// The generic parameter `ReplyError` is the type of the error reply. This should be a type
    /// that can serialize itself to the whole reply object, containing `error` and `parameter`
    /// fields. This can be easily achieved using the `serde::Serialize` derive (See the code
    /// snippet in [`super::ReadConnection::receive_reply`] documentation for an example).
    pub async fn send_error<ReplyError>(&mut self, error: &ReplyError) -> crate::Result<()>
    where
        ReplyError: Serialize + Debug,
    {
        trace!("connection {}: sending error: {:?}", self.id, error);
        self.write(error).await
    }

    async fn write<T>(&mut self, value: &T) -> crate::Result<()>
    where
        T: Serialize + ?Sized + Debug,
    {
        let len = loop {
            match to_slice(value, &mut self.buffer) {
                Ok(len) => break len,
                #[cfg(feature = "std")]
                Err(crate::Error::Json(e)) if e.is_io() => {
                    // This can only happens if `serde-json` failed to write all bytes and that
                    // means we're running out of space or already are out of space.
                    self.buffer.extend_from_slice(&[0; BUFFER_SIZE])?;
                }
                Err(e) => return Err(e),
            }
        };
        if len == self.buffer.len() {
            self.buffer.extend_from_slice(&[0; BUFFER_SIZE])?;
        } else {
            self.buffer[len] = b'\0';
        }
        self.socket.write(&self.buffer[..=len]).await.map(|_| ())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestWriteHalf(usize);

    impl WriteHalf for TestWriteHalf {
        async fn write(&mut self, value: &[u8]) -> crate::Result<()> {
            assert_eq!(value.len(), self.0);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_write() {
        const WRITE_LEN: usize =
            // Every `0u8` is one byte.
            BUFFER_SIZE +
            // `,` separators.
            (BUFFER_SIZE - 1) +
            // `[` and `]`.
            2 +
            // null byte.
            1;
        let mut write_conn = WriteConnection::new(TestWriteHalf(WRITE_LEN), 1);
        // An item that serializes into `> BUFFER_SIZE * 2` bytes.
        let item: Vec<u8, BUFFER_SIZE> = Vec::from_slice(&[0u8; BUFFER_SIZE]).unwrap();
        let res = write_conn.write(&item).await;
        #[cfg(feature = "std")]
        {
            res.unwrap();
            assert_eq!(write_conn.buffer.len(), BUFFER_SIZE * 3);
        }
        #[cfg(feature = "embedded")]
        {
            assert!(matches!(
                res,
                Err(crate::Error::JsonSerialize(
                    serde_json_core::ser::Error::BufferFull
                ))
            ));
            assert_eq!(write_conn.buffer.len(), BUFFER_SIZE);
        }
    }
}
