//! Convenience API for maintaining state, that notifies on changes.

use std::{
    fmt::Debug,
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::connection::Reply;
use tokio::sync::broadcast::{channel, Sender};
use tokio_stream::wrappers::BroadcastStream;

/// A notified state (e.g a field) of a service implementation.
#[derive(Debug, Clone)]
pub struct State<T, ReplyParams> {
    value: T,
    tx: Sender<ReplyParams>,
}

impl<T, ReplyParams> State<T, ReplyParams>
where
    T: Into<ReplyParams> + Clone + Debug,
    ReplyParams: Clone + Send + 'static + Debug,
{
    /// Create a new notified field.
    pub fn new(value: T) -> Self {
        let (tx, _) = channel(1);

        Self { value, tx }
    }

    /// Set the value of the notified field and notify all listeners.
    pub fn set(&mut self, value: T) {
        self.value = value.clone();
        // Failure means that there are currently not receivers and that's ok.
        let _ = self.tx.send(value.into());
    }

    /// Get the value of the notified field.
    pub fn get(&self) -> T {
        self.value.clone()
    }

    /// Get a stream of replies for the notified field.
    pub fn stream(&self) -> Stream<ReplyParams> {
        Stream(self.tx.subscribe().into())
    }
}

/// The stream to use as the [`crate::Service::ReplyStream`] in service implementation when using
/// [`State`].
#[derive(Debug)]
pub struct Stream<ReplyParams>(BroadcastStream<ReplyParams>);

impl<ReplyParams> futures_util::Stream for Stream<ReplyParams>
where
    ReplyParams: Clone + Send + 'static,
{
    type Item = Reply<ReplyParams>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let reply = loop {
            match ready!(Pin::new(&mut self.0).poll_next(cx)) {
                Some(Ok(reply)) => {
                    break Some(Reply::new(Some(reply)).set_continues(Some(true)));
                }
                // Some intermediate values were missed. That's OK, as long as we get the latest
                // value.
                Some(Err(_)) => continue,
                None => break None,
            }
        };

        Poll::Ready(reply)
    }
}
