//! Convenience API for maintaining state, that notifies on changes.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::connection::Reply;
use async_broadcast::{Receiver, Sender};

/// A notified state (e.g a field) of a service implementation.
#[derive(Debug, Clone)]
pub struct State<T, ReplyParams> {
    value: T,
    tx: Sender<ReplyParams>,
    rx: Receiver<ReplyParams>,
}

impl<T, ReplyParams> State<T, ReplyParams>
where
    T: Into<ReplyParams> + Clone,
    ReplyParams: Clone + Send + 'static,
{
    /// Create a new notified field.
    pub fn new(value: T) -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(1);
        tx.set_overflow(true);

        Self { value, tx, rx }
    }

    /// Set the value of the notified field and notify all listeners.
    pub fn set(&mut self, value: T) {
        self.value = value.clone();
        self.tx
            .broadcast_blocking(value.into())
            // We enabled overflow so this can't fail.
            .unwrap();
    }

    /// Get the value of the notified field.
    pub fn get(&self) -> T {
        self.value.clone()
    }

    /// Get a stream of replies for the notified field.
    pub fn stream(&mut self) -> Stream<ReplyParams> {
        Stream(self.rx.clone())
    }
}

/// The stream to use as the [`crate::Service::ReplyStream`] in service implementation when using
/// [`State`].
#[derive(Debug)]
pub struct Stream<ReplyParams>(Receiver<ReplyParams>);

impl<ReplyParams> futures_util::Stream for Stream<ReplyParams>
where
    ReplyParams: Clone,
{
    type Item = Reply<ReplyParams>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().0).poll_next(cx).map(|reply| {
            reply.map(|reply| {
                // We need to set the continues flag to true so that the client knows that this is a
                // stream.
                Reply::new(Some(reply)).set_continues(Some(true))
            })
        })
    }
}
