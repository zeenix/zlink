//! Serice-related API.

use core::fmt::Debug;

use futures_util::Stream;
use serde::{Deserialize, Serialize};

use crate::connection::{Call, Reply};

/// Service trait for handling method calls.
pub trait Service {
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
    /// The type of the item that [`Service::ReplyStream`] will be expected to yield.
    ///
    /// This should be a type that can serialize itself as the `parameters` field of the reply.
    type ReplyStreamParams: Serialize + Debug;
    /// The type of the multi-reply stream.
    ///
    /// If the client asks for multiple replies, this stream will be used to send them.
    type ReplyStream: Stream<Item = Reply<Self::ReplyStreamParams>> + Unpin + Debug;
    /// The type of the error reply.
    ///
    /// This should be a type that can serialize itself to the whole reply object, containing
    /// `error` and `parameter` fields. This can be easily achieved using the `serde::Serialize`
    /// derive (See the code snippet in [`Connection::receive_reply`] documentation for an example).
    type ReplyError<'ser>: Serialize + Debug
    where
        Self: 'ser;

    /// Handle a method call.
    ///
    /// While this methos is no async, it can return a reply stream that can be used to
    /// ascynchronously send out replies. The main use case for allowing to return a stream is
    /// multiple replies but it can be used for single reply as well.
    fn handle<'ser>(
        &'ser mut self,
        method: Call<Self::MethodCall<'_>>,
    ) -> MethodReply<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>>;
}

/// A service method call reply.
#[derive(Debug)]
pub enum MethodReply<Params, ReplyStream, ReplyError> {
    /// A single reply.
    Single(Option<Params>),
    /// An error reply.
    Error(ReplyError),
    /// A multi-reply stream.
    Multi(ReplyStream),
}
