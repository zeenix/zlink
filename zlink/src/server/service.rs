//! Serice-related API.

use core::{fmt::Debug, future::Future};

use futures_util::Stream;
use serde::{Deserialize, Serialize};

use crate::{
    connection::{Call, Socket},
    Connection,
};

/// Service trait for handling method calls.
pub trait Service
where
    <Self::ReplyStream as Stream>::Item: Serialize + core::fmt::Debug,
{
    /// The type of method call that this service handles.
    ///
    /// This should be a type that can deserialize itself from a complete method call message: i-e
    /// an object containing `method` and `parameter` fields. This can be easily achieved using the
    /// `serde::Deserialize` derive (See the code snippet in [`Connection::send_call`] documentation
    /// for an example).
    type MethodCall<'de>: Deserialize<'de> + Debug;
    /// The type of the successful reply.
    ///
    /// This should be a type that can serialize itself as the `parameters` field of the reply.
    type ReplyParams<'ser>: Serialize + Debug
    where
        Self: 'ser;
    /// The type of the multi-reply stream.
    ///
    /// If the client asks for multiple replies, this stream will be used to send them. The stream
    /// must yield items that can be serialized as the `parameters` field of the reply.
    type ReplyStream: Stream + Debug;
    /// The type of the error reply.
    ///
    /// This should be a type that can serialize itself to the whole reply object, containing
    /// `error` and `parameter` fields. This can be easily achieved using the `serde::Serialize`
    /// derive (See the code snippet in [`Connection::receive_reply`] documentation for an example).
    type ReplyError<'ser>: Serialize + Debug
    where
        Self: 'ser;

    /// Handle a method call.
    fn handle<'ser>(
        &'ser mut self,
        method: Call<Self::MethodCall<'_>>,
    ) -> impl Future<
        Output = Reply<Option<Self::ReplyParams<'ser>>, Self::ReplyStream, Self::ReplyError<'ser>>,
    >;

    /// Read the next method call from the connection and handle it.
    fn handle_next<Sock>(
        &mut self,
        connection: &mut Connection<Sock>,
    ) -> impl Future<Output = crate::Result<Option<Self::ReplyStream>>>
    where
        Sock: Socket,
    {
        async {
            let reply = {
                // Safety: The compiler doesn't know that we write to different fields
                //         in `read` and `write` so doesn't like us borrowing it twice.
                let connection = unsafe { &mut *(connection as *mut Connection<Sock>) };
                let call: Call<Self::MethodCall<'_>> = connection.receive_call().await?;
                self.handle(call).await
            };
            match reply {
                Reply::Single(reply) => {
                    connection.send_reply(reply, Some(false)).await?;

                    Ok(None)
                }
                Reply::Error(err) => {
                    connection.send_error(err).await?;

                    Ok(None)
                }
                Reply::Multi(stream) => Ok(Some(stream)),
            }
        }
    }
}

/// A service method call reply.
#[derive(Debug)]
pub enum Reply<Params, ReplyStream, ReplyError> {
    /// A single reply.
    Single(Params),
    /// An error reply.
    Error(ReplyError),
    /// A multi-reply stream.
    Multi(ReplyStream),
}
